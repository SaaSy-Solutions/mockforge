//! Edge case tests for routing module
//!
//! These tests cover error paths, edge cases, and boundary conditions
//! for route registration and matching.

use mockforge_core::routing::{HttpMethod, Route, RouteRegistry};
use serde_json::json;

/// Test Route with all HTTP methods
#[test]
fn test_route_all_methods() {
    let methods = vec![
        HttpMethod::GET,
        HttpMethod::POST,
        HttpMethod::PUT,
        HttpMethod::DELETE,
        HttpMethod::PATCH,
        HttpMethod::HEAD,
        HttpMethod::OPTIONS,
    ];
    
    for method in methods {
        let route = Route::new(method.clone(), "/api/test".to_string());
        assert_eq!(route.method, method);
        assert_eq!(route.path, "/api/test");
    }
}

/// Test Route with empty path
#[test]
fn test_route_empty_path() {
    let route = Route::new(HttpMethod::GET, "".to_string());
    assert_eq!(route.path, "");
}

/// Test Route with root path
#[test]
fn test_route_root_path() {
    let route = Route::new(HttpMethod::GET, "/".to_string());
    assert_eq!(route.path, "/");
}

/// Test Route with negative priority
#[test]
fn test_route_negative_priority() {
    let route = Route::new(HttpMethod::GET, "/api/test".to_string())
        .with_priority(-10);
    assert_eq!(route.priority, -10);
}

/// Test Route with large priority
#[test]
fn test_route_large_priority() {
    let route = Route::new(HttpMethod::GET, "/api/test".to_string())
        .with_priority(1000);
    assert_eq!(route.priority, 1000);
}

/// Test Route with multiple metadata entries
#[test]
fn test_route_multiple_metadata() {
    let route = Route::new(HttpMethod::GET, "/api/test".to_string())
        .with_metadata("key1".to_string(), json!("value1"))
        .with_metadata("key2".to_string(), json!(42))
        .with_metadata("key3".to_string(), json!(true))
        .with_metadata("key4".to_string(), json!({"nested": "object"}));
    
    assert_eq!(route.metadata.len(), 4);
    assert_eq!(route.metadata.get("key1"), Some(&json!("value1")));
    assert_eq!(route.metadata.get("key2"), Some(&json!(42)));
    assert_eq!(route.metadata.get("key3"), Some(&json!(true)));
    assert_eq!(route.metadata.get("key4"), Some(&json!({"nested": "object"})));
}

/// Test Route with overwritten metadata
#[test]
fn test_route_overwrite_metadata() {
    let route = Route::new(HttpMethod::GET, "/api/test".to_string())
        .with_metadata("key".to_string(), json!("value1"))
        .with_metadata("key".to_string(), json!("value2"));
    
    assert_eq!(route.metadata.len(), 1);
    assert_eq!(route.metadata.get("key"), Some(&json!("value2")));
}

/// Test RouteRegistry clear
#[test]
fn test_route_registry_clear() {
    let mut registry = RouteRegistry::new();
    
    registry.add_http_route(Route::new(HttpMethod::GET, "/api/users".to_string())).unwrap();
    registry.add_ws_route(Route::new(HttpMethod::GET, "/ws".to_string())).unwrap();
    registry.add_grpc_route("Service".to_string(), Route::new(HttpMethod::POST, "Method".to_string())).unwrap();
    
    assert!(!registry.get_http_routes(&HttpMethod::GET).is_empty());
    assert!(!registry.get_ws_routes().is_empty());
    assert!(!registry.get_grpc_routes("Service").is_empty());
    
    registry.clear();
    
    assert!(registry.get_http_routes(&HttpMethod::GET).is_empty());
    assert!(registry.get_ws_routes().is_empty());
    assert!(registry.get_grpc_routes("Service").is_empty());
}

/// Test RouteRegistry add_route alias
#[test]
fn test_route_registry_add_route_alias() {
    let mut registry = RouteRegistry::new();
    let route = Route::new(HttpMethod::GET, "/api/test".to_string());
    
    assert!(registry.add_route(route).is_ok());
    assert_eq!(registry.get_http_routes(&HttpMethod::GET).len(), 1);
}

