use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

pub enum AppError {
    EmptyVin,
    EmptyModel,
    InvalidVinLength,
    InvalidStatus,
    VehicleNotFound,
    DuplicateVin,
    Database,
}

impl AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            AppError::EmptyVin => StatusCode::BAD_REQUEST,
            AppError::EmptyModel => StatusCode::BAD_REQUEST,
            AppError::InvalidVinLength => StatusCode::BAD_REQUEST,
            AppError::InvalidStatus => StatusCode::BAD_REQUEST,
            AppError::VehicleNotFound => StatusCode::NOT_FOUND,
            AppError::DuplicateVin => StatusCode::CONFLICT,
            AppError::Database => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn message(&self) -> String {
        match self {
            AppError::EmptyVin => "VIN cannot be empty".to_string(),
            AppError::EmptyModel => "Model cannot be empty".to_string(),
            AppError::InvalidVinLength => "VIN must be 17 characters".to_string(),
            AppError::InvalidStatus => "Invalid status".to_string(),
            AppError::VehicleNotFound => "Vehicle not found".to_string(),
            AppError::DuplicateVin => "A vehicle with this VIN already exists".to_string(),
            AppError::Database => "Internal server error".to_string(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();

        let body = Json(ErrorResponse {
            error: self.message(),
        });

        (status, body).into_response()
    }
}
