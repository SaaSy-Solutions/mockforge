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

    #[error("Insufficient scope: required '{required}', token has [{scopes:?}]")]
    InsufficientScope {
        required: String,
        scopes: Vec<String>,
    },

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
            ApiError::InsufficientScope { required, scopes } => (
                StatusCode::FORBIDDEN,
                "INSUFFICIENT_SCOPE",
                format!(
                    "Insufficient scope: required '{}', token has [{}]",
                    required,
                    scopes.join(", ")
                ),
                json!({
                    "required_scope": required,
                    "token_scopes": scopes,
                    "hint": "Your API token does not have the required scope for this operation. Create a new token with the appropriate scope."
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::response::IntoResponse;

    // ApiError variant tests
    #[test]
    fn test_api_error_plugin_not_found() {
        let error = ApiError::PluginNotFound("test-plugin".to_string());
        assert_eq!(error.to_string(), "Plugin not found: test-plugin");
    }

    #[test]
    fn test_api_error_template_not_found() {
        let error = ApiError::TemplateNotFound("test-template".to_string());
        assert_eq!(error.to_string(), "Template not found: test-template");
    }

    #[test]
    fn test_api_error_scenario_not_found() {
        let error = ApiError::ScenarioNotFound("test-scenario".to_string());
        assert_eq!(error.to_string(), "Scenario not found: test-scenario");
    }

    #[test]
    fn test_api_error_invalid_version() {
        let error = ApiError::InvalidVersion("1.0.0".to_string());
        assert_eq!(error.to_string(), "Version not found: 1.0.0");
    }

    #[test]
    fn test_api_error_plugin_exists() {
        let error = ApiError::PluginExists("test-plugin".to_string());
        assert_eq!(error.to_string(), "Plugin already exists: test-plugin");
    }

    #[test]
    fn test_api_error_template_exists() {
        let error = ApiError::TemplateExists("test-template".to_string());
        assert_eq!(error.to_string(), "Template already exists: test-template");
    }

    #[test]
    fn test_api_error_scenario_exists() {
        let error = ApiError::ScenarioExists("test-scenario".to_string());
        assert_eq!(error.to_string(), "Scenario already exists: test-scenario");
    }

    #[test]
    fn test_api_error_auth_required() {
        let error = ApiError::AuthRequired;
        assert_eq!(error.to_string(), "Authentication required");
    }

    #[test]
    fn test_api_error_permission_denied() {
        let error = ApiError::PermissionDenied;
        assert_eq!(error.to_string(), "Permission denied");
    }

    #[test]
    fn test_api_error_organization_not_found() {
        let error = ApiError::OrganizationNotFound;
        assert_eq!(error.to_string(), "Organization not found or access denied");
    }

    #[test]
    fn test_api_error_invalid_request() {
        let error = ApiError::InvalidRequest("Bad input".to_string());
        assert_eq!(error.to_string(), "Invalid request: Bad input");
    }

    #[test]
    fn test_api_error_validation_failed() {
        let error = ApiError::ValidationFailed("Name is required".to_string());
        assert_eq!(error.to_string(), "Validation failed: Name is required");
    }

    #[test]
    fn test_api_error_rate_limit_exceeded() {
        let error = ApiError::RateLimitExceeded("100 requests/minute".to_string());
        assert_eq!(error.to_string(), "Rate limit exceeded: 100 requests/minute");
    }

    #[test]
    fn test_api_error_resource_limit_exceeded() {
        let error = ApiError::ResourceLimitExceeded("10 plugins".to_string());
        assert_eq!(error.to_string(), "Resource limit exceeded: 10 plugins");
    }

    #[test]
    fn test_api_error_storage() {
        let error = ApiError::Storage("S3 connection failed".to_string());
        assert_eq!(error.to_string(), "Storage error: S3 connection failed");
    }

    #[test]
    fn test_api_error_internal() {
        let error = ApiError::Internal(anyhow::anyhow!("Unknown error"));
        assert_eq!(error.to_string(), "Internal server error");
    }

    // IntoResponse tests - check status codes
    #[tokio::test]
    async fn test_into_response_plugin_not_found() {
        let error = ApiError::PluginNotFound("test".to_string());
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_into_response_template_not_found() {
        let error = ApiError::TemplateNotFound("test".to_string());
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_into_response_scenario_not_found() {
        let error = ApiError::ScenarioNotFound("test".to_string());
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_into_response_invalid_version() {
        let error = ApiError::InvalidVersion("1.0.0".to_string());
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_into_response_plugin_exists() {
        let error = ApiError::PluginExists("test".to_string());
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn test_into_response_template_exists() {
        let error = ApiError::TemplateExists("test".to_string());
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn test_into_response_scenario_exists() {
        let error = ApiError::ScenarioExists("test".to_string());
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn test_into_response_auth_required() {
        let error = ApiError::AuthRequired;
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_into_response_permission_denied() {
        let error = ApiError::PermissionDenied;
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_into_response_organization_not_found() {
        let error = ApiError::OrganizationNotFound;
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_into_response_invalid_request() {
        let error = ApiError::InvalidRequest("test".to_string());
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_into_response_validation_failed() {
        let error = ApiError::ValidationFailed("test".to_string());
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_into_response_rate_limit_exceeded() {
        let error = ApiError::RateLimitExceeded("test".to_string());
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[tokio::test]
    async fn test_into_response_resource_limit_exceeded() {
        let error = ApiError::ResourceLimitExceeded("test".to_string());
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_into_response_storage() {
        let error = ApiError::Storage("test".to_string());
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn test_into_response_internal() {
        let error = ApiError::Internal(anyhow::anyhow!("test"));
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    // Debug trait tests
    #[test]
    fn test_api_error_debug() {
        let error = ApiError::AuthRequired;
        let debug = format!("{:?}", error);
        assert!(debug.contains("AuthRequired"));
    }

    #[test]
    fn test_api_error_insufficient_scope() {
        let error = ApiError::InsufficientScope {
            required: "publish:packages".to_string(),
            scopes: vec!["read:packages".to_string()],
        };
        // Uses {:?} for scopes vector, so it includes quotes
        assert!(error.to_string().contains("publish:packages"));
        assert!(error.to_string().contains("read:packages"));
    }

    #[tokio::test]
    async fn test_into_response_insufficient_scope() {
        let error = ApiError::InsufficientScope {
            required: "publish:packages".to_string(),
            scopes: vec!["read:packages".to_string()],
        };
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
}
