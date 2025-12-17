//! 404 detection middleware for the runtime daemon
//!
//! This module provides middleware that detects 404 responses and triggers
//! automatic mock generation.

use axum::{extract::Request, http::StatusCode, middleware::Next, response::Response};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::auto_generator::AutoGenerator;
use crate::config::RuntimeDaemonConfig;

/// State for the 404 detector middleware
#[derive(Clone)]
pub struct NotFoundDetector {
    /// Daemon configuration
    config: Arc<RuntimeDaemonConfig>,
    /// Auto-generator for creating mocks (wrapped in Arc for sharing)
    generator: Arc<RwLock<Option<Arc<AutoGenerator>>>>,
}

impl NotFoundDetector {
    /// Create a new 404 detector
    pub fn new(config: RuntimeDaemonConfig) -> Self {
        Self {
            config: Arc::new(config),
            generator: Arc::new(RwLock::new(None)),
        }
    }

    /// Set the auto-generator
    pub async fn set_generator(&self, generator: Arc<AutoGenerator>) {
        let mut gen = self.generator.write().await;
        *gen = Some(generator);
    }

    /// Check if a path should be excluded from auto-generation
    fn should_exclude(&self, path: &str) -> bool {
        self.config.exclude_patterns.iter().any(|pattern| {
            if pattern.starts_with('/') {
                path.starts_with(pattern)
            } else {
                path.contains(pattern)
            }
        })
    }

