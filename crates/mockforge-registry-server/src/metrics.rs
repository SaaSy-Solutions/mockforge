//! Metrics instrumentation helpers for marketplace operations

use std::time::Instant;
use mockforge_observability::prometheus::MetricsRegistry;
use std::sync::Arc;

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
        ApiError::PluginNotFound(_) | ApiError::TemplateNotFound(_) | ApiError::ScenarioNotFound(_) => "not_found",
        ApiError::InvalidVersion(_) => "invalid_version",
        ApiError::PluginExists(_) | ApiError::TemplateExists(_) | ApiError::ScenarioExists(_) => "already_exists",
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
