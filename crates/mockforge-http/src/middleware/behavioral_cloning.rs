//! Behavioral cloning middleware
//!
//! This middleware applies learned behavioral patterns to requests,
//! including probabilistic status codes, latency, and error patterns.

use axum::{
    body::Body,
    extract::{Path, State},
    http::{Request, Response, StatusCode},
    middleware::Next,
};
use mockforge_core::behavioral_cloning::{
    ProbabilisticModel, SequenceLearner,
};
use mockforge_recorder::database::RecorderDatabase;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, trace};

/// Behavioral cloning middleware state
#[derive(Clone)]
pub struct BehavioralCloningMiddlewareState {
    /// Optional recorder database path
    pub database_path: Option<PathBuf>,
    /// Whether behavioral cloning is enabled
    pub enabled: bool,
    /// Cache for loaded probability models (to avoid repeated DB queries)
    pub model_cache: Arc<tokio::sync::RwLock<HashMap<String, mockforge_core::behavioral_cloning::EndpointProbabilityModel>>>,
}

impl BehavioralCloningMiddlewareState {
    /// Create new middleware state
    pub fn new() -> Self {
        Self {
            database_path: None,
            enabled: true,
            model_cache: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Create state with database path
    pub fn with_database_path(path: PathBuf) -> Self {
        Self {
            database_path: Some(path),
            enabled: true,
            model_cache: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Open database connection
    async fn open_database(&self) -> Option<RecorderDatabase> {
        let db_path = self
            .database_path
            .as_ref()
            .cloned()
            .unwrap_or_else(|| {
                std::env::current_dir()
                    .unwrap_or_else(|_| PathBuf::from("."))
                    .join("recordings.db")
            });

        RecorderDatabase::new(&db_path).await.ok()
    }

    /// Get probability model for endpoint (with caching)
    async fn get_probability_model(
        &self,
        endpoint: &str,
        method: &str,
    ) -> Option<mockforge_core::behavioral_cloning::EndpointProbabilityModel> {
        let cache_key = format!("{}:{}", method, endpoint);

        // Check cache first
        {
            let cache = self.model_cache.read().await;
            if let Some(model) = cache.get(&cache_key) {
                return Some(model.clone());
            }
        }

        // Load from database
        if let Some(db) = self.open_database().await {
            if let Ok(Some(model)) = db.get_endpoint_probability_model(endpoint, method).await {
                // Store in cache
                let mut cache = self.model_cache.write().await;
                cache.insert(cache_key, model.clone());
                return Some(model);
            }
        }

        None
    }
}

impl Default for BehavioralCloningMiddlewareState {
    fn default() -> Self {
        Self::new()
    }
}

/// Behavioral cloning middleware
///
/// Applies learned behavioral patterns to requests:
/// - Samples status codes from probability models
/// - Applies latency based on learned distributions
/// - Injects error patterns based on learned probabilities
pub async fn behavioral_cloning_middleware(
    req: Request<Body>,
    next: Next,
) -> Response<Body> {
    // Extract state from extensions (set by router)
    let state = req.extensions().get::<BehavioralCloningMiddlewareState>().cloned();

    // If no state or disabled, pass through
    let state = match state {
        Some(s) if s.enabled => s,
        _ => return next.run(req).await,
    };

    // Extract endpoint and method
    let method = req.method().to_string();
    let path = req.uri().path().to_string();

    // Get probability model for this endpoint
    let model = state.get_probability_model(&path, &method).await;

    if let Some(model) = model {
        debug!(
            "Applying behavioral cloning to {} {}",
            method, path
        );

        // Sample status code
        let sampled_status = ProbabilisticModel::sample_status_code(&model);

        // Sample latency
        let sampled_latency = ProbabilisticModel::sample_latency(&model);

        // Apply latency delay
        if sampled_latency > 0 {
            trace!("Applying latency delay: {}ms", sampled_latency);
            sleep(Duration::from_millis(sampled_latency)).await;
        }

        // Sample error pattern
        let error_pattern = ProbabilisticModel::sample_error_pattern(&model, None);

        // If we sampled an error status code, we need to modify the response
        // However, we can't easily modify the response status in middleware
        // without intercepting it. For now, we'll just apply latency.
        // Full error injection would require response interception middleware.

        if let Some(pattern) = error_pattern {
            debug!(
                "Sampled error pattern: {} (probability: {})",
                pattern.error_type, pattern.probability
            );
            // TODO: Apply error pattern to response
            // This would require response interception middleware
        }

        // Continue with request (status code modification would need response interception)
        let mut response = next.run(req).await;

        // Modify response status if we sampled a different status code
        if sampled_status != response.status().as_u16() {
            *response.status_mut() = StatusCode::from_u16(sampled_status)
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        }

        response
    } else {
        // No model found, pass through
        next.run(req).await
    }
}
