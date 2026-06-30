use crate::{
    handlers::{create_vehicle, get_vehicle, health_check, list_vehicles, update_vehicle},
    state::AppState,
};
use axum::{
    Router,
    routing::{get, post, put},
};

pub fn create_routes(state: AppState) -> Router {
    Router::new()
        .route("/", get(health_check))
        .route("/vehicles", get(list_vehicles))
        .route("/vehicles", post(create_vehicle))
        .route("/vehicles/{id}", get(get_vehicle))
        .route("/vehicles/{id}", put(update_vehicle))
        .with_state(state)
}
