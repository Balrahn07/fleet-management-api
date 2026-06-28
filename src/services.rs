use crate::errors::AppError;
use uuid::Uuid;

use crate::{
    models::{CreateVehicleRequest, Vehicle},
    repositories,
    state::AppState,
};

pub async fn list_vehicles_service(state: &AppState) -> Result<Vec<Vehicle>, AppError> {
    repositories::list_vehicles(&state.db)
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
    if request.vin.trim().is_empty() {
        return Err(AppError::EmptyVin);
    } else if request.model.trim().is_empty() {
        return Err(AppError::EmptyModel);
    } else if request.vin.len() != 17 {
        return Err(AppError::InvalidVinLength);
    }

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
