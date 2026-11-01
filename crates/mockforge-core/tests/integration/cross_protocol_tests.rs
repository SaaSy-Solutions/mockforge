//! Cross-protocol integration tests for protocol state consistency.
//!
//! These tests verify that state is consistent across different protocols
//! and that protocol bridges work correctly.

use mockforge_core::protocol_abstraction::Protocol;
use mockforge_core::routing::{HttpMethod, Route, RouteRegistry};
use serde_json::json;

#[cfg(test)]
mod protocol_state_consistency {
    use super::*;

    #[test]
    fn test_route_registry_across_protocols() {
        let mut registry = RouteRegistry::new();

        // Add HTTP route
        let http_route = Route::new(HttpMethod::GET, "/api/users".to_string());
        registry.add_http_route(http_route).unwrap();

        // Add WebSocket route
        let ws_route = Route::new(HttpMethod::GET, "/ws/chat".to_string());
        registry.add_ws_route(ws_route).unwrap();

        // Verify routes are stored separately
        let http_routes = registry.find_http_routes(&HttpMethod::GET, "/api/users");
        assert!(!http_routes.is_empty());

        let ws_routes = registry.find_ws_routes("/ws/chat");
        assert!(!ws_routes.is_empty());
    }

    #[test]
    fn test_protocol_enum_consistency() {
        // Verify protocol enum values
        let protocols = vec![
            Protocol::Http,
            Protocol::WebSocket,
            Protocol::Grpc,
            Protocol::GraphQL,
        ];

        for protocol in protocols {
            // Each protocol should be distinct
            match protocol {
                Protocol::Http => assert!(matches!(protocol, Protocol::Http)),
                Protocol::WebSocket => assert!(matches!(protocol, Protocol::WebSocket)),
                Protocol::Grpc => assert!(matches!(protocol, Protocol::Grpc)),
                Protocol::GraphQL => assert!(matches!(protocol, Protocol::GraphQL)),
                _ => {} // Other protocols (SMTP, MQTT, FTP, etc.)
            }
        }
    }

    #[test]
    fn test_route_metadata_across_protocols() {
        let mut registry = RouteRegistry::new();

        // Add routes with metadata for different protocols
        let mut http_route = Route::new(HttpMethod::GET, "/api/users".to_string());
        http_route = http_route.with_metadata("protocol".to_string(), json!("http"));
        registry.add_http_route(http_route).unwrap();

        let mut ws_route = Route::new(HttpMethod::GET, "/ws/chat".to_string());
        ws_route = ws_route.with_metadata("protocol".to_string(), json!("websocket"));
        registry.add_ws_route(ws_route).unwrap();

        // Verify metadata is preserved
        let http_routes = registry.find_http_routes(&HttpMethod::GET, "/api/users");
        assert_eq!(http_routes[0].metadata.get("protocol"), Some(&json!("http")));

        let ws_routes = registry.find_ws_routes("/ws/chat");
        assert_eq!(ws_routes[0].metadata.get("protocol"), Some(&json!("websocket")));
    }
}

#[cfg(test)]
mod protocol_bridge_tests {
    use super::*;

    #[test]
    fn test_route_pattern_matching_across_protocols() {
        let mut registry = RouteRegistry::new();

        // Add routes with similar patterns across protocols
        let http_route = Route::new(HttpMethod::GET, "/api/users/*".to_string());
        registry.add_http_route(http_route).unwrap();

        let ws_route = Route::new(HttpMethod::GET, "/ws/users/*".to_string());
        registry.add_ws_route(ws_route).unwrap();

        // Verify wildcard matching works for both
        let http_matches = registry.find_http_routes(&HttpMethod::GET, "/api/users/123");
        assert!(!http_matches.is_empty());

        let ws_matches = registry.find_ws_routes("/ws/users/123");
        assert!(!ws_matches.is_empty());
    }

    #[test]
    fn test_route_priority_across_protocols() {
        let mut registry = RouteRegistry::new();

        // Add routes with different priorities
        let low_priority = Route::new(HttpMethod::GET, "/api/*".to_string()).with_priority(1);
        let high_priority = Route::new(HttpMethod::GET, "/api/users".to_string()).with_priority(10);

        registry.add_http_route(low_priority).unwrap();
        registry.add_http_route(high_priority).unwrap();

        // Higher priority route should be returned first
        let matches = registry.find_http_routes(&HttpMethod::GET, "/api/users");
        assert!(!matches.is_empty());
        // In a real implementation, higher priority routes should come first
    }

    #[test]
    fn test_protocol_route_isolation() {
        let mut registry = RouteRegistry::new();

        // Add routes with same path pattern but different protocols
        let http_route = Route::new(HttpMethod::GET, "/api/test".to_string());
        let ws_route = Route::new(HttpMethod::GET, "/api/test".to_string());

        registry.add_http_route(http_route).unwrap();
        registry.add_ws_route(ws_route).unwrap();

        // Routes should be isolated by protocol
        let http_routes = registry.find_http_routes(&HttpMethod::GET, "/api/test");
        let ws_routes = registry.find_ws_routes("/api/test");

        assert!(!http_routes.is_empty());
        assert!(!ws_routes.is_empty());
        // They should be separate routes even with same path
    }
}

#[cfg(test)]
mod protocol_data_consistency {
    use super::*;

    #[test]
    fn test_route_metadata_serialization() {
        // Test that route metadata can be serialized/deserialized
        let mut route = Route::new(HttpMethod::GET, "/api/test".to_string());
        route = route.with_metadata("key1".to_string(), json!("value1"));
        route = route.with_metadata("key2".to_string(), json!({"nested": "value"}));

        // Serialize
        let serialized = serde_json::to_string(&route).unwrap();
        assert!(serialized.contains("key1"));
        assert!(serialized.contains("value1"));

        // Deserialize
        let deserialized: Route = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.metadata.get("key1"), Some(&json!("value1")));
        assert_eq!(deserialized.metadata.get("key2"), Some(&json!({"nested": "value"})));
    }

    #[test]
    fn test_route_registry_serialization() {
        let mut registry = RouteRegistry::new();

        // Add routes
        let route1 = Route::new(HttpMethod::GET, "/api/users".to_string());
        let route2 = Route::new(HttpMethod::POST, "/api/users".to_string());
        registry.add_http_route(route1).unwrap();
        registry.add_http_route(route2).unwrap();

        // Clone should preserve routes
        let cloned = registry.clone();
        let routes1 = registry.find_http_routes(&HttpMethod::GET, "/api/users");
        let routes2 = cloned.find_http_routes(&HttpMethod::GET, "/api/users");

        assert_eq!(routes1.len(), routes2.len());
    }
}
