//! Metrics instrumentation helpers for marketplace operations

use mockforge_observability::prometheus::MetricsRegistry;
use std::sync::Arc;
use std::time::Instant;

/// Helper to record marketplace operation metrics with timing
pub struct MarketplaceMetrics {
    registry: Arc<MetricsRegistry>,
    start_time: Instant,
    item_type: &'static str,
}

impl MarketplaceMetrics {
    /// Start timing a marketplace operation
    pub fn start(registry: Arc<MetricsRegistry>, item_type: &'static str) -> Self {
        Self {
            registry,
            start_time: Instant::now(),
            item_type,
        }
    }

    /// Record a successful search operation
    pub fn record_search_success(self) {
        let duration = self.start_time.elapsed().as_secs_f64();
        self.registry.record_marketplace_search(self.item_type, true, duration);
    }

    /// Record a failed search operation
    pub fn record_search_error(self, error_code: &str) {
        let duration = self.start_time.elapsed().as_secs_f64();
        self.registry.record_marketplace_search(self.item_type, false, duration);
        self.registry.record_marketplace_error(self.item_type, error_code);
    }

    /// Record a successful download/get operation
    pub fn record_download_success(self) {
        let duration = self.start_time.elapsed().as_secs_f64();
        self.registry.record_marketplace_download(self.item_type, true, duration);
    }

    /// Record a failed download/get operation
    pub fn record_download_error(self, error_code: &str) {
        let duration = self.start_time.elapsed().as_secs_f64();
        self.registry.record_marketplace_download(self.item_type, false, duration);
        self.registry.record_marketplace_error(self.item_type, error_code);
    }

    /// Record a successful publish operation
    pub fn record_publish_success(self) {
        let duration = self.start_time.elapsed().as_secs_f64();
        self.registry.record_marketplace_publish(self.item_type, true, duration);
    }

    /// Record a failed publish operation
    pub fn record_publish_error(self, error_code: &str) {
        let duration = self.start_time.elapsed().as_secs_f64();
        self.registry.record_marketplace_publish(self.item_type, false, duration);
        self.registry.record_marketplace_error(self.item_type, error_code);
    }
}

