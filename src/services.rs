use axum::http::StatusCode;
use uuid::Uuid;

use crate::{
    models::{CreateVehicleRequest, Vehicle},
    repositories,
    state::AppState,
};

pub async fn list_vehicles_service(state: &AppState) -> Result<Vec<Vehicle>, StatusCode> {
    repositories::list_vehicles(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub async fn get_vehicle_service(state: &AppState, id: Uuid) -> Result<Vehicle, StatusCode> {
    repositories::get_vehicle(&state.db, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)
}

pub async fn create_vehicle_service(
    state: &AppState,
    request: CreateVehicleRequest,
) -> Result<Vehicle, StatusCode> {
    if request.vin.trim().is_empty() || request.model.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let result = repositories::create_vehicle(
        &state.db,
        Uuid::new_v4(),
        request.vin,
        request.model,
        "offline".to_string(),
    )
    .await;

    let vehicle = result.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(vehicle)
}
