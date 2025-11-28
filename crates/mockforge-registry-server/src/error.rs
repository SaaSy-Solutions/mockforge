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
    // Resource not found errors
    #[error("Plugin not found: {0}")]
    PluginNotFound(String),

    #[error("Template not found: {0}")]
    TemplateNotFound(String),

    #[error("Scenario not found: {0}")]
    ScenarioNotFound(String),

    #[error("Version not found: {0}")]
    InvalidVersion(String),

    // Resource conflict errors
    #[error("Plugin already exists: {0}")]
    PluginExists(String),

    #[error("Template already exists: {0}")]
    TemplateExists(String),

    #[error("Scenario already exists: {0}")]
    ScenarioExists(String),

    // Authentication and authorization errors
    #[error("Authentication required")]
    AuthRequired,

    #[error("Permission denied")]
    PermissionDenied,

    #[error("Organization not found or access denied")]
    OrganizationNotFound,

    // Validation errors
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    // Rate limiting
    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),

    // Resource limit errors
    #[error("Resource limit exceeded: {0}")]
    ResourceLimitExceeded(String),

    // Storage and database errors
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Storage error: {0}")]
    Storage(String),

    // Internal errors
    #[error("Internal server error")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_code, error_message, details) = match self {
            // Resource not found errors (404)
            ApiError::PluginNotFound(name) => (
                StatusCode::NOT_FOUND,
                "PLUGIN_NOT_FOUND",
                format!("Plugin '{}' not found", name),
                json!({
                    "resource": "plugin",
                    "name": name
                }),
            ),
            ApiError::TemplateNotFound(name) => (
                StatusCode::NOT_FOUND,
                "TEMPLATE_NOT_FOUND",
                format!("Template '{}' not found", name),
                json!({
                    "resource": "template",
                    "name": name
                }),
            ),
            ApiError::ScenarioNotFound(name) => (
                StatusCode::NOT_FOUND,
                "SCENARIO_NOT_FOUND",
                format!("Scenario '{}' not found", name),
                json!({
                    "resource": "scenario",
                    "name": name
                }),
            ),
            ApiError::InvalidVersion(ver) => (
                StatusCode::NOT_FOUND,
                "VERSION_NOT_FOUND",
                format!("Version '{}' not found", ver),
                json!({
                    "version": ver
                }),
            ),

            // Resource conflict errors (409)
            ApiError::PluginExists(name) => (
                StatusCode::CONFLICT,
                "PLUGIN_EXISTS",
                format!("Plugin '{}' already exists", name),
                json!({
                    "resource": "plugin",
                    "name": name
                }),
            ),
            ApiError::TemplateExists(name) => (
                StatusCode::CONFLICT,
                "TEMPLATE_EXISTS",
                format!("Template '{}' already exists", name),
                json!({
                    "resource": "template",
                    "name": name
                }),
            ),
            ApiError::ScenarioExists(name) => (
                StatusCode::CONFLICT,
                "SCENARIO_EXISTS",
                format!("Scenario '{}' already exists", name),
                json!({
                    "resource": "scenario",
                    "name": name
                }),
            ),

            // Authentication and authorization errors
            ApiError::AuthRequired => (
                StatusCode::UNAUTHORIZED,
                "AUTH_REQUIRED",
                "Authentication required".to_string(),
                json!({
                    "hint": "Include a valid Authorization header with your request"
                }),
            ),
            ApiError::PermissionDenied => (
                StatusCode::FORBIDDEN,
                "PERMISSION_DENIED",
                "Permission denied".to_string(),
                json!({
                    "hint": "You don't have permission to perform this action"
                }),
            ),
            ApiError::OrganizationNotFound => (
                StatusCode::NOT_FOUND,
                "ORGANIZATION_NOT_FOUND",
                "Organization not found or access denied".to_string(),
                json!({
                    "hint": "Check that the organization exists and you have access to it"
                }),
            ),

            // Validation errors (400)
            ApiError::InvalidRequest(msg) => (
                StatusCode::BAD_REQUEST,
                "INVALID_REQUEST",
                msg.clone(),
                json!({
                    "message": msg
                }),
            ),
            ApiError::ValidationFailed(msg) => (
                StatusCode::BAD_REQUEST,
                "VALIDATION_FAILED",
                format!("Validation failed: {}", msg),
                json!({
                    "message": msg
                }),
            ),

            // Rate limiting (429)
            ApiError::RateLimitExceeded(msg) => {
                tracing::warn!("Rate limit exceeded: {}", msg);
                (
                    StatusCode::TOO_MANY_REQUESTS,
                    "RATE_LIMIT_EXCEEDED",
                    format!("Rate limit exceeded: {}", msg),
                    json!({
                        "message": msg,
                        "hint": "Please wait before making more requests or upgrade your plan"
                    }),
                )
            }

            // Resource limits (403)
            ApiError::ResourceLimitExceeded(msg) => {
                tracing::warn!("Resource limit exceeded: {}", msg);
                (
                    StatusCode::FORBIDDEN,
                    "RESOURCE_LIMIT_EXCEEDED",
                    format!("Resource limit exceeded: {}", msg),
                    json!({
                        "message": msg,
                        "hint": "Upgrade your plan to increase limits"
                    }),
                )
            }

            // Storage and database errors (500)
            ApiError::Database(e) => {
                tracing::error!("Database error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "DATABASE_ERROR",
                    "Database error occurred".to_string(),
                    json!({
                        "hint": "Please try again later or contact support if the problem persists"
                    }),
                )
            }
            ApiError::Storage(msg) => {
                tracing::error!("Storage error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "STORAGE_ERROR",
                    format!("Storage error: {}", msg),
                    json!({
                        "message": msg,
                        "hint": "Please try again later or contact support if the problem persists"
                    }),
                )
            }

            // Internal errors (500)
            ApiError::Internal(e) => {
                tracing::error!("Internal error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    "Internal server error".to_string(),
                    json!({
                        "hint": "Please try again later or contact support if the problem persists"
                    }),
                )
            }
        };

        let body = Json(json!({
            "error": error_message,
            "error_code": error_code,
            "status": status.as_u16(),
            "details": details
        }));

        (status, body).into_response()
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
