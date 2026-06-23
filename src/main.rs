use axum::{
    routing::get,
    Json, Router,
};
use serde::Serialize;
use std::net::SocketAddr;
use uuid::Uuid;

#[derive(Serialize)]
struct Vehicle {
    id: Uuid,
    vin: String,
    model: String,
    status: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/", get(health_check))
        .route("/vehicles", get(list_vehicles));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    tracing::info!("server running on http://{}", addr);

    axum::serve(
        tokio::net::TcpListener::bind(addr).await.unwrap(),
        app,
    )
    .await
    .unwrap();
}

async fn health_check() -> &'static str {
    "OK"
}

async fn list_vehicles() -> Json<Vec<Vehicle>> {
    let vehicles = vec![
        Vehicle {
            id: Uuid::new_v4(),
            vin: "VF123456789".to_string(),
            model: "Tesla Model Y".to_string(),
            status: "online".to_string(),
        },
        Vehicle {
            id: Uuid::new_v4(),
            vin: "VF987654321".to_string(),
            model: "Renault Megane E-Tech".to_string(),
            status: "offline".to_string(),
        },
    ];

    Json(vehicles)
}