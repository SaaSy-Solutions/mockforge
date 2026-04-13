//! Proxy functionality for forwarding requests to upstream services
//!
//! This crate provides proxy/reverse-proxy capabilities for MockForge:
//! - config: Proxy configuration and rule management
//! - handler: Request/response handling and processing
//! - client: HTTP client functionality for upstream requests
//! - middleware: Proxy middleware and request transformation
//! - routing: Route matching and rule evaluation

pub mod body_transform;
pub mod client;
pub mod conditional;
pub mod config;
pub mod handler;
pub mod middleware;
pub mod routing;

// Re-export commonly used types
pub use body_transform::BodyTransformationMiddleware;
pub use config::{BodyTransform, BodyTransformRule, MigrationMode, TransformOperation};
pub use middleware::*;
pub use routing::*;

pub use client::{ProxyClient, ProxyResponse};
pub use conditional::{evaluate_proxy_condition, find_matching_rule};
pub use config::{ProxyConfig, ProxyRule};
pub use handler::ProxyHandler;

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Method;

    #[test]
    fn test_proxy_config() {
        let mut config = ProxyConfig::new("http://api.example.com".to_string());
        config.enabled = true;
        assert!(config.should_proxy(&Method::GET, "/proxy/users"));
        assert!(!config.should_proxy(&Method::GET, "/api/users"));

        let stripped = config.strip_prefix("/proxy/users");
        assert_eq!(stripped, "/users");
    }

    #[test]
    fn test_proxy_config_no_prefix() {
        let mut config = ProxyConfig::new("http://api.example.com".to_string());
        config.prefix = None;
        config.enabled = true;

        assert!(config.should_proxy(&Method::GET, "/api/users"));
        assert!(config.should_proxy(&Method::GET, "/any/path"));

        let stripped = config.strip_prefix("/api/users");
        assert_eq!(stripped, "/api/users");
    }

    #[test]
    fn test_proxy_config_with_rules() {
        let mut config = ProxyConfig::new("http://default.example.com".to_string());
        config.enabled = true;
        config.rules.push(ProxyRule {
            path_pattern: "/api/users/*".to_string(),
            target_url: "http://users.example.com".to_string(),
            enabled: true,
            pattern: "/api/users/*".to_string(),
            upstream_url: "http://users.example.com".to_string(),
            migration_mode: MigrationMode::Auto,
            migration_group: None,
            condition: None,
        });
        config.rules.push(ProxyRule {
            path_pattern: "/api/orders/*".to_string(),
            target_url: "http://orders.example.com".to_string(),
            enabled: true,
            pattern: "/api/orders/*".to_string(),
            upstream_url: "http://orders.example.com".to_string(),
            migration_mode: MigrationMode::Auto,
            migration_group: None,
            condition: None,
        });

        assert!(config.should_proxy(&Method::GET, "/api/users/123"));
        assert!(config.should_proxy(&Method::GET, "/api/orders/456"));

        assert_eq!(config.get_upstream_url("/api/users/123"), "http://users.example.com");
        assert_eq!(config.get_upstream_url("/api/orders/456"), "http://orders.example.com");
        assert_eq!(config.get_upstream_url("/api/products"), "http://default.example.com");
    }

    #[test]
    fn test_proxy_config_passthrough() {
        let mut config = ProxyConfig::new("http://api.example.com".to_string());
        config.passthrough_by_default = true;
        config.prefix = None;
        config.enabled = true;

        assert!(config.should_proxy(&Method::GET, "/api/users"));
        assert!(config.should_proxy(&Method::POST, "/api/orders"));

        config.passthrough_by_default = false;
        config.prefix = Some("/proxy".to_string());

        assert!(config.should_proxy(&Method::GET, "/proxy/users"));
        assert!(!config.should_proxy(&Method::GET, "/api/users"));
    }
}
