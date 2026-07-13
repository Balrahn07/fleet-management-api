use crate::{
    handlers::{
        assign_driver, create_vehicle, delete_vehicle, get_vehicle, health_check, list_vehicles,
        update_vehicle,
    },
    state::AppState,
};
use axum::{
    Router,
    http::HeaderName,
    routing::{delete, get, post, put},
};

use tower_http::{
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};

pub fn create_routes(state: AppState) -> Router {
    let request_id_header = HeaderName::from_static("x-request-id");

    Router::new()
        .route("/health", get(health_check))
        .route("/vehicles", get(list_vehicles))
        .route("/vehicles", post(create_vehicle))
        .route("/vehicles/{id}", get(get_vehicle))
        .route("/vehicles/{id}", put(update_vehicle))
        .route("/vehicles/{id}", delete(delete_vehicle))
        .route("/vehicles/{id}/assign-driver", post(assign_driver))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(PropagateRequestIdLayer::new(request_id_header.clone()))
        .layer(SetRequestIdLayer::new(request_id_header, MakeRequestUuid))
}
