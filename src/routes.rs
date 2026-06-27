use axum::{
    routing::{get, post},
    Router,
};
use crate::{
    handlers::{create_vehicle, get_vehicle, health_check, list_vehicles},
    state::AppState,
};

pub fn create_routes(state: AppState) -> Router {
    Router::new()
        .route("/", get(health_check))
        .route("/vehicles", get(list_vehicles))
        .route("/vehicles", post(create_vehicle))
        .route("/vehicles/{id}", get(get_vehicle))
        .with_state(state)
}