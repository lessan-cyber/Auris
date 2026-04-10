use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use thiserror::Error;
use tokio::task::JoinError;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("Validation error: {0}")]
    Validation(String),
    #[error("Not found")]
    NotFound,
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Internal server error: {0}")]
    Internal(String),
    #[error("Task join error: {0}")]
    JoinError(#[from] JoinError),
    #[error("Anyhow error: {0}")]
    AnyhowError(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match &self {
            AppError::Database(e) => {
                tracing::error!("Database error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
            AppError::Storage(e) => {
                tracing::error!("Storage error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Storage error".to_string(),
                )
            }
            AppError::Validation(e) => (StatusCode::BAD_REQUEST, e.clone()),
            AppError::NotFound => (StatusCode::NOT_FOUND, "Resource not found".to_string()),
            AppError::BadRequest(e) => (StatusCode::BAD_REQUEST, e.clone()),
            AppError::Internal(e) => {
                tracing::error!("Internal error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
            AppError::JoinError(e) => {
                tracing::error!("Task join error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
            AppError::AnyhowError(e) => {
                tracing::error!("Anyhow error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
        };

        let body = Json(json!({
            "error": error_message,
            "status": status.as_u16()
        }));

        (status, body).into_response()
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
