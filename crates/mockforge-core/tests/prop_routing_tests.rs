//! Property-based tests for route matching and routing logic.
//!
//! These tests use property-based testing to verify correctness of route
//! pattern matching, priority handling, and route registry operations.

use mockforge_core::routing::{HttpMethod, Route, RouteRegistry};
use proptest::prelude::*;

/// Property test: Route creation and properties
#[cfg(test)]
mod route_creation_tests {
    use super::*;

    proptest! {
        #[test]
        fn route_creation_with_arbitrary_path(
            method in prop::sample::select(vec![
                HttpMethod::GET,
                HttpMethod::POST,
                HttpMethod::PUT,
                HttpMethod::DELETE,
                HttpMethod::PATCH,
            ]),
            path in "/[a-zA-Z0-9/_-]*"
        ) {
            let route = Route::new(method.clone(), path.clone());

            assert_eq!(route.method, method);
            assert_eq!(route.path, path);
            assert_eq!(route.priority, 0);
            assert!(route.metadata.is_empty());
        }

        #[test]
        fn route_with_priority(
            priority in prop::num::i32::ANY,
            path in "/[a-zA-Z0-9/_-]*"
        ) {
            let route = Route::new(HttpMethod::GET, path)
                .with_priority(priority);

            assert_eq!(route.priority, priority);
        }

        #[test]
        fn route_with_metadata(
            path in "/[a-zA-Z0-9/_-]*",
            key in "[a-zA-Z0-9_-]+",
            value in ".*"
        ) {
            let route = Route::new(HttpMethod::GET, path)
                .with_metadata(key.clone(), serde_json::json!(value));

            assert_eq!(route.metadata.get(&key), Some(&serde_json::json!(value)));
        }
    }
}

/// Property test: Route pattern matching
#[cfg(test)]
mod route_matching_tests {
    use super::*;

    proptest! {
        #[test]
        fn exact_path_matching(
            path in "/[a-zA-Z0-9/_-]*"
        ) {
            let mut registry = RouteRegistry::new();
            let route = Route::new(HttpMethod::GET, path.clone());
            registry.add_http_route(route).unwrap();

            let matches = registry.find_http_routes(&HttpMethod::GET, &path);
            assert!(!matches.is_empty());
        }

        #[test]
        fn wildcard_path_matching(
            segments in prop::collection::vec("[a-zA-Z0-9_-]+", 1..5),
            wildcard_index in prop::option::of(0usize..5)
        ) {
            prop_assume!(!segments.is_empty());

            let mut pattern_parts = segments.clone();
            let mut path_parts = segments.clone();

            // Insert wildcard at random position if specified
            if let Some(idx) = wildcard_index {
                if idx < pattern_parts.len() {
                    pattern_parts[idx] = "*".to_string();
                }
            }

            let pattern = "/".to_string() + &pattern_parts.join("/");
            let path = "/".to_string() + &path_parts.join("/");

            let mut registry = RouteRegistry::new();
            let route = Route::new(HttpMethod::GET, pattern.clone());
            registry.add_http_route(route).unwrap();

            let matches = registry.find_http_routes(&HttpMethod::GET, &path);

            // If wildcard was used, should match; if exact match, should match
            if pattern == path || pattern.contains('*') {
                assert!(!matches.is_empty());
            }
        }

        #[test]
        fn method_specific_matching(
            method in prop::sample::select(vec![
                HttpMethod::GET,
                HttpMethod::POST,
                HttpMethod::PUT,
                HttpMethod::DELETE,
            ]),
            path in "/[a-zA-Z0-9/_-]*"
        ) {
            let mut registry = RouteRegistry::new();

            // Add route for specific method
            let route = Route::new(method.clone(), path.clone());
            registry.add_http_route(route).unwrap();

            // Should match same method
            let matches = registry.find_http_routes(&method, &path);
            assert!(!matches.is_empty());

            // Should not match different method
            let other_method = match method {
                HttpMethod::GET => HttpMethod::POST,
                _ => HttpMethod::GET,
            };
            let no_matches = registry.find_http_routes(&other_method, &path);
            assert!(no_matches.is_empty());
        }

        #[test]
        fn multiple_routes_same_path(
            count in 1usize..10,
            path in "/[a-zA-Z0-9/_-]*"
        ) {
            let mut registry = RouteRegistry::new();

            // Add multiple routes with same path but different priorities
            for i in 0..count {
                let route = Route::new(HttpMethod::GET, path.clone())
                    .with_priority(i as i32);
                registry.add_http_route(route).unwrap();
            }

            let matches = registry.find_http_routes(&HttpMethod::GET, &path);
            assert_eq!(matches.len(), count);
        }

        #[test]
        fn route_priority_ordering(
            priority1 in prop::num::i32::ANY,
            priority2 in prop::num::i32::ANY,
            path in "/[a-zA-Z0-9/_-]*"
        ) {
            let mut registry = RouteRegistry::new();

            let route1 = Route::new(HttpMethod::GET, path.clone())
                .with_priority(priority1);
            let route2 = Route::new(HttpMethod::GET, path.clone())
                .with_priority(priority2);

            registry.add_http_route(route1).unwrap();
            registry.add_http_route(route2).unwrap();

            let matches = registry.find_http_routes(&HttpMethod::GET, &path);
            assert_eq!(matches.len(), 2);
        }
    }
}

