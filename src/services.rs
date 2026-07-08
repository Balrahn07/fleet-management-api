use crate::{errors::AppError, models::UpdateVehicleRequest};
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
) -> Result<Vec<Vehicle>, AppError> {
    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(10);

    if page < 1 {
        return Err(AppError::InvalidPagination);
    }

    if limit < 1 || limit > 100 {
        return Err(AppError::InvalidPagination);
    }

    let offset = (page - 1) * limit;

    repositories::list_vehicles(&state.db, limit, offset)
        .await
        .map_err(|_| AppError::Database)
}

pub async fn get_vehicle_service(state: &AppState, id: Uuid) -> Result<Vehicle, AppError> {
    repositories::get_vehicle(&state.db, id)
        .await
        .map_err(|_| AppError::Database)?
        .ok_or(AppError::VehicleNotFound)
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
    info!("Validating create vehicle request.");

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
    info!("Validating update vehicle request.");

    if request.status.trim().is_empty() {
        return Err(AppError::InvalidStatus);
    }
    repositories::update_vehicle(&state.db, id, request.status)
        .await
        .map_err(|error| match error {
            sqlx::Error::RowNotFound => AppError::VehicleNotFound,
            _ => AppError::Database,
        })
}

pub async fn delete_vehicle_service(state: &AppState, id: Uuid) -> Result<(), AppError> {
    match repositories::delete_vehicle(&state.db, id).await {
        Ok(true) => Ok(()),
        Ok(false) => Err(AppError::VehicleNotFound),
        Err(_) => Err(AppError::Database),
    }
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