/// Test RouteRegistry with multiple routes same method
#[test]
fn test_route_registry_multiple_same_method() {
    let mut registry = RouteRegistry::new();
    
    registry.add_http_route(Route::new(HttpMethod::GET, "/api/users".to_string())).unwrap();
    registry.add_http_route(Route::new(HttpMethod::GET, "/api/posts".to_string())).unwrap();
    registry.add_http_route(Route::new(HttpMethod::GET, "/api/comments".to_string())).unwrap();
    
    assert_eq!(registry.get_http_routes(&HttpMethod::GET).len(), 3);
}

/// Test RouteRegistry with multiple routes different methods
#[test]
fn test_route_registry_multiple_different_methods() {
    let mut registry = RouteRegistry::new();
    
    registry.add_http_route(Route::new(HttpMethod::GET, "/api/users".to_string())).unwrap();
    registry.add_http_route(Route::new(HttpMethod::POST, "/api/users".to_string())).unwrap();
    registry.add_http_route(Route::new(HttpMethod::PUT, "/api/users".to_string())).unwrap();
    registry.add_http_route(Route::new(HttpMethod::DELETE, "/api/users".to_string())).unwrap();
    
    assert_eq!(registry.get_http_routes(&HttpMethod::GET).len(), 1);
    assert_eq!(registry.get_http_routes(&HttpMethod::POST).len(), 1);
    assert_eq!(registry.get_http_routes(&HttpMethod::PUT).len(), 1);
    assert_eq!(registry.get_http_routes(&HttpMethod::DELETE).len(), 1);
}

/// Test RouteRegistry find_http_routes with no matches
#[test]
fn test_route_registry_find_http_routes_no_match() {
    let mut registry = RouteRegistry::new();
    registry.add_http_route(Route::new(HttpMethod::GET, "/api/users".to_string())).unwrap();
    
    let found = registry.find_http_routes(&HttpMethod::GET, "/api/nonexistent");
    assert!(found.is_empty());
}

/// Test RouteRegistry find_http_routes with wrong method
#[test]
fn test_route_registry_find_http_routes_wrong_method() {
    let mut registry = RouteRegistry::new();
    registry.add_http_route(Route::new(HttpMethod::GET, "/api/users".to_string())).unwrap();
    
    let found = registry.find_http_routes(&HttpMethod::POST, "/api/users");
    assert!(found.is_empty());
}

/// Test RouteRegistry find_http_routes with wildcard
#[test]
fn test_route_registry_find_http_routes_wildcard() {
    let mut registry = RouteRegistry::new();
    registry.add_http_route(Route::new(HttpMethod::GET, "/api/*".to_string())).unwrap();
    
    let found = registry.find_http_routes(&HttpMethod::GET, "/api/users");
    assert_eq!(found.len(), 1);
    
    let found2 = registry.find_http_routes(&HttpMethod::GET, "/api/posts");
    assert_eq!(found2.len(), 1);
}

/// Test RouteRegistry find_http_routes with multiple wildcard matches
#[test]
fn test_route_registry_find_http_routes_multiple_wildcards() {
    let mut registry = RouteRegistry::new();
    registry.add_http_route(Route::new(HttpMethod::GET, "/api/*".to_string())).unwrap();
    registry.add_http_route(Route::new(HttpMethod::GET, "/api/users/*".to_string())).unwrap();
    
    // /api/users matches /api/* (2 segments each)
    let found = registry.find_http_routes(&HttpMethod::GET, "/api/users");
    assert_eq!(found.len(), 1); // Only /api/* matches (same segment count)
    
    // /api/users/123 matches /api/users/* (3 segments each)
    let found2 = registry.find_http_routes(&HttpMethod::GET, "/api/users/123");
    assert_eq!(found2.len(), 1); // Only /api/users/* matches
}

/// Test RouteRegistry find_ws_routes
#[test]
fn test_route_registry_find_ws_routes() {
    let mut registry = RouteRegistry::new();
    registry.add_ws_route(Route::new(HttpMethod::GET, "/ws/chat".to_string())).unwrap();
    registry.add_ws_route(Route::new(HttpMethod::GET, "/ws/notifications".to_string())).unwrap();
    
    let found = registry.find_ws_routes("/ws/chat");
    assert_eq!(found.len(), 1);
    
    let found2 = registry.find_ws_routes("/ws/notifications");
    assert_eq!(found2.len(), 1);
    
    let found3 = registry.find_ws_routes("/ws/nonexistent");
    assert!(found3.is_empty());
}