/// Extract error code from ApiError for metrics
pub fn error_code_from_api_error(error: &crate::error::ApiError) -> &'static str {
    use crate::error::ApiError;
    match error {
        ApiError::PluginNotFound(_)
        | ApiError::TemplateNotFound(_)
        | ApiError::ScenarioNotFound(_) => "not_found",
        ApiError::InvalidVersion(_) => "invalid_version",
        ApiError::PluginExists(_) | ApiError::TemplateExists(_) | ApiError::ScenarioExists(_) => {
            "already_exists"
        }
        ApiError::AuthRequired => "auth_required",
        ApiError::PermissionDenied => "permission_denied",
        ApiError::OrganizationNotFound => "organization_not_found",
        ApiError::InvalidRequest(_) => "invalid_request",
        ApiError::ValidationFailed(_) => "validation_failed",
        ApiError::RateLimitExceeded(_) => "rate_limit_exceeded",
        ApiError::ResourceLimitExceeded(_) => "resource_limit_exceeded",
        ApiError::Database(_) => "database_error",
        ApiError::Storage(_) => "storage_error",
        ApiError::Internal(_) => "internal_error",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ApiError;

    // error_code_from_api_error tests
    #[test]
    fn test_error_code_plugin_not_found() {
        let error = ApiError::PluginNotFound("test".to_string());
        assert_eq!(error_code_from_api_error(&error), "not_found");
    }

    #[test]
    fn test_error_code_template_not_found() {
        let error = ApiError::TemplateNotFound("test".to_string());
        assert_eq!(error_code_from_api_error(&error), "not_found");
    }

    #[test]
    fn test_error_code_scenario_not_found() {
        let error = ApiError::ScenarioNotFound("test".to_string());
        assert_eq!(error_code_from_api_error(&error), "not_found");
    }

    #[test]
    fn test_error_code_invalid_version() {
        let error = ApiError::InvalidVersion("1.0.0".to_string());
        assert_eq!(error_code_from_api_error(&error), "invalid_version");
    }

    #[test]
    fn test_error_code_plugin_exists() {
        let error = ApiError::PluginExists("test".to_string());
        assert_eq!(error_code_from_api_error(&error), "already_exists");
    }

    #[test]
    fn test_error_code_template_exists() {
        let error = ApiError::TemplateExists("test".to_string());
        assert_eq!(error_code_from_api_error(&error), "already_exists");
    }

    #[test]
    fn test_error_code_scenario_exists() {
        let error = ApiError::ScenarioExists("test".to_string());
        assert_eq!(error_code_from_api_error(&error), "already_exists");
    }

    #[test]
    fn test_error_code_auth_required() {
        let error = ApiError::AuthRequired;
        assert_eq!(error_code_from_api_error(&error), "auth_required");
    }

    #[test]
    fn test_error_code_permission_denied() {
        let error = ApiError::PermissionDenied;
        assert_eq!(error_code_from_api_error(&error), "permission_denied");
    }

    #[test]
    fn test_error_code_organization_not_found() {
        let error = ApiError::OrganizationNotFound;
        assert_eq!(error_code_from_api_error(&error), "organization_not_found");
    }

    #[test]
    fn test_error_code_invalid_request() {
        let error = ApiError::InvalidRequest("bad input".to_string());
        assert_eq!(error_code_from_api_error(&error), "invalid_request");
    }

    #[test]
    fn test_error_code_validation_failed() {
        let error = ApiError::ValidationFailed("missing field".to_string());
        assert_eq!(error_code_from_api_error(&error), "validation_failed");
    }

    #[test]
    fn test_error_code_rate_limit_exceeded() {
        let error = ApiError::RateLimitExceeded("too fast".to_string());
        assert_eq!(error_code_from_api_error(&error), "rate_limit_exceeded");
    }

    #[test]
    fn test_error_code_resource_limit_exceeded() {
        let error = ApiError::ResourceLimitExceeded("max plugins".to_string());
        assert_eq!(error_code_from_api_error(&error), "resource_limit_exceeded");
    }

    #[test]
    fn test_error_code_storage() {
        let error = ApiError::Storage("s3 error".to_string());
        assert_eq!(error_code_from_api_error(&error), "storage_error");
    }

    #[test]
    fn test_error_code_internal() {
        let error = ApiError::Internal(anyhow::anyhow!("unknown"));
        assert_eq!(error_code_from_api_error(&error), "internal_error");
    }

    // Test that all error codes are non-empty strings
    #[test]
    fn test_all_error_codes_are_non_empty() {
        let errors = vec![
            ApiError::PluginNotFound("".to_string()),
            ApiError::TemplateNotFound("".to_string()),
            ApiError::ScenarioNotFound("".to_string()),
            ApiError::InvalidVersion("".to_string()),
            ApiError::PluginExists("".to_string()),
            ApiError::TemplateExists("".to_string()),
            ApiError::ScenarioExists("".to_string()),
            ApiError::AuthRequired,
            ApiError::PermissionDenied,
            ApiError::OrganizationNotFound,
            ApiError::InvalidRequest("".to_string()),
            ApiError::ValidationFailed("".to_string()),
            ApiError::RateLimitExceeded("".to_string()),
            ApiError::ResourceLimitExceeded("".to_string()),
            ApiError::Storage("".to_string()),
            ApiError::Internal(anyhow::anyhow!("")),
        ];

        for error in errors {
            let code = error_code_from_api_error(&error);
            assert!(!code.is_empty(), "Error code should not be empty");
            assert!(!code.contains(' '), "Error code should not contain spaces");
        }
    }
}
