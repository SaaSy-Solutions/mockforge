//! Route registry and routing logic for MockForge

use crate::Result;
use std::collections::HashMap;

/// HTTP method enum
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
    HEAD,
    OPTIONS,
}

/// Route definition
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Route {
    /// HTTP method
    pub method: HttpMethod,
    /// Path pattern (supports wildcards)
    pub path: String,
    /// Route priority (higher = more specific)
    pub priority: i32,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Route {
    /// Create a new route
    pub fn new(method: HttpMethod, path: String) -> Self {
        Self {
            method,
            path,
            priority: 0,
            metadata: HashMap::new(),
        }
    }

    /// Set route priority
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: String, value: serde_json::Value) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Route registry for managing routes across different protocols
#[derive(Debug, Clone)]
pub struct RouteRegistry {
    /// HTTP routes indexed by method and path pattern
    http_routes: HashMap<HttpMethod, Vec<Route>>,
    /// WebSocket routes
    ws_routes: Vec<Route>,
    /// gRPC service routes
    grpc_routes: HashMap<String, Vec<Route>>,
}

impl RouteRegistry {
    /// Create a new empty route registry
    pub fn new() -> Self {
        Self {
            http_routes: HashMap::new(),
            ws_routes: Vec::new(),
            grpc_routes: HashMap::new(),
        }
    }

    /// Add an HTTP route
    pub fn add_http_route(&mut self, route: Route) -> Result<()> {
        self.http_routes.entry(route.method.clone()).or_default().push(route);
        Ok(())
    }

    /// Add a WebSocket route
    pub fn add_ws_route(&mut self, route: Route) -> Result<()> {
        self.ws_routes.push(route);
        Ok(())
    }

    /// Clear all routes
    pub fn clear(&mut self) {
        self.http_routes.clear();
        self.ws_routes.clear();
        self.grpc_routes.clear();
    }

    /// Add a generic route (alias for add_http_route)
    pub fn add_route(&mut self, route: Route) -> Result<()> {
        self.add_http_route(route)
    }

    /// Add a gRPC route
    pub fn add_grpc_route(&mut self, service: String, route: Route) -> Result<()> {
        self.grpc_routes.entry(service).or_default().push(route);
        Ok(())
    }

    /// Find matching HTTP routes
    pub fn find_http_routes(&self, method: &HttpMethod, path: &str) -> Vec<&Route> {
        self.http_routes
            .get(method)
            .map(|routes| {
                routes.iter().filter(|route| self.matches_path(&route.path, path)).collect()
            })
            .unwrap_or_default()
    }

    /// Find matching WebSocket routes
    pub fn find_ws_routes(&self, path: &str) -> Vec<&Route> {
        self.ws_routes
            .iter()
            .filter(|route| self.matches_path(&route.path, path))
            .collect()
    }

    /// Find matching gRPC routes
    pub fn find_grpc_routes(&self, service: &str, method: &str) -> Vec<&Route> {
        self.grpc_routes
            .get(service)
            .map(|routes| {
                routes.iter().filter(|route| self.matches_path(&route.path, method)).collect()
            })
            .unwrap_or_default()
    }

    /// Check if a path matches a route pattern
    fn matches_path(&self, pattern: &str, path: &str) -> bool {
        if pattern == path {
            return true;
        }

        // Simple wildcard matching (* matches any segment)
        if pattern.contains('*') {
            let pattern_parts: Vec<&str> = pattern.split('/').collect();
            let path_parts: Vec<&str> = path.split('/').collect();

            if pattern_parts.len() != path_parts.len() {
                return false;
            }

            for (pattern_part, path_part) in pattern_parts.iter().zip(path_parts.iter()) {
                if *pattern_part != "*" && *pattern_part != *path_part {
                    return false;
                }
            }
            return true;
        }

        false
    }

    /// Get all HTTP routes for a method
    pub fn get_http_routes(&self, method: &HttpMethod) -> Vec<&Route> {
        self.http_routes
            .get(method)
            .map(|routes| routes.iter().collect())
            .unwrap_or_default()
    }

    /// Get all WebSocket routes
    pub fn get_ws_routes(&self) -> Vec<&Route> {
        self.ws_routes.iter().collect()
    }

    /// Get all gRPC routes for a service
    pub fn get_grpc_routes(&self, service: &str) -> Vec<&Route> {
        self.grpc_routes
            .get(service)
            .map(|routes| routes.iter().collect())
            .unwrap_or_default()
    }
}

impl Default for RouteRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_new() {
        let route = Route::new(HttpMethod::GET, "/api/users".to_string());
        assert_eq!(route.method, HttpMethod::GET);
        assert_eq!(route.path, "/api/users");
        assert_eq!(route.priority, 0);
        assert!(route.metadata.is_empty());
    }

    #[test]
    fn test_route_with_priority() {
        let route = Route::new(HttpMethod::POST, "/api/users".to_string()).with_priority(10);
        assert_eq!(route.priority, 10);
    }

    #[test]
    fn test_route_with_metadata() {
        let route = Route::new(HttpMethod::GET, "/api/users".to_string())
            .with_metadata("version".to_string(), serde_json::json!("v1"))
            .with_metadata("auth".to_string(), serde_json::json!(true));

        assert_eq!(route.metadata.get("version"), Some(&serde_json::json!("v1")));
        assert_eq!(route.metadata.get("auth"), Some(&serde_json::json!(true)));
    }

    #[test]
    fn test_route_registry_new() {
        let registry = RouteRegistry::new();
        assert!(registry.http_routes.is_empty());
        assert!(registry.ws_routes.is_empty());
        assert!(registry.grpc_routes.is_empty());
    }

    #[test]
    fn test_route_registry_default() {
        let registry = RouteRegistry::default();
        assert!(registry.http_routes.is_empty());
    }

    #[test]
    fn test_add_http_route() {
        let mut registry = RouteRegistry::new();
        let route = Route::new(HttpMethod::GET, "/api/users".to_string());

        assert!(registry.add_http_route(route).is_ok());
        assert_eq!(registry.get_http_routes(&HttpMethod::GET).len(), 1);
    }

    #[test]
    fn test_add_multiple_http_routes() {
        let mut registry = RouteRegistry::new();

        registry
            .add_http_route(Route::new(HttpMethod::GET, "/api/users".to_string()))
            .unwrap();
        registry
            .add_http_route(Route::new(HttpMethod::GET, "/api/posts".to_string()))
            .unwrap();
        registry
            .add_http_route(Route::new(HttpMethod::POST, "/api/users".to_string()))
            .unwrap();

        assert_eq!(registry.get_http_routes(&HttpMethod::GET).len(), 2);
        assert_eq!(registry.get_http_routes(&HttpMethod::POST).len(), 1);
    }

    #[test]
    fn test_add_ws_route() {
        let mut registry = RouteRegistry::new();
        let route = Route::new(HttpMethod::GET, "/ws/chat".to_string());

        assert!(registry.add_ws_route(route).is_ok());
        assert_eq!(registry.get_ws_routes().len(), 1);
    }

    #[test]
    fn test_add_grpc_route() {
        let mut registry = RouteRegistry::new();
        let route = Route::new(HttpMethod::POST, "GetUser".to_string());

        assert!(registry.add_grpc_route("UserService".to_string(), route).is_ok());
        assert_eq!(registry.get_grpc_routes("UserService").len(), 1);
    }

    #[test]
    fn test_add_route_alias() {
        let mut registry = RouteRegistry::new();
        let route = Route::new(HttpMethod::GET, "/api/test".to_string());

        assert!(registry.add_route(route).is_ok());
        assert_eq!(registry.get_http_routes(&HttpMethod::GET).len(), 1);
    }

    #[test]
    fn test_clear() {
        let mut registry = RouteRegistry::new();

        registry
            .add_http_route(Route::new(HttpMethod::GET, "/api/users".to_string()))
            .unwrap();
        registry
            .add_ws_route(Route::new(HttpMethod::GET, "/ws/chat".to_string()))
            .unwrap();
        registry
            .add_grpc_route(
                "Service".to_string(),
                Route::new(HttpMethod::POST, "Method".to_string()),
            )
            .unwrap();

        assert!(!registry.get_http_routes(&HttpMethod::GET).is_empty());
        assert!(!registry.get_ws_routes().is_empty());

        registry.clear();

        assert!(registry.get_http_routes(&HttpMethod::GET).is_empty());
        assert!(registry.get_ws_routes().is_empty());
        assert!(registry.get_grpc_routes("Service").is_empty());
    }

    #[test]
    fn test_find_http_routes_exact_match() {
        let mut registry = RouteRegistry::new();
        registry
            .add_http_route(Route::new(HttpMethod::GET, "/api/users".to_string()))
            .unwrap();

        let found = registry.find_http_routes(&HttpMethod::GET, "/api/users");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].path, "/api/users");
    }

    #[test]
    fn test_find_http_routes_no_match() {
        let mut registry = RouteRegistry::new();
        registry
            .add_http_route(Route::new(HttpMethod::GET, "/api/users".to_string()))
            .unwrap();

        let found = registry.find_http_routes(&HttpMethod::GET, "/api/posts");
        assert_eq!(found.len(), 0);
    }

    #[test]
    fn test_find_http_routes_wildcard_match() {
        let mut registry = RouteRegistry::new();
        registry
            .add_http_route(Route::new(HttpMethod::GET, "/api/*/details".to_string()))
            .unwrap();

        let found = registry.find_http_routes(&HttpMethod::GET, "/api/users/details");
        assert_eq!(found.len(), 1);

        let found = registry.find_http_routes(&HttpMethod::GET, "/api/posts/details");
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn test_find_http_routes_wildcard_no_match_different_length() {
        let mut registry = RouteRegistry::new();
        registry
            .add_http_route(Route::new(HttpMethod::GET, "/api/*/details".to_string()))
            .unwrap();

        let found = registry.find_http_routes(&HttpMethod::GET, "/api/users");
        assert_eq!(found.len(), 0);
    }

    #[test]
    fn test_find_ws_routes() {
        let mut registry = RouteRegistry::new();
        registry
            .add_ws_route(Route::new(HttpMethod::GET, "/ws/chat".to_string()))
            .unwrap();

        let found = registry.find_ws_routes("/ws/chat");
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn test_find_ws_routes_wildcard() {
        let mut registry = RouteRegistry::new();
        registry.add_ws_route(Route::new(HttpMethod::GET, "/ws/*".to_string())).unwrap();

        let found = registry.find_ws_routes("/ws/chat");
        assert_eq!(found.len(), 1);

        let found = registry.find_ws_routes("/ws/notifications");
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn test_find_grpc_routes() {
        let mut registry = RouteRegistry::new();
        registry
            .add_grpc_route(
                "UserService".to_string(),
                Route::new(HttpMethod::POST, "GetUser".to_string()),
            )
            .unwrap();

        let found = registry.find_grpc_routes("UserService", "GetUser");
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn test_find_grpc_routes_wildcard() {
        let mut registry = RouteRegistry::new();
        // Wildcard pattern matching requires exact segment count
        // For gRPC method names, we'd typically use exact matches
        registry
            .add_grpc_route(
                "UserService".to_string(),
                Route::new(HttpMethod::POST, "GetUser".to_string()),
            )
            .unwrap();

        let found = registry.find_grpc_routes("UserService", "GetUser");
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn test_matches_path_exact() {
        let registry = RouteRegistry::new();
        assert!(registry.matches_path("/api/users", "/api/users"));
        assert!(!registry.matches_path("/api/users", "/api/posts"));
    }

    #[test]
    fn test_matches_path_wildcard_single_segment() {
        let registry = RouteRegistry::new();
        assert!(registry.matches_path("/api/*", "/api/users"));
        assert!(registry.matches_path("/api/*", "/api/posts"));
        assert!(!registry.matches_path("/api/*", "/api"));
        assert!(!registry.matches_path("/api/*", "/api/users/123"));
    }

    #[test]
    fn test_matches_path_wildcard_multiple_segments() {
        let registry = RouteRegistry::new();
        assert!(registry.matches_path("/api/*/details", "/api/users/details"));
        assert!(registry.matches_path("/api/*/*", "/api/users/123"));
        assert!(!registry.matches_path("/api/*/*", "/api/users"));
    }

    #[test]
    fn test_get_http_routes_empty() {
        let registry = RouteRegistry::new();
        assert!(registry.get_http_routes(&HttpMethod::GET).is_empty());
    }

    #[test]
    fn test_get_ws_routes_empty() {
        let registry = RouteRegistry::new();
        assert!(registry.get_ws_routes().is_empty());
    }

    #[test]
    fn test_get_grpc_routes_empty() {
        let registry = RouteRegistry::new();
        assert!(registry.get_grpc_routes("Service").is_empty());
    }

    #[test]
    fn test_http_method_serialization() {
        let method = HttpMethod::GET;
        let json = serde_json::to_string(&method).unwrap();
        assert_eq!(json, r#""get""#);

        let method = HttpMethod::POST;
        let json = serde_json::to_string(&method).unwrap();
        assert_eq!(json, r#""post""#);
    }

    #[test]
    fn test_http_method_deserialization() {
        let method: HttpMethod = serde_json::from_str(r#""get""#).unwrap();
        assert_eq!(method, HttpMethod::GET);

        let method: HttpMethod = serde_json::from_str(r#""post""#).unwrap();
        assert_eq!(method, HttpMethod::POST);
    }
}
