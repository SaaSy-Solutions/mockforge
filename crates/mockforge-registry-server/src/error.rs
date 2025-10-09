//! Error types for the registry server

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Plugin not found: {0}")]
    PluginNotFound(String),

    #[error("Invalid version: {0}")]
    InvalidVersion(String),

    #[error("Plugin already exists: {0}")]
    PluginExists(String),

    #[error("Authentication required")]
    AuthRequired,

    #[error("Permission denied")]
    PermissionDenied,

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Internal server error")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            ApiError::PluginNotFound(name) => {
                (StatusCode::NOT_FOUND, format!("Plugin '{}' not found", name))
            }
            ApiError::InvalidVersion(ver) => {
                (StatusCode::BAD_REQUEST, format!("Invalid version: {}", ver))
            }
            ApiError::PluginExists(name) => (
                StatusCode::CONFLICT,
                format!("Plugin '{}' already exists", name),
            ),
            ApiError::AuthRequired => (StatusCode::UNAUTHORIZED, "Authentication required".to_string()),
            ApiError::PermissionDenied => (StatusCode::FORBIDDEN, "Permission denied".to_string()),
            ApiError::InvalidRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::Database(e) => {
                tracing::error!("Database error: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string())
            }
            ApiError::Storage(msg) => {
                tracing::error!("Storage error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Storage error: {}", msg))
            }
            ApiError::Internal(e) => {
                tracing::error!("Internal error: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string())
            }
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
