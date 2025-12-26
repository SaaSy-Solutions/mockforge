//! A/B testing middleware for variant selection
//!
//! This module provides middleware functionality for selecting and applying
//! mock variants based on A/B test configuration.

use crate::ab_testing::manager::VariantManager;
use crate::ab_testing::types::{ABTestConfig, MockVariant, VariantSelectionStrategy};
use crate::error::Result;
use axum::body::Body;
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use rand::Rng;
use std::sync::Arc;
use tracing::warn;

/// State for A/B testing middleware
#[derive(Clone)]
pub struct ABTestingMiddlewareState {
    /// Variant manager
    pub variant_manager: Arc<VariantManager>,
}

impl ABTestingMiddlewareState {
    /// Create new middleware state
    pub fn new(variant_manager: Arc<VariantManager>) -> Self {
        Self { variant_manager }
    }
}

/// Select a variant for a request based on A/B test configuration
///
/// This function extracts all needed data from the request before selection
pub async fn select_variant(
    config: &ABTestConfig,
    request_headers: &HeaderMap,
    request_uri: &str,
    variant_manager: &VariantManager,
) -> Result<Option<MockVariant>> {
    // Check if test is enabled and within time window
    if !config.enabled {
        return Ok(None);
    }

    let now = chrono::Utc::now();
    if let Some(start_time) = config.start_time {
        if now < start_time {
            return Ok(None);
        }
    }
    if let Some(end_time) = config.end_time {
        if now > end_time {
            return Ok(None);
        }
    }

    // Select variant based on strategy
    let variant_id = match config.strategy {
        VariantSelectionStrategy::Random => select_variant_random(&config.allocations)?,
        VariantSelectionStrategy::ConsistentHash => {
            select_variant_consistent_hash(config, request_headers, request_uri)?
        }
        VariantSelectionStrategy::RoundRobin => {
            select_variant_round_robin(config, variant_manager).await?
        }
        VariantSelectionStrategy::StickySession => {
            select_variant_sticky_session(config, request_headers)?
        }
    };

    // Find the selected variant
    let variant = config.variants.iter().find(|v| v.variant_id == variant_id).cloned();

    if variant.is_none() {
        warn!("Selected variant '{}' not found in test '{}'", variant_id, config.test_name);
    }

    Ok(variant)
}

/// Select variant using random allocation
fn select_variant_random(
    allocations: &[crate::ab_testing::types::VariantAllocation],
) -> Result<String> {
    let mut rng = rand::thread_rng();
    let random_value = rng.gen_range(0.0..100.0);
    let mut cumulative = 0.0;

    for allocation in allocations {
        cumulative += allocation.percentage;
        if random_value <= cumulative {
            return Ok(allocation.variant_id.clone());
        }
    }

    // Fallback to last variant if rounding errors
    allocations.last().map(|a| Ok(a.variant_id.clone())).unwrap_or_else(|| {
        Err(crate::error::Error::validation("No allocations defined".to_string()))
    })
}

/// Select variant using consistent hashing
fn select_variant_consistent_hash(
    config: &ABTestConfig,
    request_headers: &HeaderMap,
    request_uri: &str,
) -> Result<String> {
    // Try to extract a consistent attribute (e.g., user ID, IP address)
    let attribute = extract_hash_attribute(request_headers, request_uri);

    // Hash the attribute to get a value between 0-100
    let hash_value = VariantManager::consistent_hash(&attribute, 100) as f64;

    // Find which allocation bucket this hash falls into
    let mut cumulative = 0.0;
    for allocation in &config.allocations {
        cumulative += allocation.percentage;
        if hash_value <= cumulative {
            return Ok(allocation.variant_id.clone());
        }
    }

    // Fallback
    config.allocations.last().map(|a| Ok(a.variant_id.clone())).unwrap_or_else(|| {
        Err(crate::error::Error::validation("No allocations defined".to_string()))
    })
}

/// Select variant using round-robin
async fn select_variant_round_robin(
    config: &ABTestConfig,
    variant_manager: &VariantManager,
) -> Result<String> {
    let index = variant_manager
        .increment_round_robin(&config.method, &config.endpoint_path, config.allocations.len())
        .await;

    config
        .allocations
        .get(index)
        .map(|a| Ok(a.variant_id.clone()))
        .unwrap_or_else(|| {
            Err(crate::error::Error::validation("Invalid allocation index".to_string()))
        })
}

