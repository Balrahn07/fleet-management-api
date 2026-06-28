use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

pub enum AppError {
    InvalidInput,
    NotFound,
    Conflict,
    Database,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match self {
            AppError::InvalidInput => StatusCode::BAD_REQUEST,
            AppError::NotFound => StatusCode::NOT_FOUND,
            AppError::Conflict => StatusCode::CONFLICT,
            AppError::Database => StatusCode::INTERNAL_SERVER_ERROR,
        };

        status.into_response()
    }
}
