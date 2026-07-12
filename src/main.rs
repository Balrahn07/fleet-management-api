use fleet_management_api::config::DatabaseConfig;
use std::net::SocketAddr;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::Level;

use dotenvy::dotenv;

use fleet_management_api::{routes::create_routes, state::AppState};

#[tokio::main]
async fn main() {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    let database_config = DatabaseConfig::from_env();

    let db = database_config
        .create_pool()
        .await
        .expect("Failed to create PostgreSQL connection pool");

    let state = AppState { db };

    let app = create_routes(state).layer(
        TraceLayer::new_for_http()
            .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
            .on_response(DefaultOnResponse::new().level(Level::INFO)),
    );

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    tracing::info!("server running on http://{}", addr);

    axum::serve(tokio::net::TcpListener::bind(addr).await.unwrap(), app)
        .await
        .unwrap();
}