/// Select variant using sticky session
fn select_variant_sticky_session(
    config: &ABTestConfig,
    request_headers: &HeaderMap,
) -> Result<String> {
    // Try to get session ID from cookie or header
    let session_id = extract_session_id(request_headers);

    // Use consistent hashing on session ID
    let hash_value = VariantManager::consistent_hash(&session_id, 100) as f64;

    let mut cumulative = 0.0;
    for allocation in &config.allocations {
        cumulative += allocation.percentage;
        if hash_value <= cumulative {
            return Ok(allocation.variant_id.clone());
        }
    }

    // Fallback
    config.allocations.last().map(|a| Ok(a.variant_id.clone())).unwrap_or_else(|| {
        Err(crate::error::Error::validation("No allocations defined".to_string()))
    })
}

/// Extract a consistent attribute for hashing from request
fn extract_hash_attribute(request_headers: &HeaderMap, request_uri: &str) -> String {
    // Try to get user ID from headers
    if let Some(user_id) = request_headers.get("X-User-ID") {
        if let Ok(user_id_str) = user_id.to_str() {
            return format!("user:{}", user_id_str);
        }
    }

    // Try to get user ID from query parameters
    if let Some(query_start) = request_uri.find('?') {
        let query = &request_uri[query_start + 1..];
        for param in query.split('&') {
            if let Some((key, value)) = param.split_once('=') {
                if key == "user_id" || key == "userId" {
                    return format!("user:{}", value);
                }
            }
        }
    }

    // Fallback to IP address
    if let Some(ip) = request_headers.get("X-Forwarded-For") {
        if let Ok(ip_str) = ip.to_str() {
            return format!("ip:{}", ip_str.split(',').next().unwrap_or("unknown"));
        }
    }

    // Final fallback: use a random value (not ideal for consistent hashing)
    format!("random:{}", uuid::Uuid::new_v4())
}

/// Extract session ID from request
fn extract_session_id(request_headers: &HeaderMap) -> String {
    // Try to get session ID from cookie
    if let Some(cookie_header) = request_headers.get("Cookie") {
        if let Ok(cookie_str) = cookie_header.to_str() {
            for cookie in cookie_str.split(';') {
                let cookie = cookie.trim();
                if let Some((key, value)) = cookie.split_once('=') {
                    if key == "session_id" || key == "sessionId" || key == "JSESSIONID" {
                        return value.to_string();
                    }
                }
            }
        }
    }

    // Try to get session ID from header
    if let Some(session_id) = request_headers.get("X-Session-ID") {
        if let Ok(session_id_str) = session_id.to_str() {
            return session_id_str.to_string();
        }
    }

    // Fallback: generate a session ID based on IP
    // We need to pass a dummy URI since extract_hash_attribute needs it
    extract_hash_attribute(request_headers, "")
}

/// Apply variant to response
pub fn apply_variant_to_response(
    variant: &MockVariant,
    _response: Response<Body>,
) -> Response<Body> {
    // Create new response with variant body
    let mut response_builder = Response::builder().status(
        StatusCode::from_u16(variant.status_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
    );

    // Add variant headers
    for (key, value) in &variant.headers {
        if let (Ok(key), Ok(value)) = (
            axum::http::HeaderName::try_from(key.as_str()),
            axum::http::HeaderValue::try_from(value.as_str()),
        ) {
            response_builder = response_builder.header(key, value);
        }
    }

    // Add variant ID header for tracking
    if let Ok(header_name) = axum::http::HeaderName::try_from("X-MockForge-Variant") {
        if let Ok(header_value) = axum::http::HeaderValue::try_from(variant.variant_id.as_str()) {
            response_builder = response_builder.header(header_name, header_value);
        }
    }

    // Convert variant body to response body
    let body = match serde_json::to_string(&variant.body) {
        Ok(json_str) => Body::from(json_str),
        Err(_) => Body::from("{}"), // Fallback to empty JSON
    };

    response_builder.body(body).unwrap_or_else(|_| {
        // Fallback response if building fails
        // This expect is safe because we're using known valid constants
        Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from("{}"))
            .expect("fallback response with valid status and body should never fail")
    })
}
