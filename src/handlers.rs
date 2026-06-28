use crate::errors::AppError;
use crate::{
    models::{VehicleRequest, Vehicle},
    services::{create_vehicle_service, get_vehicle_service, list_vehicles_service},
    state::AppState,
};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use tracing::info;
use uuid::Uuid;

pub async fn health_check() -> &'static str {
    "OK"
}

pub async fn list_vehicles(State(state): State<AppState>) -> Result<Json<Vec<Vehicle>>, AppError> {
    let vehicles = list_vehicles_service(&state).await?;

    Ok(Json(vehicles))
}

pub async fn get_vehicle(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vehicle>, AppError> {
    let vehicle = get_vehicle_service(&state, id).await?;

    Ok(Json(vehicle))
}

pub async fn create_vehicle(
    State(state): State<AppState>,
    Json(request): Json<VehicleRequest>,
) -> Result<(StatusCode, Json<Vehicle>), AppError> {
    info!(
        vin = %request.vin,
        model = %request.model,
        "Creating vehicle"
    );

    let vehicle = create_vehicle_service(&state, request).await?;

    Ok((StatusCode::CREATED, Json(vehicle)))
}
