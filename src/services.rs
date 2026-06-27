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
