use axum::http::StatusCode;

use crate::models::{CreateVehicleRequest, Vehicle};
use crate::state::AppState;

pub fn create_vehicle_service(
    state: &AppState,
    request: CreateVehicleRequest,
) -> Result<Vehicle, StatusCode> {
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

    Ok(vehicle)
}

pub fn list_vehicles_service(state: &AppState) -> Vec<Vehicle> {
    let vehicles = state.vehicles.lock().unwrap();

    vehicles.clone()
}

pub fn get_vehicle_service(state: &AppState, id: u32) -> Result<Vehicle, StatusCode> {
    let vehicles = state.vehicles.lock().unwrap();

    vehicles
        .iter()
        .find(|vehicle| vehicle.id == id)
        .cloned()
        .ok_or(StatusCode::NOT_FOUND)
}
