use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use crate::{
    models::{CreateVehicleRequest, Vehicle},
    state::AppState,
};

pub async fn health_check() -> &'static str {
    "OK"
}

pub async fn list_vehicles(
    State(state): State<AppState>,
) -> Json<Vec<Vehicle>> {
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
    if request.vin.trim().is_empty() || request.model.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let mut vehicles = state.vehicles.lock().unwrap();

    let new_id = vehicles.len() as u32 + 1;

    let vehicle = Vehicle {
        id: new_id,
        vin: request.vin,
        model: request.model,
        status: "offline".to_string(),
    };

    vehicles.push(vehicle.clone());

    Ok((StatusCode::CREATED, Json(vehicle)))
}