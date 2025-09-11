//! Enhanced failure injection system with per-tag include/exclude filters
//! and error rate configuration.

use rand::{rng, Rng};
use std::collections::HashMap;

/// Failure injection configuration
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct FailureConfig {
    /// Global error rate (0.0 to 1.0)
    pub global_error_rate: f64,
    /// Default status codes for failures
    pub default_status_codes: Vec<u16>,
    /// Per-tag error rates and status overrides
    pub tag_configs: HashMap<String, TagFailureConfig>,
    /// Tags to include in failure injection (if empty, all tags are included)
    pub include_tags: Vec<String>,
    /// Tags to exclude from failure injection
    pub exclude_tags: Vec<String>,
}

/// Per-tag failure configuration
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct TagFailureConfig {
    /// Error rate for this tag (0.0 to 1.0)
    pub error_rate: f64,
    /// Status codes for this tag (overrides global defaults)
    pub status_codes: Option<Vec<u16>>,
    /// Custom error message for this tag
    pub error_message: Option<String>,
}

impl Default for FailureConfig {
    fn default() -> Self {
        Self {
            global_error_rate: 0.0,
            default_status_codes: vec![500, 502, 503, 504],
            tag_configs: HashMap::new(),
            include_tags: Vec::new(),
            exclude_tags: Vec::new(),
        }
    }
}

impl Default for TagFailureConfig {
    fn default() -> Self {
        Self {
            error_rate: 0.0,
            status_codes: None,
            error_message: None,
        }
    }
}

/// Enhanced failure injector with tag filtering and error rates
#[derive(Debug, Clone)]
pub struct FailureInjector {
    /// Global failure configuration
    config: Option<FailureConfig>,
    /// Whether failure injection is enabled globally
    enabled: bool,
}

impl FailureInjector {
    /// Create a new failure injector
    pub fn new(config: Option<FailureConfig>, enabled: bool) -> Self {
        Self { config, enabled }
    }

    /// Check if failure injection is enabled globally
    pub fn is_enabled(&self) -> bool {
        self.enabled && self.config.is_some()
    }

    /// Determine if a failure should be injected for the given tags
    pub fn should_inject_failure(&self, tags: &[String]) -> bool {
        if !self.is_enabled() {
            return false;
        }

        let config = match &self.config {
            Some(cfg) => cfg,
            None => return false,
        };

        // Check if any tag is in the exclude list
        if tags.iter().any(|tag| config.exclude_tags.contains(tag)) {
            return false;
        }

        // Check include tags (if specified, only include these tags)
        if !config.include_tags.is_empty() {
            if !tags.iter().any(|tag| config.include_tags.contains(tag)) {
                return false;
            }
        }

        // Find the best matching tag configuration
        let tag_config = self.find_best_tag_config(tags, config);

        // Use tag-specific error rate if available, otherwise global rate
        let error_rate = tag_config
            .map(|tc| tc.error_rate)
            .unwrap_or(config.global_error_rate);

        // Check if failure should occur based on error rate
        if error_rate <= 0.0 {
            return false;
        }
        if error_rate >= 1.0 {
            return true;
        }

        let mut rng = rng();
        rng.random_bool(error_rate)
    }

    /// Get failure response details for the given tags
    pub fn get_failure_response(&self, tags: &[String]) -> Option<(u16, String)> {
        if !self.is_enabled() {
            return None;
        }

        let config = match &self.config {
            Some(cfg) => cfg,
            None => return None,
        };

        // Find the best matching tag configuration
        let tag_config = self.find_best_tag_config(tags, config);

        // Determine status codes to use
        let status_codes = tag_config
            .and_then(|tc| tc.status_codes.clone())
            .unwrap_or_else(|| config.default_status_codes.clone());

        // Determine error message
        let error_message = tag_config
            .and_then(|tc| tc.error_message.clone())
            .unwrap_or_else(|| "Injected failure".to_string());

        // Select a random status code
        let mut rng = rng();
        let status_code = if status_codes.is_empty() {
            500
        } else {
            let index = rng.random_range(0..status_codes.len());
            status_codes[index]
        };

        Some((status_code, error_message))
    }

