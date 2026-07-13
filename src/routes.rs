use crate::{
    handlers::{
        assign_driver, create_vehicle, delete_vehicle, get_vehicle, health_check, list_vehicles,
        update_vehicle,
    },
    state::AppState,
};
use axum::{
    Router,
    http::{HeaderName, Request},
    routing::{delete, get, post, put},
};
use std::time::Duration;
use tower_http::{
    classify::ServerErrorsFailureClass,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};

use tracing::{Span, info_span};

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
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &Request<_>| {
                    let request_id = request
                        .headers()
                        .get("x-request-id")
                        .and_then(|value| value.to_str().ok())
                        .unwrap_or("unknown");

                    info_span!(
                        "http_request",
                        request_id = %request_id,
                        method = %request.method(),
                        uri = %request.uri(),
                        version = ?request.version(),
                    )
                })
                .on_response(
                    |response: &axum::response::Response, latency: Duration, _span: &Span| {
                        tracing::info!(
                            status = %response.status(),
                            latency_ms = latency.as_millis(),
                            "Request completed"
                        );
                    },
                )
                .on_failure(
                    |failure: ServerErrorsFailureClass, latency: Duration, _span: &Span| {
                        tracing::error!(
                            failure = %failure,
                            latency_ms = latency.as_millis(),
                            "Request failed"
                        );
                    },
                ),
        )
        .layer(PropagateRequestIdLayer::new(request_id_header.clone()))
        .layer(SetRequestIdLayer::new(request_id_header, MakeRequestUuid))
}
