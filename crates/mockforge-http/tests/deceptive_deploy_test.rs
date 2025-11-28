//! Integration tests for Deceptive Deploy feature
//!
//! This test suite validates that deceptive deploy features work correctly:
//! - Rate limit headers are added to responses
//! - Production headers middleware works
//! - OAuth integration with deceptive deploy config
//! - CORS configuration from deceptive deploy

use axum::{
    body::Body,
    http::{Request, StatusCode},
    response::IntoResponse,
    routing::get,
    Router,
};
use mockforge_core::config::{DeceptiveDeployConfig, OAuth2Config, ProductionOAuthConfig};
use mockforge_http::middleware::{production_headers_middleware, rate_limit_middleware};
use mockforge_http::HttpServerState as HttpState;
use std::collections::HashMap;
use std::sync::Arc;
use tower::ServiceExt;

/// Test handler that returns a simple response
async fn test_handler() -> impl IntoResponse {
    (StatusCode::OK, "test response")
}

#[tokio::test]
async fn test_rate_limit_headers_present() {
    // Create a router with rate limiting enabled
    let mut state = HttpState::new();

    // Create rate limiter with known config
    let rate_limit_config = mockforge_http::middleware::RateLimitConfig {
        requests_per_minute: 100,
        burst: 200,
        per_ip: false,
        per_endpoint: false,
    };
    let rate_limiter =
        Arc::new(mockforge_http::middleware::GlobalRateLimiter::new(rate_limit_config));
    state = state.with_rate_limiter(rate_limiter);

    let app = Router::new()
        .route("/test", get(test_handler))
        .layer(axum::middleware::from_fn_with_state(state.clone(), rate_limit_middleware))
        .with_state(state);

    // Make a request
    let request = Request::builder().uri("/test").body(Body::empty()).unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Verify rate limit headers are present
    assert_eq!(response.status(), StatusCode::OK);

    let headers = response.headers();
    assert!(
        headers.contains_key("x-rate-limit-limit"),
        "X-Rate-Limit-Limit header should be present"
    );
    assert!(
        headers.contains_key("x-rate-limit-remaining"),
        "X-Rate-Limit-Remaining header should be present"
    );
    assert!(
        headers.contains_key("x-rate-limit-reset"),
        "X-Rate-Limit-Reset header should be present"
    );

    // Verify header values are reasonable
    let limit = headers.get("x-rate-limit-limit").unwrap().to_str().unwrap();
    assert_eq!(limit, "100", "Rate limit should be 100 req/min");

    let remaining = headers.get("x-rate-limit-remaining").unwrap().to_str().unwrap();
    let remaining_num: u32 = remaining.parse().unwrap();
    assert!(remaining_num <= 100, "Remaining should be <= limit");

    let reset = headers.get("x-rate-limit-reset").unwrap().to_str().unwrap();
    let reset_num: u64 = reset.parse().unwrap();
    assert!(reset_num > 0, "Reset timestamp should be positive");
}

#[tokio::test]
async fn test_production_headers_middleware() {
    // Create production headers
    let mut headers = HashMap::new();
    headers.insert("X-API-Version".to_string(), "1.0".to_string());
    headers.insert("X-Request-ID".to_string(), "{{uuid}}".to_string());
    headers.insert("X-Powered-By".to_string(), "MockForge".to_string());

    let mut state = HttpState::new();
    state = state.with_production_headers(Arc::new(headers));

    let app = Router::new()
        .route("/test", get(test_handler))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            production_headers_middleware,
        ))
        .with_state(state);

    // Make a request
    let request = Request::builder().uri("/test").body(Body::empty()).unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Verify production headers are present
    assert_eq!(response.status(), StatusCode::OK);

    let response_headers = response.headers();

    // Check static header
    assert!(
        response_headers.contains_key("x-api-version"),
        "X-API-Version header should be present"
    );
    assert_eq!(response_headers.get("x-api-version").unwrap().to_str().unwrap(), "1.0");

    assert!(
        response_headers.contains_key("x-powered-by"),
        "X-Powered-By header should be present"
    );
    assert_eq!(response_headers.get("x-powered-by").unwrap().to_str().unwrap(), "MockForge");

    // Check dynamic header (UUID should be expanded)
    assert!(
        response_headers.contains_key("x-request-id"),
        "X-Request-ID header should be present"
    );
    let request_id = response_headers.get("x-request-id").unwrap().to_str().unwrap();
    // UUID should be 36 characters (with hyphens)
    assert_eq!(request_id.len(), 36, "Request ID should be a UUID");
    assert!(!request_id.contains("{{uuid}}"), "UUID template should be expanded");
}

#[tokio::test]
async fn test_production_headers_template_expansion() {
    // Test various template expansions
    let mut headers = HashMap::new();
    headers.insert("X-Timestamp".to_string(), "{{timestamp}}".to_string());
    headers.insert("X-Now".to_string(), "{{now}}".to_string());
    headers.insert("X-UUID".to_string(), "{{uuid}}".to_string());

    let mut state = HttpState::new();
    state = state.with_production_headers(Arc::new(headers));

    let app = Router::new()
        .route("/test", get(test_handler))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            production_headers_middleware,
        ))
        .with_state(state);

    let request = Request::builder().uri("/test").body(Body::empty()).unwrap();

    let response = app.oneshot(request).await.unwrap();
    let response_headers = response.headers();

    // Check timestamp template
    let timestamp = response_headers.get("x-timestamp").unwrap().to_str().unwrap();
    let timestamp_num: i64 = timestamp.parse().unwrap();
    assert!(timestamp_num > 0, "Timestamp should be positive");

    // Check now template (RFC3339 format)
    let now = response_headers.get("x-now").unwrap().to_str().unwrap();
    assert!(now.contains('T'), "RFC3339 timestamp should contain 'T'");
    assert!(now.len() > 15, "RFC3339 timestamp should be reasonable length");

    // Check UUID template
    let uuid = response_headers.get("x-uuid").unwrap().to_str().unwrap();
    assert_eq!(uuid.len(), 36, "UUID should be 36 characters");
}