    /// Middleware function that detects 404s and triggers auto-generation
    pub async fn detect_and_auto_create(self, request: Request, next: Next) -> Response {
        // Extract request details before consuming the request
        let method = request.method().clone();
        let uri = request.uri().clone();
        let path = uri.path().to_string();

        // Execute the request and get the response
        let response = next.run(request).await;

        // Only process if daemon is enabled and auto-create is enabled
        if !self.config.enabled || !self.config.auto_create_on_404 {
            return response;
        }

        // Check if response is a 404
        if response.status() != StatusCode::NOT_FOUND {
            return response;
        }

        // Check if path should be excluded
        if self.should_exclude(&path) {
            debug!("Excluding path from auto-generation: {}", path);
            return response;
        }

        info!("Detected 404 for {} {}, triggering auto-generation", method, path);

        // Try to auto-generate a mock
        let generator = self.generator.read().await;
        if let Some(ref gen) = *generator {
            // Clone what we need for async operation
            let method_str = method.to_string();
            let path_str = path.clone();
            let gen_clone = Arc::clone(gen);

            // Spawn async task to generate mock (don't block the response)
            tokio::spawn(async move {
                if let Err(e) = gen_clone.generate_mock_from_404(&method_str, &path_str).await {
                    warn!("Failed to auto-generate mock for {} {}: {}", method_str, path_str, e);
                } else {
                    info!("Successfully auto-generated mock for {} {}", method_str, path_str);
                }
            });
        } else {
            debug!("Auto-generator not available, skipping mock creation");
        }

        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_detector() {
        let config = RuntimeDaemonConfig::default();
        let detector = NotFoundDetector::new(config);
        assert!(detector.config.exclude_patterns.contains(&"/health".to_string()));
    }

    #[test]
    fn test_detector_clone() {
        let config = RuntimeDaemonConfig {
            enabled: true,
            exclude_patterns: vec!["/test".to_string()],
            ..Default::default()
        };
        let detector = NotFoundDetector::new(config);
        let cloned = detector.clone();
        assert!(cloned.config.enabled);
        assert!(cloned.config.exclude_patterns.contains(&"/test".to_string()));
    }

    #[tokio::test]
    async fn test_should_exclude() {
        let config = RuntimeDaemonConfig {
            exclude_patterns: vec!["/health".to_string(), "/metrics".to_string()],
            ..Default::default()
        };
        let detector = NotFoundDetector::new(config);

        assert!(detector.should_exclude("/health"));
        assert!(detector.should_exclude("/health/check"));
        assert!(detector.should_exclude("/metrics"));
        assert!(!detector.should_exclude("/api/users"));
    }

    #[test]
    fn test_should_exclude_prefix_patterns() {
        let config = RuntimeDaemonConfig {
            exclude_patterns: vec!["/internal".to_string(), "/admin".to_string()],
            ..Default::default()
        };
        let detector = NotFoundDetector::new(config);

        // Prefix patterns (start with /) should match path starts
        assert!(detector.should_exclude("/internal"));
        assert!(detector.should_exclude("/internal/api"));
        assert!(detector.should_exclude("/internal/users/123"));
        assert!(detector.should_exclude("/admin"));
        assert!(detector.should_exclude("/admin/dashboard"));
        assert!(!detector.should_exclude("/api/internal"));
    }

    #[test]
    fn test_should_exclude_contains_patterns() {
        let config = RuntimeDaemonConfig {
            exclude_patterns: vec!["secret".to_string(), "private".to_string()],
            ..Default::default()
        };
        let detector = NotFoundDetector::new(config);

        // Contains patterns (not starting with /) should match anywhere
        assert!(detector.should_exclude("/api/secret/data"));
        assert!(detector.should_exclude("/secret"));
        assert!(detector.should_exclude("/users/secret/key"));
        assert!(detector.should_exclude("/private/info"));
        assert!(detector.should_exclude("/api/private"));
        assert!(!detector.should_exclude("/api/public/data"));
    }

    #[test]
    fn test_should_exclude_mixed_patterns() {
        let config = RuntimeDaemonConfig {
            exclude_patterns: vec![
                "/health".to_string(),  // Prefix pattern
                "internal".to_string(), // Contains pattern
            ],
            ..Default::default()
        };
        let detector = NotFoundDetector::new(config);

        // Prefix pattern
        assert!(detector.should_exclude("/health"));
        assert!(detector.should_exclude("/health/check"));
        assert!(!detector.should_exclude("/api/health")); // /health only matches start

        // Contains pattern
        assert!(detector.should_exclude("/internal"));
        assert!(detector.should_exclude("/api/internal"));
        assert!(detector.should_exclude("/internal/api"));
    }

    #[test]
    fn test_should_exclude_empty_patterns() {
        let config = RuntimeDaemonConfig {
            exclude_patterns: vec![],
            ..Default::default()
        };
        let detector = NotFoundDetector::new(config);

        // Nothing should be excluded
        assert!(!detector.should_exclude("/health"));
        assert!(!detector.should_exclude("/metrics"));
        assert!(!detector.should_exclude("/api/users"));
    }

    #[test]
    fn test_should_exclude_default_patterns() {
        let config = RuntimeDaemonConfig::default();
        let detector = NotFoundDetector::new(config);

        // Default patterns include /health, /metrics, /__mockforge
        assert!(detector.should_exclude("/health"));
        assert!(detector.should_exclude("/metrics"));
        assert!(detector.should_exclude("/__mockforge"));
        assert!(detector.should_exclude("/__mockforge/api/mocks"));
        assert!(!detector.should_exclude("/api/users"));
    }

    #[tokio::test]
    async fn test_set_generator() {
        let config = RuntimeDaemonConfig::default();
        let detector = NotFoundDetector::new(config.clone());

        // Initially generator should be None
        {
            let gen = detector.generator.read().await;
            assert!(gen.is_none());
        }

        // Set a generator
        let auto_gen = Arc::new(AutoGenerator::new(config, "http://localhost:3000".to_string()));
        detector.set_generator(auto_gen).await;

        // Now generator should be Some
        {
            let gen = detector.generator.read().await;
            assert!(gen.is_some());
        }
    }

    #[test]
    fn test_detector_config_arc_sharing() {
        let config = RuntimeDaemonConfig {
            enabled: true,
            auto_create_on_404: true,
            ..Default::default()
        };
        let detector = NotFoundDetector::new(config);
        let cloned = detector.clone();

        // Both should point to the same Arc
        assert!(Arc::ptr_eq(&detector.config, &cloned.config));
    }
}
