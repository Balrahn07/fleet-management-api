use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};

use crate::services::create_vehicle_service;
use crate::{
    models::{CreateVehicleRequest, Vehicle},
    state::AppState,
};

pub async fn health_check() -> &'static str {
    "OK"
}

pub async fn list_vehicles(State(state): State<AppState>) -> Json<Vec<Vehicle>> {
    let vehicles = state.vehicles.lock().unwrap();

    Json(vehicles.clone())
}

pub async fn get_vehicle(
    State(state): State<AppState>,
    Path(id): Path<u32>,
) -> Result<Json<Vehicle>, StatusCode> {
    let vehicles = state.vehicles.lock().unwrap();

    vehicles
        .iter()
        .find(|vehicle| vehicle.id == id)
        .cloned()
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

pub async fn create_vehicle(
    State(state): State<AppState>,
    Json(request): Json<CreateVehicleRequest>,
) -> Result<(StatusCode, Json<Vehicle>), StatusCode> {
    let vehicle = create_vehicle_service(&state, request)?;

    Ok((StatusCode::CREATED, Json(vehicle)))
}
