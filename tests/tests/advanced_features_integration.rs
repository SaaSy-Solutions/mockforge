//! Integration tests for advanced MockForge features:
//! - Record & Playback (stub mapping conversion)
//! - Stateful behavior simulation
//! - Per-route fault injection and latency
//! - Conditional proxying

use axum::http::{HeaderMap, Method, StatusCode, Uri};
use mockforge_core::{
    conditions::ConditionContext,
    config::{RouteConfig, RouteFaultInjectionConfig, RouteFaultType, RouteLatencyConfig},
    proxy::{
        conditional::{evaluate_proxy_condition, find_matching_rule},
        config::{ProxyConfig, ProxyRule},
    },
    route_chaos::{RouteChaosInjector, RouteMatcher},
    stateful_handler::{
        ResourceIdExtract, StateResponse, StatefulConfig, StatefulResponseHandler,
        TransitionTrigger,
    },
    Result,
};
use mockforge_recorder::{
    models::{Protocol, RecordedExchange, RecordedRequest, RecordedResponse},
    StubFormat, StubMappingConverter,
};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::Instant;

#[tokio::test]
async fn test_stub_mapping_conversion() {
    // Create a recorded exchange
    let recorded_request = RecordedRequest {
        method: "POST".to_string(),
        path: "/api/users".to_string(),
        headers: HashMap::from([("content-type".to_string(), "application/json".to_string())]),
        body: Some(r#"{"name": "John Doe", "email": "john@example.com"}"#.to_string()),
        query_params: HashMap::new(),
        timestamp: chrono::Utc::now(),
    };

    let recorded_response = RecordedResponse {
        status_code: 201,
        headers: HashMap::from([("content-type".to_string(), "application/json".to_string())]),
        body: r#"{"id": "123e4567-e89b-12d3-a456-426614174000", "name": "John Doe", "email": "john@example.com", "created_at": "2024-01-01T00:00:00Z"}"#.to_string(),
        timestamp: chrono::Utc::now(),
    };

    let exchange = RecordedExchange {
        id: "test-123".to_string(),
        protocol: Protocol::Http,
        request: recorded_request,
        response: recorded_response,
        duration_ms: 50,
    };

    // Convert to stub mapping
    let converter = StubMappingConverter::new();
    let stub = converter
        .convert_to_stub(&exchange, StubFormat::Yaml)
        .expect("Should convert to stub");

    // Verify stub structure
    assert!(stub.contains("request"));
    assert!(stub.contains("response"));
    assert!(stub.contains("POST"));
    assert!(stub.contains("/api/users"));

    // Check that dynamic values are templated
    assert!(stub.contains("{{uuid}}") || stub.contains("{{timestamp}}"));
}

#[tokio::test]
async fn test_stateful_response_handler() {
    // Create stateful config
    let mut state_responses = HashMap::new();
    state_responses.insert(
        "pending".to_string(),
        StateResponse {
            status_code: 200,
            headers: HashMap::new(),
            body_template: r#"{"status": "pending", "order_id": "{{resource_id}}"}"#.to_string(),
            content_type: "application/json".to_string(),
        },
    );
    state_responses.insert(
        "processing".to_string(),
        StateResponse {
            status_code: 200,
            headers: HashMap::new(),
            body_template: r#"{"status": "processing", "order_id": "{{resource_id}}"}"#.to_string(),
            content_type: "application/json".to_string(),
        },
    );

    let config = StatefulConfig {
        resource_id_extract: ResourceIdExtract::PathParam {
            param: "order_id".to_string(),
        },
        resource_type: "order".to_string(),
        state_responses,
        transitions: vec![TransitionTrigger {
            method: Method::POST,
            path_pattern: "/api/orders".to_string(),
            from_state: "initial".to_string(),
            to_state: "pending".to_string(),
            condition: None,
        }],
    };

    let handler = StatefulResponseHandler::new().expect("Should create handler");
    handler.add_config("/api/orders".to_string(), config).await;

    // Test initial request (creates order)
    let method = Method::POST;
    let uri = Uri::from_static("/api/orders");
    let headers = HeaderMap::new();
    let body = Some(r#"{"product": "widget"}"#.as_bytes());

    let response = handler
        .process_request(&method, &uri, &headers, body)
        .await
        .expect("Should process request");

    assert!(response.is_some());
    let response = response.unwrap();
    assert_eq!(response.status_code, 200);
    assert!(response.body.contains("pending"));
}

#[tokio::test]
async fn test_per_route_fault_injection() {
    // Create route with fault injection
    let route = RouteConfig {
        path: "/api/test".to_string(),
        method: "GET".to_string(),
        request: None,
        response: mockforge_core::config::RouteResponseConfig {
            status: 200,
            headers: HashMap::new(),
            body: None,
        },
        fault_injection: Some(RouteFaultInjectionConfig {
            enabled: true,
            probability: 1.0, // Always inject for testing
            fault_types: vec![RouteFaultType::HttpError {
                status_code: 500,
                message: Some("Test error".to_string()),
            }],
        }),
        latency: None,
    };

    let injector = RouteChaosInjector::new(vec![route]).expect("Should create injector");

    let method = Method::GET;
    let uri = Uri::from_static("/api/test");

    // Should inject fault
    let fault_response = injector.get_fault_response(&method, &uri);
    assert!(fault_response.is_some());
    let fault = fault_response.unwrap();
    assert_eq!(fault.status_code, 500);
    assert_eq!(fault.error_message, "Test error");
}

#[tokio::test]
async fn test_per_route_latency() {
    // Create route with latency
    let route = RouteConfig {
        path: "/api/slow".to_string(),
        method: "GET".to_string(),
        request: None,
        response: mockforge_core::config::RouteResponseConfig {
            status: 200,
            headers: HashMap::new(),
            body: None,
        },
        fault_injection: None,
        latency: Some(RouteLatencyConfig {
            enabled: true,
            probability: 1.0,
            fixed_delay_ms: Some(100),
            random_delay_range_ms: None,
            jitter_percent: 0.0,
            distribution: mockforge_core::config::LatencyDistribution::Fixed,
        }),
    };

    let injector = RouteChaosInjector::new(vec![route]).expect("Should create injector");

    let method = Method::GET;
    let uri = Uri::from_static("/api/slow");

    // Measure latency injection
    let start = Instant::now();
    injector.inject_latency(&method, &uri).await.expect("Should inject latency");
    let elapsed = start.elapsed();

    // Should have delayed at least 100ms
    assert!(elapsed >= Duration::from_millis(100));
}

#[tokio::test]
async fn test_conditional_proxying_jsonpath() {
    // Create proxy rule with JSONPath condition
    let rule = ProxyRule {
        path_pattern: "/api/users".to_string(),
        target_url: "http://example.com".to_string(),
        enabled: true,
        pattern: "/api/users".to_string(),
        upstream_url: "http://example.com".to_string(),
        migration_mode: mockforge_core::proxy::config::MigrationMode::Auto,
        migration_group: None,
        condition: Some("$.user.role == 'admin'".to_string()),
    };

    let method = Method::POST;
    let uri = Uri::from_static("/api/users");
    let mut headers = HeaderMap::new();
    headers.insert("content-type", "application/json".parse().unwrap());

    // Test with admin role
    let body_admin = json!({
        "user": {
            "role": "admin"
        }
    });
    let body_bytes = serde_json::to_string(&body_admin).unwrap().into_bytes();

    let result = evaluate_proxy_condition(&rule, &method, &uri, &headers, Some(&body_bytes))
        .expect("Should evaluate condition");
    assert!(result); // Should proxy admin requests

    // Test with non-admin role
    let body_user = json!({
        "user": {
            "role": "user"
        }
    });
    let body_bytes = serde_json::to_string(&body_user).unwrap().into_bytes();

    let result = evaluate_proxy_condition(&rule, &method, &uri, &headers, Some(&body_bytes))
        .expect("Should evaluate condition");
    assert!(!result); // Should not proxy non-admin requests
}

#[tokio::test]
async fn test_conditional_proxying_header() {
    // Create proxy rule with header condition
    let rule = ProxyRule {
        path_pattern: "/api/protected/*".to_string(),
        target_url: "http://example.com".to_string(),
        enabled: true,
        pattern: "/api/protected/*".to_string(),
        upstream_url: "http://example.com".to_string(),
        migration_mode: mockforge_core::proxy::config::MigrationMode::Auto,
        migration_group: None,
        condition: Some("header[authorization] != ''".to_string()),
    };

    let method = Method::GET;
    let uri = Uri::from_static("/api/protected/resource");

    // Test with authorization header
    let mut headers = HeaderMap::new();
    headers.insert("authorization", "Bearer token123".parse().unwrap());

    let result = evaluate_proxy_condition(&rule, &method, &uri, &headers, None)
        .expect("Should evaluate condition");
    assert!(result); // Should proxy authenticated requests

    // Test without authorization header
    let headers = HeaderMap::new();

    let result = evaluate_proxy_condition(&rule, &method, &uri, &headers, None)
        .expect("Should evaluate condition");
    assert!(!result); // Should not proxy unauthenticated requests
}

#[tokio::test]
async fn test_conditional_proxying_query_param() {
    // Create proxy rule with query parameter condition
    let rule = ProxyRule {
        path_pattern: "/api/data/*".to_string(),
        target_url: "http://example.com".to_string(),
        enabled: true,
        pattern: "/api/data/*".to_string(),
        upstream_url: "http://example.com".to_string(),
        migration_mode: mockforge_core::proxy::config::MigrationMode::Auto,
        migration_group: None,
        condition: Some("query[env] == 'production'".to_string()),
    };

    let method = Method::GET;

    // Test with production env
    let uri = Uri::from_static("/api/data/resource?env=production");
    let headers = HeaderMap::new();

    let result = evaluate_proxy_condition(&rule, &method, &uri, &headers, None)
        .expect("Should evaluate condition");
    assert!(result); // Should proxy production requests

    // Test with staging env
    let uri = Uri::from_static("/api/data/resource?env=staging");
    let headers = HeaderMap::new();

    let result = evaluate_proxy_condition(&rule, &method, &uri, &headers, None)
        .expect("Should evaluate condition");
    assert!(!result); // Should not proxy staging requests
}

#[tokio::test]
async fn test_find_matching_rule_with_condition() {
    let rules = vec![
        ProxyRule {
            path_pattern: "/api/users/*".to_string(),
            target_url: "http://users.example.com".to_string(),
            enabled: true,
            pattern: "/api/users/*".to_string(),
            upstream_url: "http://users.example.com".to_string(),
            migration_mode: mockforge_core::proxy::config::MigrationMode::Auto,
            migration_group: None,
            condition: Some("header[authorization] != ''".to_string()),
        },
        ProxyRule {
            path_pattern: "/api/public/*".to_string(),
            target_url: "http://public.example.com".to_string(),
            enabled: true,
            pattern: "/api/public/*".to_string(),
            upstream_url: "http://public.example.com".to_string(),
            migration_mode: mockforge_core::proxy::config::MigrationMode::Auto,
            migration_group: None,
            condition: None, // No condition - always matches
        },
    ];

    let method = Method::GET;
    let uri = Uri::from_static("/api/users/123");
    let mut headers = HeaderMap::new();
    headers.insert("authorization", "Bearer token".parse().unwrap());

    // Should find matching rule with condition
    let matching_rule =
        find_matching_rule(&rules, &method, &uri, &headers, None, |pattern, path| {
            // Simple path matching
            path.starts_with(pattern.trim_end_matches("/*"))
        });

    assert!(matching_rule.is_some());
    assert_eq!(matching_rule.unwrap().path_pattern, "/api/users/*");
}

#[tokio::test]
async fn test_route_matcher() {
    let routes = vec![
        RouteConfig {
            path: "/api/users/{id}".to_string(),
            method: "GET".to_string(),
            request: None,
            response: mockforge_core::config::RouteResponseConfig {
                status: 200,
                headers: HashMap::new(),
                body: None,
            },
            fault_injection: None,
            latency: None,
        },
        RouteConfig {
            path: "/api/orders/{order_id}".to_string(),
            method: "POST".to_string(),
            request: None,
            response: mockforge_core::config::RouteResponseConfig {
                status: 200,
                headers: HashMap::new(),
                body: None,
            },
            fault_injection: None,
            latency: None,
        },
    ];

    let matcher = RouteMatcher::new(routes).expect("Should create matcher");

    // Test matching
    let method = Method::GET;
    let uri = Uri::from_static("/api/users/123");
    assert!(matcher.match_route(&method, &uri).is_some());

    let method = Method::POST;
    let uri = Uri::from_static("/api/orders/456");
    assert!(matcher.match_route(&method, &uri).is_some());

    // Test non-matching
    let method = Method::GET;
    let uri = Uri::from_static("/api/products/789");
    assert!(matcher.match_route(&method, &uri).is_none());
}
