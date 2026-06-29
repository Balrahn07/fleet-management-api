use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use tower::ServiceExt;

use fleet_management_api::{routes::create_routes, state::AppState};

/// Builds the Axum app using the dedicated test database.
///
/// The test database comes from `.env.test`.
async fn test_app() -> axum::Router {
    dotenvy::from_filename(".env.test").ok();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let db = sqlx::PgPool::connect(&database_url)
        .await
        .expect("failed to connect to test database");

    sqlx::query("DELETE FROM vehicles")
        .execute(&db)
        .await
        .expect("failed to clean vehicles table");

    let state = AppState { db };

    create_routes(state)
}

#[tokio::test]
async fn health_check_returns_ok() {
    let app = test_app().await;

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();

    assert_eq!(&body[..], b"OK");
}

#[tokio::test]
async fn list_vehicles_returns_empty_list() {
    let app = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/vehicles")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();

    assert_eq!(&body[..], b"[]");
}
