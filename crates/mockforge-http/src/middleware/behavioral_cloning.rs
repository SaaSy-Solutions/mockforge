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
use mockforge_core::behavioral_cloning::{ProbabilisticModel, SequenceLearner};
use mockforge_recorder::database::RecorderDatabase;
use rand::Rng;
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
    pub model_cache: Arc<
        tokio::sync::RwLock<
            HashMap<String, mockforge_core::behavioral_cloning::EndpointProbabilityModel>,
        >,
    >,
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
        let db_path = self.database_path.as_ref().cloned().unwrap_or_else(|| {
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
pub async fn behavioral_cloning_middleware(req: Request<Body>, next: Next) -> Response<Body> {
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
        debug!("Applying behavioral cloning to {} {}", method, path);

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

        // Continue with request
        let mut response = next.run(req).await;

        // Apply error pattern if sampled
        if let Some(pattern) = &error_pattern {
            debug!(
                "Applying error pattern: {} (probability: {})",
                pattern.error_type, pattern.probability
            );

            // Update status code if pattern has one
            if let Some(pattern_status) = pattern.status_code {
                *response.status_mut() = StatusCode::from_u16(pattern_status)
                    .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            } else if sampled_status != response.status().as_u16() {
                // Use sampled status if pattern doesn't specify one
                *response.status_mut() = StatusCode::from_u16(sampled_status)
                    .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            }

            // Apply error pattern body if sample responses are available
            if !pattern.sample_responses.is_empty() {
                use axum::body::HttpBody;
                use axum::body::Body;

                // Pick a random sample response (or first one)
                let sample_idx = if pattern.sample_responses.len() > 1 {
                    rand::thread_rng().gen_range(0..pattern.sample_responses.len())
                } else {
                    0
                };

                if let Some(sample_body) = pattern.sample_responses.get(sample_idx) {
                    // Serialize the sample response to JSON
                    if let Ok(json_string) = serde_json::to_string(sample_body) {
                        // Replace response body
                        *response.body_mut() = Body::from(json_string);

                        // Set content-type header
                        response.headers_mut().insert(
                            axum::http::header::CONTENT_TYPE,
                            axum::http::HeaderValue::from_static("application/json"),
                        );

                        debug!("Applied error pattern body from sample response");
                    }
                }
            }
        } else {
            // No error pattern, but still apply sampled status code if different
            if sampled_status != response.status().as_u16() {
                *response.status_mut() = StatusCode::from_u16(sampled_status)
                    .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }

        response
    } else {
        // No model found, pass through
        next.run(req).await
    }
}
