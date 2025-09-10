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
