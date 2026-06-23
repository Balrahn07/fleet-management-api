use axum::{Json, Router, extract::Path, http::StatusCode, routing::get};
use serde::Serialize;
use std::net::SocketAddr;

#[derive(Serialize)]
struct Vehicle {
    id: u32,
    vin: String,
    model: String,
    status: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/", get(health_check))
        .route("/vehicles/{id}", get(get_vehicle));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    tracing::info!("server running on http://{}", addr);

    axum::serve(tokio::net::TcpListener::bind(addr).await.unwrap(), app)
        .await
        .unwrap();
}

async fn health_check() -> &'static str {
    "OK"
}

async fn get_vehicle(Path(id): Path<u32>) -> Result<Json<Vehicle>, StatusCode> {
    let vehicles = sample_vehicles();

    vehicles
        .into_iter()
        .find(|v| v.id == id)
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

fn sample_vehicles() -> Vec<Vehicle> {
    vec![
        Vehicle {
            id: 1,
            vin: "VF123456789".to_string(),
            model: "Tesla Model Y".to_string(),
            status: "online".to_string(),
        },
        Vehicle {
            id: 2,
            vin: "VF987654321".to_string(),
            model: "Renault Megane E-Tech".to_string(),
            status: "offline".to_string(),
        },
    ]
}
