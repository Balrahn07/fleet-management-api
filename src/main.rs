mod handlers;
mod models;
mod routes;
mod services;
mod state;

use std::net::SocketAddr;

use crate::{routes::create_routes, state::AppState};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let state = AppState::new();
    let app = create_routes(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    tracing::info!("server running on http://{}", addr);

    axum::serve(tokio::net::TcpListener::bind(addr).await.unwrap(), app)
        .await
        .unwrap();
}
