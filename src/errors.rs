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
    InvalidPagination,
    DuplicateVin,
    Database,
    InvalidSortField,
    InvalidSortOrder,
    DriverNotFound,
    VehicleAlreadyAssigned,
    DriverAlreadyAssigned,
}

impl AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            AppError::EmptyVin => StatusCode::BAD_REQUEST,
            AppError::EmptyModel => StatusCode::BAD_REQUEST,
            AppError::InvalidVinLength => StatusCode::BAD_REQUEST,
            AppError::InvalidStatus => StatusCode::BAD_REQUEST,
            AppError::InvalidPagination => StatusCode::BAD_REQUEST,
            AppError::VehicleNotFound => StatusCode::NOT_FOUND,
            AppError::DuplicateVin => StatusCode::CONFLICT,
            AppError::Database => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::InvalidSortField => StatusCode::BAD_REQUEST,
            AppError::InvalidSortOrder => StatusCode::BAD_REQUEST,
            AppError::DriverNotFound => StatusCode::NOT_FOUND,
            AppError::VehicleAlreadyAssigned => StatusCode::CONFLICT,
            AppError::DriverAlreadyAssigned => StatusCode::CONFLICT,
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
            AppError::InvalidPagination => "Invalid pagination parameters".to_string(),
            AppError::InvalidSortField => "Invalid sort field".to_string(),
            AppError::InvalidSortOrder => "Invalid sort order".to_string(),
            AppError::DriverNotFound => "Driver not found".to_string(),
            AppError::VehicleAlreadyAssigned => {
                "Vehicle is already assigned to a driver".to_string()
            }
            AppError::DriverAlreadyAssigned => {
                "Driver is already assigned to another vehicle".to_string()
            }
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
