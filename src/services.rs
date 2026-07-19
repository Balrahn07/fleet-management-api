use crate::{
    errors::AppError,
    models::{
        PaginatedResponse, Pagination, SortOrder, UpdateVehicleRequest, VehicleFilter,
        VehicleSortField,
    },
};
use uuid::Uuid;

use crate::{
    models::{CreateVehicleRequest, ListVehiclesQuery, Vehicle},
    repositories,
    state::AppState,
};
use tracing::{info, warn};

pub async fn list_vehicles_service(
    state: &AppState,
    query: ListVehiclesQuery,
) -> Result<PaginatedResponse<Vehicle>, AppError> {
    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(10);

    if page < 1 || !(1..=100).contains(&limit) {
        return Err(AppError::InvalidPagination);
    }

    if let Some(status) = &query.status {
        validate_status(status)?;
    }

    let sort_field = match query.sort_by.as_deref().unwrap_or("created_at") {
        "created_at" => VehicleSortField::CreatedAt,
        "model" => VehicleSortField::Model,
        "status" => VehicleSortField::Status,
        _ => return Err(AppError::InvalidSortField),
    };

    let sort_order = match query.order.as_deref().unwrap_or("desc") {
        "asc" => SortOrder::Asc,
        "desc" => SortOrder::Desc,
        _ => return Err(AppError::InvalidSortOrder),
    };

    let filter = VehicleFilter {
        status: query.status,
        sort_field,
        sort_order,
    };

    let offset = (page - 1) * limit;

    let vehicles = repositories::list_vehicles(&state.db, limit, offset, &filter)
        .await
        .map_err(|error| {
            tracing::error!(
                error = ?error,
                "Database operation failed"
            );

            AppError::Database
        })?;
    let total = repositories::count_vehicles(&state.db, &filter)
        .await
        .map_err(|error| {
            tracing::error!(
                error = ?error,
                "Database operation failed"
            );

            AppError::Database
        })?;

    let total_pages = (total + limit - 1) / limit;

    let response = PaginatedResponse {
        data: vehicles,
        pagination: Pagination {
            page,
            limit,
            total_items: total,
            total_pages,
            has_next: page < total_pages,
            has_previous: page > 1,
        },
    };

    Ok(response)
}

pub async fn get_vehicle_service(state: &AppState, id: Uuid) -> Result<Vehicle, AppError> {
    let cache_key = format!("vehicle:{id}");

    if let Some(cached_vehicle) = state.cache.get(&cache_key).await {
        tracing::info!(vehicle_id = %id, "Cache hit");

        let vehicle = serde_json::from_str::<Vehicle>(&cached_vehicle).map_err(|error| {
            tracing::error!(
                vehicle_id = %id,
                error = %error,
                "Failed to deserialize cached vehicle"
            );

            AppError::Cache
        })?;

        return Ok(vehicle);
    }

    tracing::info!(vehicle_id = %id, "Cache miss");

    let vehicle = repositories::get_vehicle(&state.db, id)
        .await
        .map_err(|error| {
            tracing::error!(
                vehicle_id = %id,
                error = %error,
                "Failed to retrieve vehicle"
            );

            AppError::Database
        })?
        .ok_or(AppError::VehicleNotFound)?;

    let serialized_vehicle = serde_json::to_string(&vehicle).map_err(|error| {
        tracing::error!(
            vehicle_id = %id,
            error = %error,
            "Failed to serialize vehicle"
        );

        AppError::Cache
    })?;

    state.cache.set(&cache_key, serialized_vehicle).await;

    Ok(vehicle)
}

/// Creates a new vehicle.
///
/// Business rules:
/// - VIN must not be empty.
/// - VIN must be exactly 17 characters.
/// - Model must not be empty.
/// - New vehicles are created with the "offline" status.
pub async fn create_vehicle_service(
    state: &AppState,
    request: CreateVehicleRequest,
) -> Result<Vehicle, AppError> {
    info!(
        vin = %request.vin,
        model = %request.model,
        "Validating create vehicle request"
    );

    validate_create_vehicle_request(&request)?;

    let result = repositories::create_vehicle(
        &state.db,
        Uuid::new_v4(),
        request.vin,
        request.model,
        "offline".to_string(),
    )
    .await;

    let vehicle = result.map_err(map_create_vehicle_error)?;
    Ok(vehicle)
}

pub async fn update_vehicle_service(
    state: &AppState,
    id: Uuid,
    request: UpdateVehicleRequest,
) -> Result<Vehicle, AppError> {
    info!(
        vehicle_id = %id,
        status = %request.status,
        "Updating vehicle status"
    );

    validate_status(&request.status)?;

    let vehicle = repositories::update_vehicle(&state.db, id, request.status)
        .await
        .map_err(|error| match error {
            sqlx::Error::RowNotFound => AppError::VehicleNotFound,
            other => {
                tracing::error!(
                    vehicle_id = %id,
                    error = %other,
                    "Failed to update vehicle"
                );

                AppError::Database
            }
        })?;

    let cache_key = format!("vehicle:{id}");
    state.cache.remove(&cache_key).await;

    Ok(vehicle)
}

