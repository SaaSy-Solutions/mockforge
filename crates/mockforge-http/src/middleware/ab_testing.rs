//! A/B testing middleware for HTTP requests
//!
//! This middleware intercepts requests, selects appropriate variants based on
//! A/B test configuration, and applies the variant response.

use crate::handlers::ab_testing::ABTestingState;
use axum::{body::Body, extract::Request, http::StatusCode, middleware::Next, response::Response};
use mockforge_core::ab_testing::{apply_variant_to_response, select_variant};
use std::time::Instant;
use tracing::{debug, trace};

/// A/B testing middleware
///
/// This middleware:
/// 1. Checks if there's an A/B test configured for the request endpoint
/// 2. Selects a variant based on the test configuration
/// 3. Applies the variant response (status code, headers, body)
/// 4. Records analytics for the selected variant
pub async fn ab_testing_middleware(req: Request, next: Next) -> Response<Body> {
    let start_time = Instant::now();

    // Extract method and path before borrowing req
    let method = req.method().to_string();
    let path = req.uri().path().to_string();
    let uri = req.uri().to_string();

    // Extract headers before borrowing req
    let headers = req.headers().clone();

    // Get A/B testing state from extensions (clone to avoid borrow issues)
    let state_opt = req.extensions().get::<ABTestingState>().cloned();

    if let Some(state) = state_opt {
        // Get A/B test configuration for this endpoint
        if let Some(test_config) = state.variant_manager.get_test(&method, &path).await {
            trace!("A/B test found for {} {}", method, path);

            // Select a variant using extracted headers and URI
            match select_variant(&test_config, &headers, &uri, &state.variant_manager).await {
                Ok(Some(variant)) => {
                    debug!("Selected variant '{}' for {} {}", variant.variant_id, method, path);

                    // Continue with request processing
                    let response = next.run(req).await;

                    // Apply variant to response
                    let response = apply_variant_to_response(&variant, response);

                    // Record analytics
                    let response_time_ms = start_time.elapsed().as_millis() as f64;
                    let status_code = response.status().as_u16();
                    state
                        .variant_manager
                        .record_request(
                            &method,
                            &path,
                            &variant.variant_id,
                            status_code,
                            response_time_ms,
                        )
                        .await;

                    // Add latency if configured
                    if let Some(latency_ms) = variant.latency_ms {
                        tokio::time::sleep(tokio::time::Duration::from_millis(latency_ms)).await;
                    }

                    return response;
                }
                Ok(None) => {
                    trace!("No variant selected for {} {}", method, path);
                }
                Err(e) => {
                    debug!("Error selecting variant for {} {}: {}", method, path, e);
                }
            }
        }
    }

    // No A/B test configured or variant selection failed - proceed normally
    next.run(req).await
}
