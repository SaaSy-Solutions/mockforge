//! Production headers middleware for deceptive deploy
//!
//! This module provides middleware that adds production-like headers to all responses,
//! supporting template expansion for dynamic values like request IDs.

use axum::{
    body::Body,
    extract::State,
    http::{HeaderName, HeaderValue, Request},
    middleware::Next,
    response::Response,
};
use tracing::debug;
use uuid::Uuid;

use crate::HttpServerState;

/// Production headers middleware
///
/// Adds configured headers to all responses, with support for template expansion.
/// Templates supported:
/// - `{{uuid}}` - Generates a new UUID for each request
/// - `{{now}}` - Current timestamp in RFC3339 format
/// - `{{timestamp}}` - Current Unix timestamp
pub async fn production_headers_middleware(
    State(state): State<HttpServerState>,
    req: Request<Body>,
    next: Next,
) -> Response<Body> {
    // Process the request
    let mut response = next.run(req).await;

    // Get headers configuration from state
    if let Some(headers) = &state.production_headers {
        for (key, value) in headers.iter() {
            // Expand templates in header values
            let expanded_value = expand_templates(value);

            // Parse header name and value
            if let (Ok(header_name), Ok(header_value)) =
                (key.parse::<HeaderName>(), expanded_value.parse::<HeaderValue>())
            {
                // Only add if not already present (don't override existing headers)
                if !response.headers().contains_key(&header_name) {
                    response.headers_mut().insert(header_name, header_value);
                    debug!("Added production header: {} = {}", key, expanded_value);
                }
            } else {
                tracing::warn!("Failed to parse production header: {} = {}", key, expanded_value);
            }
        }
    }

    response
}

/// Expand template placeholders in header values
///
/// Supported templates:
/// - `{{uuid}}` - Generates a new UUID v4
/// - `{{now}}` - Current timestamp in RFC3339 format
/// - `{{timestamp}}` - Current Unix timestamp (seconds)
fn expand_templates(value: &str) -> String {
    let mut result = value.to_string();

    // Replace {{uuid}} with a new UUID
    if result.contains("{{uuid}}") {
        let uuid = Uuid::new_v4().to_string();
        result = result.replace("{{uuid}}", &uuid);
    }

    // Replace {{now}} with current RFC3339 timestamp
    if result.contains("{{now}}") {
        let now = chrono::Utc::now().to_rfc3339();
        result = result.replace("{{now}}", &now);
    }

    // Replace {{timestamp}} with Unix timestamp
    if result.contains("{{timestamp}}") {
        let timestamp = chrono::Utc::now().timestamp().to_string();
        result = result.replace("{{timestamp}}", &timestamp);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_uuid_template() {
        let value = "{{uuid}}";
        let expanded = expand_templates(value);
        // UUID should be 36 characters (with hyphens)
        assert_eq!(expanded.len(), 36);
        assert!(!expanded.contains("{{uuid}}"));
    }

    #[test]
    fn test_expand_now_template() {
        let value = "{{now}}";
        let expanded = expand_templates(value);
        // RFC3339 timestamp should be around 20 characters
        assert!(expanded.len() > 15);
        assert!(!expanded.contains("{{now}}"));
        // Should contain 'T' separator (RFC3339 format)
        assert!(expanded.contains('T'));
    }

    #[test]
    fn test_expand_timestamp_template() {
        let value = "{{timestamp}}";
        let expanded = expand_templates(value);
        // Unix timestamp should be numeric
        assert!(expanded.parse::<i64>().is_ok());
        assert!(!expanded.contains("{{timestamp}}"));
    }

    #[test]
    fn test_expand_multiple_templates() {
        let value = "Request-{{uuid}} at {{timestamp}}";
        let expanded = expand_templates(value);
        assert!(!expanded.contains("{{uuid}}"));
        assert!(!expanded.contains("{{timestamp}}"));
        assert!(expanded.starts_with("Request-"));
    }

    #[test]
    fn test_no_templates() {
        let value = "Static header value";
        let expanded = expand_templates(value);
        assert_eq!(expanded, value);
    }
}