pub async fn delete_vehicle_service(state: &AppState, id: Uuid) -> Result<(), AppError> {
    match repositories::delete_vehicle(&state.db, id).await {
        Ok(true) => {
            let cache_key = format!("vehicle:{id}");
            state.cache.remove(&cache_key).await;

            Ok(())
        }
        Ok(false) => Err(AppError::VehicleNotFound),
        Err(error) => {
            tracing::error!(
                vehicle_id = %id,
                error = %error,
                "Failed to delete vehicle"
            );

            Err(AppError::Database)
        }
    }
}

pub async fn assign_driver_service(
    state: &AppState,
    vehicle_id: Uuid,
    driver_id: Uuid,
) -> Result<Vehicle, AppError> {
    let mut tx = state.db.begin().await.map_err(|_| AppError::Database)?;

    let current_driver = repositories::find_vehicle_driver_for_update(&mut tx, vehicle_id)
        .await
        .map_err(|_| AppError::Database)?;

    let current_driver = current_driver.ok_or(AppError::VehicleNotFound)?;

    if current_driver.is_some() {
        return Err(AppError::VehicleAlreadyAssigned);
    }

    let driver_exists = repositories::driver_exists(&mut tx, driver_id)
        .await
        .map_err(|_| AppError::Database)?;

    if !driver_exists {
        return Err(AppError::DriverNotFound);
    }

    let driver_is_assigned = repositories::is_driver_assigned(&mut tx, driver_id)
        .await
        .map_err(|_| AppError::Database)?;

    if driver_is_assigned {
        return Err(AppError::DriverAlreadyAssigned);
    }

    let vehicle = repositories::assign_driver_to_vehicle(&mut tx, vehicle_id, driver_id)
        .await
        .map_err(|_| AppError::Database)?;

    tx.commit().await.map_err(|_| AppError::Database)?;

    Ok(vehicle)
}

/// Maps low-level SQLx errors into business-level application errors.
///
/// For example:
/// - UNIQUE constraint violation on VIN -> `AppError::DuplicateVin`
/// - Any other database error -> `AppError::Database`
fn map_create_vehicle_error(error: sqlx::Error) -> AppError {
    match error {
        sqlx::Error::Database(db_error) => {
            if db_error.constraint() == Some("vehicles_vin_key") {
                AppError::DuplicateVin
            } else {
                AppError::Database
            }
        }
        _ => AppError::Database,
    }
}

/// Validates the input required to create a vehicle.
fn validate_create_vehicle_request(request: &CreateVehicleRequest) -> Result<(), AppError> {
    if request.vin.trim().is_empty() {
        warn!("Rejected vehicle creation: VIN is empty.");
        return Err(AppError::EmptyVin);
    }

    if request.model.trim().is_empty() {
        warn!("Rejected vehicle creation: model is empty.");
        return Err(AppError::EmptyModel);
    }

    if request.vin.len() != 17 {
        warn!("Rejected vehicle creation: VIN has invalid length.");
        return Err(AppError::InvalidVinLength);
    }

    Ok(())
}

fn validate_status(status: &str) -> Result<(), AppError> {
    let valid_statuses = ["online", "offline", "maintenance"];
    if !valid_statuses.contains(&status) {
        warn!("Rejected vehicle update: invalid status '{}'.", status);
        return Err(AppError::InvalidStatus);
    }
    Ok(())
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::models::CreateVehicleRequest;

    // ---------- Validation Tests ----------
    #[test]
    fn validate_request_rejects_empty_vin() {
        let request = CreateVehicleRequest {
            vin: "".to_string(),
            model: "Tesla Model 3".to_string(),
        };

        let result = validate_create_vehicle_request(&request);

        assert!(matches!(result, Err(AppError::EmptyVin)));
    }

    #[test]
    fn validate_request_rejects_empty_model() {
        let request = CreateVehicleRequest {
            vin: "5YJ3E1EA7KF317123".to_string(),
            model: "".to_string(),
        };
        let result = validate_create_vehicle_request(&request);

        assert!(matches!(result, Err(AppError::EmptyModel)));
    }

    #[test]
    fn validate_request_rejects_invalid_vin_length() {
        let request = CreateVehicleRequest {
            vin: "5YJ3E1EA7KF31712".to_string(),
            model: "Tesla Model 3".to_string(),
        };
        let result = validate_create_vehicle_request(&request);

        assert!(matches!(result, Err(AppError::InvalidVinLength)));
    }

    #[test]
    fn validate_request_accepts_valid_request() {
        let request = CreateVehicleRequest {
            vin: "5YJ3E1EA7KF317123".to_string(),
            model: "Tesla Model 3".to_string(),
        };
        let result = validate_create_vehicle_request(&request);
        assert!(result.is_ok());
    }
}