    /// Find the best matching tag configuration for the given tags
    /// Returns the first matching tag config, or None if no match
    fn find_best_tag_config<'a>(
        &self,
        tags: &[String],
        config: &'a FailureConfig,
    ) -> Option<&'a TagFailureConfig> {
        // Look for the first tag that has a configuration
        for tag in tags {
            if let Some(tag_config) = config.tag_configs.get(tag) {
                return Some(tag_config);
            }
        }
        None
    }

    /// Process a request with failure injection
    /// Returns Some((status_code, error_message)) if failure should be injected, None otherwise
    pub fn process_request(&self, tags: &[String]) -> Option<(u16, String)> {
        if self.should_inject_failure(tags) {
            self.get_failure_response(tags)
        } else {
            None
        }
    }

    /// Update the failure configuration
    pub fn update_config(&mut self, config: Option<FailureConfig>) {
        self.config = config;
    }

    /// Enable or disable failure injection
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

impl Default for FailureInjector {
    fn default() -> Self {
        Self::new(None, false)
    }
}

/// Helper function to create a failure injector from core config
pub fn create_failure_injector(
    failures_enabled: bool,
    failure_config: Option<FailureConfig>,
) -> FailureInjector {
    FailureInjector::new(failure_config, failures_enabled)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_config() -> FailureConfig {
        let mut tag_configs = HashMap::new();
        tag_configs.insert(
            "auth".to_string(),
            TagFailureConfig {
                error_rate: 0.1,
                status_codes: Some(vec![401, 403]),
                error_message: Some("Authentication failed".to_string()),
            },
        );
        tag_configs.insert(
            "payments".to_string(),
            TagFailureConfig {
                error_rate: 0.05,
                status_codes: Some(vec![402, 503]),
                error_message: Some("Payment failed".to_string()),
            },
        );

        FailureConfig {
            global_error_rate: 0.02,
            default_status_codes: vec![500, 502],
            tag_configs,
            include_tags: Vec::new(),
            exclude_tags: vec!["health".to_string()],
        }
    }

    #[test]
    fn test_failure_injector_disabled() {
        let injector = FailureInjector::new(Some(create_test_config()), false);
        assert!(!injector.is_enabled());
        assert!(!injector.should_inject_failure(&["auth".to_string()]));
        assert!(injector.get_failure_response(&["auth".to_string()]).is_none());
    }

    #[test]
    fn test_failure_injector_no_config() {
        let injector = FailureInjector::new(None, true);
        assert!(!injector.is_enabled());
        assert!(!injector.should_inject_failure(&["auth".to_string()]));
    }

    #[test]
    fn test_exclude_tags() {
        let injector = FailureInjector::new(Some(create_test_config()), true);
        assert!(!injector.should_inject_failure(&["health".to_string()]));
        assert!(!injector.should_inject_failure(&["health".to_string(), "auth".to_string()]));
    }

    #[test]
    fn test_include_tags() {
        let mut config = create_test_config();
        config.include_tags = vec!["auth".to_string()];
        // Set error rate to 1.0 to ensure failure injection
        config.tag_configs.get_mut("auth").unwrap().error_rate = 1.0;
        let injector = FailureInjector::new(Some(config), true);

        assert!(injector.should_inject_failure(&["auth".to_string()]));
        assert!(!injector.should_inject_failure(&["payments".to_string()]));
        assert!(!injector.should_inject_failure(&["other".to_string()]));
    }

    #[test]
    fn test_tag_config_priority() {
        let injector = FailureInjector::new(Some(create_test_config()), true);

        // Test with auth tag (should use auth config)
        let result = injector.get_failure_response(&["auth".to_string()]);
        assert!(result.is_some());
        let (status, message) = result.unwrap();
        assert!(status == 401 || status == 403);
        assert_eq!(message, "Authentication failed");

        // Test with payments tag (should use payments config)
        let result = injector.get_failure_response(&["payments".to_string()]);
        assert!(result.is_some());
        let (status, message) = result.unwrap();
        assert!(status == 402 || status == 503);
        assert_eq!(message, "Payment failed");
    }

    #[test]
    fn test_global_config_fallback() {
        let injector = FailureInjector::new(Some(create_test_config()), true);

        // Test with unknown tag (should use global config)
        let result = injector.get_failure_response(&["unknown".to_string()]);
        assert!(result.is_some());
        let (status, message) = result.unwrap();
        assert!(status == 500 || status == 502);
        assert_eq!(message, "Injected failure");
    }
}
