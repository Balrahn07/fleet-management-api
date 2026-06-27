use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};

use crate::services::{create_vehicle_service, get_vehicle_service, list_vehicles_service};
use crate::{
    models::{CreateVehicleRequest, Vehicle},
    state::AppState,
};

pub async fn health_check() -> &'static str {
    "OK"
}

pub async fn list_vehicles(State(state): State<AppState>) -> Json<Vec<Vehicle>> {
    Json(list_vehicles_service(&state))
}

pub async fn get_vehicle(
    State(state): State<AppState>,
    Path(id): Path<u32>,
) -> Result<Json<Vehicle>, StatusCode> {
    let vehicle = get_vehicle_service(&state, id)?;

    Ok(Json(vehicle))
}

pub async fn create_vehicle(
    State(state): State<AppState>,
    Json(request): Json<CreateVehicleRequest>,
) -> Result<(StatusCode, Json<Vehicle>), StatusCode> {
    let vehicle = create_vehicle_service(&state, request)?;

    Ok((StatusCode::CREATED, Json(vehicle)))
}
