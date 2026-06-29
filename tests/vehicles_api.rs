use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use tower::ServiceExt;

use fleet_management_api::{routes::create_routes, state::AppState};

use serial_test::serial;

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
#[serial]
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
#[serial]
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

#[tokio::test]
#[serial]
async fn create_vehicle_returns_created_vehicle() {
    let app = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/vehicles")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{"vin":"5YJ3E1EA7KF317123","model":"Tesla Model 3"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(body["vin"], "5YJ3E1EA7KF317123");
    assert_eq!(body["model"], "Tesla Model 3");
    assert_eq!(body["status"], "offline");

    assert!(body["id"].is_string());
    assert!(body["created_at"].is_string());
    assert!(body["updated_at"].is_string());
}

#[tokio::test]
#[serial]
async fn create_vehicle_rejects_duplicate_vin() {
    let app = test_app().await;

    let request_body = r#"{"vin":"5YJ3E1EA7KF317124","model":"Tesla Model 3"}"#;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/vehicles")
                .header("Content-Type", "application/json")
                .body(Body::from(request_body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/vehicles")
                .header("Content-Type", "application/json")
                .body(Body::from(request_body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CONFLICT);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(body["error"], "A vehicle with this VIN already exists");
}

#[tokio::test]
#[serial]
async fn list_vehicles_returns_created_vehicle() {
    let app = test_app().await;

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/vehicles")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{"vin":"5YJ3E1EA7KF317125","model":"Tesla Model 3"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(create_response.status(), StatusCode::CREATED);

    let list_response = app
        .oneshot(
            Request::builder()
                .uri("/vehicles")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(list_response.status(), StatusCode::OK);

    let body = list_response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();

    let vehicles: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(vehicles.as_array().unwrap().len(), 1);
    assert_eq!(vehicles[0]["vin"], "5YJ3E1EA7KF317125");
    assert_eq!(vehicles[0]["model"], "Tesla Model 3");
    assert_eq!(vehicles[0]["status"], "offline");
}

#[tokio::test]
#[serial]
async fn get_vehicle_returns_created_vehicle() {
    let app = test_app().await;

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/vehicles")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{"vin":"5YJ3E1EA7KF317126","model":"Tesla Model 3"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(create_response.status(), StatusCode::CREATED);

    let body = create_response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();

    let created_vehicle: serde_json::Value = serde_json::from_slice(&body).unwrap();

    let id = created_vehicle["id"].as_str().unwrap();

    let get_response = app
        .oneshot(
            Request::builder()
                .uri(format!("/vehicles/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(get_response.status(), StatusCode::OK);

    let body = get_response.into_body().collect().await.unwrap().to_bytes();

    let vehicle: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(vehicle["id"], id);
    assert_eq!(vehicle["vin"], "5YJ3E1EA7KF317126");
    assert_eq!(vehicle["model"], "Tesla Model 3");
    assert_eq!(vehicle["status"], "offline");
}
