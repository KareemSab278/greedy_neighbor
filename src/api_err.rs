use axum::{http::StatusCode, Json};
use crate::structs;

#[derive(Debug)]
pub enum ApiError {
    BadRequest(String),
    Internal(String),
}

impl ApiError {
    pub fn bad_request(message: String) -> Self {
        ApiError::BadRequest(message)
    }

    pub fn internal(message: String) -> Self {
        ApiError::Internal(message)
    }

    pub fn into_response(self) -> (StatusCode, Json<structs::ErrorResponse>) {
        match self {
            ApiError::BadRequest(message) => (
                StatusCode::BAD_REQUEST,
                Json(structs::ErrorResponse { error: message }),
            ),
            ApiError::Internal(message) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(structs::ErrorResponse { error: message }),
            ),
        }
    }
}
