//! 404 detection middleware for the runtime daemon
//!
//! This module provides middleware that detects 404 responses and triggers
//! automatic mock generation.

use axum::{
    extract::Request,
    http::{Method, StatusCode, Uri},
    middleware::Next,
    response::Response,
};
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
    pub async fn detect_and_auto_create(
        self,
        request: Request,
        next: Next,
    ) -> Response {
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

        info!(
            "Detected 404 for {} {}, triggering auto-generation",
            method, path
        );

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
}