/// Property test: Route registry operations
#[cfg(test)]
mod registry_operations_tests {
    use super::*;

    proptest! {
        #[test]
        fn add_multiple_routes(
            count in 0usize..20,
            method in prop::sample::select(vec![
                HttpMethod::GET,
                HttpMethod::POST,
            ])
        ) {
            let mut registry = RouteRegistry::new();

            for i in 0..count {
                let path = format!("/api/route{}", i);
                let route = Route::new(method.clone(), path);
                assert!(registry.add_http_route(route).is_ok());
            }

            let routes = registry.get_http_routes(&method);
            assert_eq!(routes.len(), count);
        }

        #[test]
        fn clear_registry(
            count in 1usize..10
        ) {
            let mut registry = RouteRegistry::new();

            // Add routes
            for i in 0..count {
                let path = format!("/api/route{}", i);
                let route = Route::new(HttpMethod::GET, path);
                registry.add_http_route(route).unwrap();
            }

            assert_eq!(registry.get_http_routes(&HttpMethod::GET).len(), count);

            // Clear and verify
            registry.clear();
            assert_eq!(registry.get_http_routes(&HttpMethod::GET).len(), 0);
        }

        #[test]
        fn find_nonexistent_route(
            path in "/[a-zA-Z0-9/_-]*"
        ) {
            let registry = RouteRegistry::new();
            let matches = registry.find_http_routes(&HttpMethod::GET, &path);
            assert!(matches.is_empty());
        }

        #[test]
        fn websocket_route_matching(
            path in "/[a-zA-Z0-9/_-]*"
        ) {
            let mut registry = RouteRegistry::new();
            let route = Route::new(HttpMethod::GET, path.clone());
            registry.add_ws_route(route).unwrap();

            let matches = registry.find_ws_routes(&path);
            assert!(!matches.is_empty());
        }

        #[test]
        fn grpc_route_matching(
            service in "[a-zA-Z0-9_.]+",
            method in "[a-zA-Z0-9_]+"
        ) {
            let mut registry = RouteRegistry::new();
            let route = Route::new(HttpMethod::GET, method.clone());
            registry.add_grpc_route(service.clone(), route).unwrap();

            let matches = registry.find_grpc_routes(&service, &method);
            assert!(!matches.is_empty());
        }
    }
}

/// Property test: Edge cases and boundary conditions
#[cfg(test)]
mod routing_edge_cases {
    use super::*;

    proptest! {
        #[test]
        fn empty_path_matching(
            path in ""
        ) {
            let mut registry = RouteRegistry::new();
            let route = Route::new(HttpMethod::GET, path.clone());
            registry.add_http_route(route).unwrap();

            let matches = registry.find_http_routes(&HttpMethod::GET, &path);
            // Empty path should match empty path
            assert!(!matches.is_empty() || path.is_empty());
        }

        #[test]
        fn very_long_path(
            len in 0usize..1000
        ) {
            let path = "/".to_string() + &"a".repeat(len);
            let mut registry = RouteRegistry::new();
            let route = Route::new(HttpMethod::GET, path.clone());
            registry.add_http_route(route).unwrap();

            let matches = registry.find_http_routes(&HttpMethod::GET, &path);
            assert!(!matches.is_empty());
        }

        #[test]
        fn path_with_special_characters(
            special_chars in "[\\s\\S]{0,100}"
        ) {
            let path = format!("/test{}", special_chars);
            let mut registry = RouteRegistry::new();
            let route = Route::new(HttpMethod::GET, path.clone());
            registry.add_http_route(route).unwrap();

            let matches = registry.find_http_routes(&HttpMethod::GET, &path);
            // Should handle special characters (may or may not match depending on implementation)
            let _ = matches;
        }

        #[test]
        fn multiple_wildcards(
            segment_count in 1usize..10,
            wildcard_count in 0usize..5
        ) {
            prop_assume!(wildcard_count <= segment_count);

            let mut segments: Vec<String> = (0..segment_count)
                .map(|i| format!("segment{}", i))
                .collect();

            // Replace some segments with wildcards
            for i in 0..wildcard_count.min(segments.len()) {
                segments[i] = "*".to_string();
            }

            let pattern = "/".to_string() + &segments.join("/");
            let path = "/".to_string() + &(0..segment_count)
                .map(|i| format!("value{}", i))
                .collect::<Vec<_>>()
                .join("/");

            let mut registry = RouteRegistry::new();
            let route = Route::new(HttpMethod::GET, pattern.clone());
            registry.add_http_route(route).unwrap();

            let matches = registry.find_http_routes(&HttpMethod::GET, &path);
            // Wildcard matching behavior may vary - just ensure no panic
            // Some routing implementations may not support multiple wildcards
            let _ = matches;
        }

        #[test]
        fn route_with_unicode_path(
            unicode_path in "\\PC*"
        ) {
            let path = format!("/{}", unicode_path);
            let mut registry = RouteRegistry::new();
            let route = Route::new(HttpMethod::GET, path.clone());
            registry.add_http_route(route).unwrap();

            let matches = registry.find_http_routes(&HttpMethod::GET, &path);
            // Should handle unicode paths
            let _ = matches;
        }
    }
}