/// Test RouteRegistry find_ws_routes with wildcard
#[test]
fn test_route_registry_find_ws_routes_wildcard() {
    let mut registry = RouteRegistry::new();
    registry.add_ws_route(Route::new(HttpMethod::GET, "/ws/*".to_string())).unwrap();
    
    let found = registry.find_ws_routes("/ws/chat");
    assert_eq!(found.len(), 1);
    
    let found2 = registry.find_ws_routes("/ws/notifications");
    assert_eq!(found2.len(), 1);
}

/// Test RouteRegistry find_grpc_routes
#[test]
fn test_route_registry_find_grpc_routes() {
    let mut registry = RouteRegistry::new();
    registry.add_grpc_route("UserService".to_string(), Route::new(HttpMethod::POST, "GetUser".to_string())).unwrap();
    registry.add_grpc_route("UserService".to_string(), Route::new(HttpMethod::POST, "CreateUser".to_string())).unwrap();
    registry.add_grpc_route("OrderService".to_string(), Route::new(HttpMethod::POST, "CreateOrder".to_string())).unwrap();
    
    let found = registry.find_grpc_routes("UserService", "GetUser");
    assert_eq!(found.len(), 1);
    
    let found2 = registry.find_grpc_routes("UserService", "CreateUser");
    assert_eq!(found2.len(), 1);
    
    let found3 = registry.find_grpc_routes("OrderService", "CreateOrder");
    assert_eq!(found3.len(), 1);
    
    let found4 = registry.find_grpc_routes("UserService", "Nonexistent");
    assert!(found4.is_empty());
    
    let found5 = registry.find_grpc_routes("NonexistentService", "GetUser");
    assert!(found5.is_empty());
}

/// Test RouteRegistry get_grpc_routes multiple services
#[test]
fn test_route_registry_get_grpc_routes_multiple_services() {
    let mut registry = RouteRegistry::new();
    registry.add_grpc_route("Service1".to_string(), Route::new(HttpMethod::POST, "Method1".to_string())).unwrap();
    registry.add_grpc_route("Service2".to_string(), Route::new(HttpMethod::POST, "Method2".to_string())).unwrap();
    
    assert_eq!(registry.get_grpc_routes("Service1").len(), 1);
    assert_eq!(registry.get_grpc_routes("Service2").len(), 1);
    assert!(registry.get_grpc_routes("Service3").is_empty());
}

/// Test RouteRegistry path matching edge cases (tested through find_http_routes)
#[test]
fn test_route_registry_path_matching_edge_cases() {
    // Test each case with a fresh registry to avoid interference
    // Empty paths
    let mut registry1 = RouteRegistry::new();
    registry1.add_http_route(Route::new(HttpMethod::GET, "".to_string())).unwrap();
    let found = registry1.find_http_routes(&HttpMethod::GET, "");
    assert_eq!(found.len(), 1);
    
    // Root path
    let mut registry2 = RouteRegistry::new();
    registry2.add_http_route(Route::new(HttpMethod::GET, "/".to_string())).unwrap();
    let found2 = registry2.find_http_routes(&HttpMethod::GET, "/");
    assert_eq!(found2.len(), 1);
    
    // Wildcard at start
    let mut registry3 = RouteRegistry::new();
    registry3.add_http_route(Route::new(HttpMethod::GET, "/*/users".to_string())).unwrap();
    let found3 = registry3.find_http_routes(&HttpMethod::GET, "/api/users");
    assert_eq!(found3.len(), 1);
    
    // Wildcard at end
    let mut registry4 = RouteRegistry::new();
    registry4.add_http_route(Route::new(HttpMethod::GET, "/api/*".to_string())).unwrap();
    let found4 = registry4.find_http_routes(&HttpMethod::GET, "/api/users");
    assert_eq!(found4.len(), 1);
    
    // Multiple wildcards
    let mut registry5 = RouteRegistry::new();
    registry5.add_http_route(Route::new(HttpMethod::GET, "/*/*".to_string())).unwrap();
    let found5 = registry5.find_http_routes(&HttpMethod::GET, "/api/users");
    assert_eq!(found5.len(), 1);
    
    // Wildcard doesn't match different segment count
    let mut registry6 = RouteRegistry::new();
    registry6.add_http_route(Route::new(HttpMethod::GET, "/api/*".to_string())).unwrap();
    let found6 = registry6.find_http_routes(&HttpMethod::GET, "/api");
    assert!(found6.is_empty());
    
    let mut registry7 = RouteRegistry::new();
    registry7.add_http_route(Route::new(HttpMethod::GET, "/api/*".to_string())).unwrap();
    let found7 = registry7.find_http_routes(&HttpMethod::GET, "/api/users/123");
    assert!(found7.is_empty());
}

