//! Proxy routing logic

use crate::{
    routing::{HttpMethod, Route},
    Result,
};
use std::collections::HashMap;

/// Proxy router for determining if a request should be proxied
pub struct ProxyRouter {
    /// Routes that should be proxied
    proxy_routes: HashMap<HttpMethod, Vec<Route>>,
}

impl ProxyRouter {
    /// Create a new proxy router
    pub fn new() -> Self {
        Self {
            proxy_routes: HashMap::new(),
        }
    }

    /// Add a route that should be proxied
    pub fn add_proxy_route(&mut self, route: Route) -> Result<()> {
        self.proxy_routes.entry(route.method.clone()).or_default().push(route);
        Ok(())
    }

    /// Check if a request should be proxied
    pub fn should_proxy(&self, method: &HttpMethod, path: &str) -> bool {
        if let Some(routes) = self.proxy_routes.get(method) {
            routes.iter().any(|route| self.matches_path(&route.path, path))
        } else {
            false
        }
    }

    /// Get the target URL for a proxied request
    pub fn get_target_url(
        &self,
        method: &HttpMethod,
        path: &str,
        base_url: &str,
    ) -> Option<String> {
        if let Some(routes) = self.proxy_routes.get(method) {
            for route in routes {
                if self.matches_path(&route.path, path) {
                    // Perform URL rewriting based on the route pattern
                    let target_path = self.rewrite_path(&route.path, path);
                    return Some(format!("{}{}", base_url.trim_end_matches('/'), target_path));
                }
            }
        }
        None
    }

    /// Simple path matching with wildcard support (* matches any segment)
    fn matches_path(&self, route_path: &str, request_path: &str) -> bool {
        if route_path == request_path {
            return true;
        }

        // Support wildcard matching (* matches any segment)
        if route_path.contains('*') {
            let pattern_parts: Vec<&str> = route_path.split('/').collect();
            let path_parts: Vec<&str> = request_path.split('/').collect();

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

    /// Rewrite the request path based on the route pattern
    fn rewrite_path(&self, pattern: &str, path: &str) -> String {
        if pattern == path {
            return path.to_string();
        }

        // For wildcard patterns like "/api/*", strip the prefix and keep the rest
        if pattern.ends_with("/*") {
            let prefix = &pattern[..pattern.len() - 2]; // Remove "/*"
            if path.starts_with(prefix) && path.len() > prefix.len() {
                let remaining = &path[prefix.len()..];
                // Ensure we don't have double slashes
                if remaining.starts_with('/') {
                    return remaining.to_string();
                } else {
                    return format!("/{}", remaining);
                }
            }
        }

        // For exact matches or other patterns, return the path as-is
        path.to_string()
    }
}

impl Default for ProxyRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routing::HttpMethod;

    #[test]
    fn test_matches_path_exact() {
        let router = ProxyRouter::new();
        assert!(router.matches_path("/api/users", "/api/users"));
        assert!(!router.matches_path("/api/users", "/api/posts"));
    }

    #[test]
    fn test_matches_path_wildcard() {
        let router = ProxyRouter::new();
        assert!(router.matches_path("/api/*", "/api/users"));
        assert!(router.matches_path("/api/*", "/api/posts"));
        assert!(!router.matches_path("/api/*", "/admin/users"));
        assert!(!router.matches_path("/api/*", "/api/users/profile"));
    }

    #[test]
    fn test_rewrite_path_exact() {
        let router = ProxyRouter::new();
        assert_eq!(router.rewrite_path("/api/users", "/api/users"), "/api/users");
    }

    #[test]
    fn test_rewrite_path_wildcard() {
        let router = ProxyRouter::new();
        assert_eq!(router.rewrite_path("/api/*", "/api/users"), "/users");
        assert_eq!(router.rewrite_path("/proxy/*", "/proxy/api/v1/users"), "/api/v1/users");
        assert_eq!(router.rewrite_path("/v1/*", "/v1/api/users"), "/api/users");
    }

    #[test]
    fn test_get_target_url() {
        let mut router = ProxyRouter::new();
        let route = crate::routing::Route::new(HttpMethod::GET, "/api/*".to_string());
        router.add_proxy_route(route).unwrap();

        let base_url = "http://backend:8080";
        assert_eq!(
            router.get_target_url(&HttpMethod::GET, "/api/users", base_url),
            Some("http://backend:8080/users".to_string())
        );
        assert_eq!(
            router.get_target_url(&HttpMethod::GET, "/api/posts", base_url),
            Some("http://backend:8080/posts".to_string())
        );
        assert_eq!(router.get_target_url(&HttpMethod::GET, "/admin/users", base_url), None);
    }
}
