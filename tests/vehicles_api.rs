use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use tower::ServiceExt;

use fleet_management_api::{
    models::{PaginatedResponse, Vehicle},
    routes::create_routes,
    state::AppState,
};

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
    let vehicles: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(vehicles["data"], serde_json::json!([]));
    assert_eq!(vehicles["pagination"]["page"], 1);
    assert_eq!(vehicles["pagination"]["limit"], 10);
    assert_eq!(vehicles["pagination"]["total_items"], 0);
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

    assert_eq!(vehicles["data"].as_array().unwrap().len(), 1);
    assert_eq!(vehicles["data"][0]["vin"], "5YJ3E1EA7KF317125");
    assert_eq!(vehicles["data"][0]["model"], "Tesla Model 3");
    assert_eq!(vehicles["data"][0]["status"], "offline");
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

#[tokio::test]
#[serial]
async fn get_vehicle_returns_404_when_missing() {
    let app = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/vehicles/550e8400-e29b-41d4-a716-446655440000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let body = response.into_body().collect().await.unwrap().to_bytes();

    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(body["error"], "Vehicle not found");
}

#[tokio::test]
#[serial]
async fn delete_vehicle_deletes_existing_vehicle() {
    let app = test_app().await;

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/vehicles")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{"vin":"5YJ3E1EA7KF317127","model":"Tesla Model 3"}"#,
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

    let delete_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/vehicles/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(delete_response.status(), StatusCode::NO_CONTENT);

    let get_response = app
        .oneshot(
            Request::builder()
                .uri(format!("/vehicles/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(get_response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
#[serial]
async fn delete_vehicle_returns_404_when_missing() {
    let app = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/vehicles/550e8400-e29b-41d4-a716-446655440000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
#[serial]
async fn list_vehicles_supports_pagination() {
    let app = test_app().await;

    for i in 0..3 {
        let request_body = format!(
            r#"{{"vin":"5YJ3E1EA7KF31712{}","model":"Tesla Model 3"}}"#,
            i
        );

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
    }

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/vehicles?page=1&limit=2")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let vehicles: PaginatedResponse<Vehicle> = serde_json::from_slice(&body).unwrap();

    assert_eq!(vehicles.data.len(), 2);
}

#[tokio::test]
#[serial]
async fn list_vehicles_rejects_invalid_page() {
    let app = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/vehicles?page=0&limit=10")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(body["error"], "Invalid pagination parameters");
}

#[tokio::test]
#[serial]
async fn list_vehicles_rejects_invalid_limit() {
    let app = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/vehicles?page=1&limit=500")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
#[serial]
async fn list_vehicles_filters_by_status() {
    let app = test_app().await;

    let vehicles = [
        (
            r#"{"vin":"5YJ3E1EA7KF317124","model":"Tesla Model 3"}"#,
            "online",
        ),
        (
            r#"{"vin":"VF1AAAAAA12345678","model":"Renault Megane"}"#,
            "offline",
        ),
        (
            r#"{"vin":"WVWZZZ1JZXW000001","model":"Volkswagen Golf"}"#,
            "online",
        ),
    ];

    for (request_body, status) in vehicles {
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

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let vehicle: Vehicle = serde_json::from_slice(&body).unwrap();

        let update_body = format!(r#"{{"status":"{status}"}}"#);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/vehicles/{}", vehicle.id))
                    .header("Content-Type", "application/json")
                    .body(Body::from(update_body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/vehicles?status=online")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: PaginatedResponse<Vehicle> = serde_json::from_slice(&body).unwrap();

    assert_eq!(body.data.len(), 2);
    assert_eq!(body.pagination.total_items, 2);

    assert!(body.data.iter().all(|vehicle| vehicle.status == "online"));
}

#[tokio::test]
#[serial]
async fn list_vehicles_rejects_invalid_status_filter() {
    let app = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/vehicles?status=flying")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(body["error"], "Invalid status");
}

#[tokio::test]
#[serial]
async fn list_vehicles_sorts_by_model_ascending() {
    let app = test_app().await;

    let vehicles = [
        r#"{"vin":"5YJ3E1EA7KF317124","model":"Tesla Model 3"}"#,
        r#"{"vin":"VF1AAAAAA12345678","model":"Renault Megane"}"#,
        r#"{"vin":"WVWZZZ1JZXW000001","model":"Audi A3"}"#,
    ];

    for request_body in vehicles {
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
    }

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/vehicles?sort_by=model&order=asc")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();

    let response: PaginatedResponse<Vehicle> = serde_json::from_slice(&body).unwrap();

    let models: Vec<&str> = response
        .data
        .iter()
        .map(|vehicle| vehicle.model.as_str())
        .collect();

    assert_eq!(models, vec!["Audi A3", "Renault Megane", "Tesla Model 3"]);
}

#[tokio::test]
#[serial]
async fn list_vehicles_rejects_invalid_sort_field() {
    let app = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/vehicles?sort_by=password")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(body["error"], "Invalid sort field");
}

#[tokio::test]
#[serial]
async fn list_vehicles_rejects_invalid_sort_order() {
    let app = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/vehicles?order=random")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(body["error"], "Invalid sort order");
}