/// Test RouteRegistry with special characters in paths
#[test]
fn test_route_registry_special_characters() {
    let mut registry = RouteRegistry::new();
    
    // Paths with hyphens
    registry.add_http_route(Route::new(HttpMethod::GET, "/api/user-profiles".to_string())).unwrap();
    let found = registry.find_http_routes(&HttpMethod::GET, "/api/user-profiles");
    assert_eq!(found.len(), 1);
    
    // Paths with underscores
    registry.add_http_route(Route::new(HttpMethod::GET, "/api/user_profiles".to_string())).unwrap();
    let found2 = registry.find_http_routes(&HttpMethod::GET, "/api/user_profiles");
    assert_eq!(found2.len(), 1);
    
    // Paths with numbers
    registry.add_http_route(Route::new(HttpMethod::GET, "/api/v1/users".to_string())).unwrap();
    let found3 = registry.find_http_routes(&HttpMethod::GET, "/api/v1/users");
    assert_eq!(found3.len(), 1);
}

/// Test RouteRegistry with very long paths
#[test]
fn test_route_registry_long_paths() {
    let mut registry = RouteRegistry::new();
    let long_path = "/api/".to_string() + &"very/".repeat(20) + "endpoint";
    
    registry.add_http_route(Route::new(HttpMethod::GET, long_path.clone())).unwrap();
    let found = registry.find_http_routes(&HttpMethod::GET, &long_path);
    assert_eq!(found.len(), 1);
}

/// Test RouteRegistry with duplicate routes
#[test]
fn test_route_registry_duplicate_routes() {
    let mut registry = RouteRegistry::new();
    
    // Same route added multiple times
    registry.add_http_route(Route::new(HttpMethod::GET, "/api/users".to_string())).unwrap();
    registry.add_http_route(Route::new(HttpMethod::GET, "/api/users".to_string())).unwrap();
    registry.add_http_route(Route::new(HttpMethod::GET, "/api/users".to_string())).unwrap();
    
    let found = registry.find_http_routes(&HttpMethod::GET, "/api/users");
    assert_eq!(found.len(), 3); // All three should be found
}

/// Test RouteRegistry with routes having different priorities
#[test]
fn test_route_registry_different_priorities() {
    let mut registry = RouteRegistry::new();
    
    let route1 = Route::new(HttpMethod::GET, "/api/users".to_string()).with_priority(10);
    let route2 = Route::new(HttpMethod::GET, "/api/users".to_string()).with_priority(20);
    let route3 = Route::new(HttpMethod::GET, "/api/users".to_string()).with_priority(5);
    
    registry.add_http_route(route1).unwrap();
    registry.add_http_route(route2).unwrap();
    registry.add_http_route(route3).unwrap();
    
    let found = registry.find_http_routes(&HttpMethod::GET, "/api/users");
    assert_eq!(found.len(), 3);
    
    // Verify priorities are preserved
    let priorities: Vec<i32> = found.iter().map(|r| r.priority).collect();
    assert!(priorities.contains(&10));
    assert!(priorities.contains(&20));
    assert!(priorities.contains(&5));
}

/// Test RouteRegistry with routes having different metadata
#[test]
fn test_route_registry_different_metadata() {
    let mut registry = RouteRegistry::new();
    
    let route1 = Route::new(HttpMethod::GET, "/api/users".to_string())
        .with_metadata("version".to_string(), json!("v1"));
    let route2 = Route::new(HttpMethod::GET, "/api/users".to_string())
        .with_metadata("version".to_string(), json!("v2"));
    
    registry.add_http_route(route1).unwrap();
    registry.add_http_route(route2).unwrap();
    
    let found = registry.find_http_routes(&HttpMethod::GET, "/api/users");
    assert_eq!(found.len(), 2);
    
    // Both should have different metadata
    let versions: Vec<&serde_json::Value> = found.iter()
        .map(|r| r.metadata.get("version").unwrap())
        .collect();
    assert!(versions.contains(&&json!("v1")));
    assert!(versions.contains(&&json!("v2")));
}

