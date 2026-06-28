use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use tower::ServiceExt;

use fleet_management_api::{routes::create_routes, state::AppState};

#[tokio::test]
async fn health_check_returns_ok() {
    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let db = sqlx::PgPool::connect(&database_url)
        .await
        .expect("failed to connect to database");

    let state = AppState { db };

    let app = create_routes(state);

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();

    assert_eq!(&body[..], b"OK");
}
