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
        .ok_or(AppError::NotFound)
}

pub async fn create_vehicle_service(
    state: &AppState,
    request: CreateVehicleRequest,
) -> Result<Vehicle, AppError> {
    if request.vin.trim().is_empty() || request.model.trim().is_empty() {
        return Err(AppError::InvalidInput);
    }

    let result = repositories::create_vehicle(
        &state.db,
        Uuid::new_v4(),
        request.vin,
        request.model,
        "offline".to_string(),
    )
    .await;

    let vehicle = result.map_err(|_| AppError::Database)?;
    Ok(vehicle)
}