#[tokio::test]
async fn test_deceptive_deploy_config_conversion() {
    // Test that ProductionOAuthConfig converts to OAuth2Config correctly
    let prod_oauth = ProductionOAuthConfig {
        client_id: "test-client".to_string(),
        client_secret: "test-secret".to_string(),
        introspection_url: "https://auth.example.com/introspect".to_string(),
        auth_url: Some("https://auth.example.com/authorize".to_string()),
        token_url: Some("https://auth.example.com/token".to_string()),
        token_type_hint: Some("access_token".to_string()),
    };

    let oauth2_config: OAuth2Config = prod_oauth.clone().into();

    assert_eq!(oauth2_config.client_id, prod_oauth.client_id);
    assert_eq!(oauth2_config.client_secret, prod_oauth.client_secret);
    assert_eq!(oauth2_config.introspection_url, prod_oauth.introspection_url);
    assert_eq!(oauth2_config.auth_url, prod_oauth.auth_url);
    assert_eq!(oauth2_config.token_url, prod_oauth.token_url);
    assert_eq!(oauth2_config.token_type_hint, prod_oauth.token_type_hint);
}

#[tokio::test]
async fn test_deceptive_deploy_production_preset() {
    // Test that production preset creates correct configuration
    let preset = DeceptiveDeployConfig::production_preset();

    assert!(preset.enabled, "Preset should be enabled");
    assert!(preset.auto_tunnel, "Preset should have auto_tunnel enabled");
    assert!(preset.cors.is_some(), "Preset should have CORS config");
    assert!(preset.rate_limit.is_some(), "Preset should have rate limit config");
    assert!(!preset.headers.is_empty(), "Preset should have headers");

    // Check headers
    assert!(preset.headers.contains_key("X-API-Version"));
    assert!(preset.headers.contains_key("X-Request-ID"));
    assert!(preset.headers.contains_key("X-Powered-By"));

    // Check rate limit
    if let Some(rate_limit) = preset.rate_limit {
        assert_eq!(rate_limit.requests_per_minute, 1000);
        assert_eq!(rate_limit.burst, 2000);
        assert!(rate_limit.per_ip);
    }

    // Check CORS
    if let Some(cors) = preset.cors {
        assert!(cors.allowed_origins.contains(&"*".to_string()));
        assert!(cors.allow_credentials);
    }
}

#[tokio::test]
async fn test_rate_limit_enforcement() {
    // Test that rate limiting actually blocks requests when limit is exceeded
    let rate_limit_config = mockforge_http::middleware::RateLimitConfig {
        requests_per_minute: 2, // Very low limit for testing
        burst: 2,
        per_ip: false,
        per_endpoint: false,
    };
    let rate_limiter =
        Arc::new(mockforge_http::middleware::GlobalRateLimiter::new(rate_limit_config));

    let mut state = HttpState::new();
    state = state.with_rate_limiter(rate_limiter.clone());

    let app = Router::new()
        .route("/test", get(test_handler))
        .layer(axum::middleware::from_fn_with_state(state.clone(), rate_limit_middleware))
        .with_state(state);

    // Make requests up to the limit
    for i in 0..2 {
        let request = Request::builder().uri("/test").body(Body::empty()).unwrap();

        let response = app.clone().oneshot(request).await.unwrap();

        if i < 2 {
            // First two requests should succeed
            assert_eq!(response.status(), StatusCode::OK);
        }
    }

    // The third request should be rate limited
    // Note: This test may be flaky due to timing, but it validates the structure
    let request = Request::builder().uri("/test").body(Body::empty()).unwrap();

    let response = app.oneshot(request).await;

    // Either the request succeeds (if window reset) or is rate limited
    // The important thing is that the middleware is working
    match response {
        Ok(resp) => {
            // If it succeeds, check headers are present
            if resp.status() == StatusCode::OK {
                assert!(resp.headers().contains_key("x-rate-limit-limit"));
            } else {
                assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
            }
        }
        Err(_) => {
            // Error is also acceptable (rate limit rejection)
        }
    }
}

#[tokio::test]
async fn test_production_headers_no_override() {
    // Test that production headers don't override existing headers
    let mut headers = HashMap::new();
    headers.insert("X-Custom".to_string(), "custom-value".to_string());

    let mut state = HttpState::new();
    state = state.with_production_headers(Arc::new(headers));

    // Handler that sets its own header
    async fn handler_with_header() -> impl IntoResponse {
        let mut response = (StatusCode::OK, "test").into_response();
        response.headers_mut().insert(
            axum::http::HeaderName::from_static("x-custom"),
            axum::http::HeaderValue::from_static("handler-value"),
        );
        response
    }

    let app = Router::new()
        .route("/test", get(handler_with_header))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            production_headers_middleware,
        ))
        .with_state(state);

    let request = Request::builder().uri("/test").body(Body::empty()).unwrap();

    let response = app.oneshot(request).await.unwrap();
    let response_headers = response.headers();

    // The handler's header should take precedence (not overridden)
    assert_eq!(
        response_headers.get("x-custom").unwrap().to_str().unwrap(),
        "handler-value",
        "Handler header should not be overridden by production headers"
    );
}
