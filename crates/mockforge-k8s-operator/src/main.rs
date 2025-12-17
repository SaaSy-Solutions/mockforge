//! MockForge Kubernetes Operator Main Entry Point

use kube::Client;
use mockforge_k8s_operator::Controller;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Get the default tracing filter string
pub fn default_tracing_filter() -> String {
    "mockforge_k8s_operator=info,kube=info".to_string()
}

/// Get watch namespace from environment
pub fn get_watch_namespace() -> Option<String> {
    std::env::var("WATCH_NAMESPACE").ok()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| default_tracing_filter().into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting MockForge Kubernetes Operator");

    // Create Kubernetes client
    let client = Client::try_default().await?;

    info!("Connected to Kubernetes cluster");

    // Create and run controller
    let controller = Controller::new(client);

    // Get namespace from environment variable, or watch all namespaces
    let namespace = get_watch_namespace();

    if let Err(e) = controller.run(namespace).await {
        error!("Controller error: {:?}", e);
        std::process::exit(1);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_tracing_filter() {
        let filter = default_tracing_filter();
        assert!(filter.contains("mockforge_k8s_operator=info"));
        assert!(filter.contains("kube=info"));
    }

    #[test]
    fn test_default_tracing_filter_not_empty() {
        let filter = default_tracing_filter();
        assert!(!filter.is_empty());
    }

    #[test]
    fn test_default_tracing_filter_format() {
        let filter = default_tracing_filter();
        assert_eq!(filter, "mockforge_k8s_operator=info,kube=info");
    }

    #[test]
    fn test_get_watch_namespace_not_set() {
        // When WATCH_NAMESPACE is not set, should return None
        std::env::remove_var("WATCH_NAMESPACE");
        let namespace = get_watch_namespace();
        assert!(namespace.is_none());
    }

    #[test]
    fn test_get_watch_namespace_set() {
        // When WATCH_NAMESPACE is set, should return the value
        let test_namespace = "test-namespace";
        std::env::set_var("WATCH_NAMESPACE", test_namespace);
        let namespace = get_watch_namespace();
        assert_eq!(namespace, Some(test_namespace.to_string()));
        std::env::remove_var("WATCH_NAMESPACE");
    }

    #[test]
    fn test_get_watch_namespace_empty_string() {
        // When WATCH_NAMESPACE is empty string, should return Some("")
        std::env::set_var("WATCH_NAMESPACE", "");
        let namespace = get_watch_namespace();
        assert_eq!(namespace, Some("".to_string()));
        std::env::remove_var("WATCH_NAMESPACE");
    }

    #[test]
    fn test_get_watch_namespace_with_special_chars() {
        // Test namespace with hyphens and underscores
        let test_namespace = "my-test_namespace-123";
        std::env::set_var("WATCH_NAMESPACE", test_namespace);
        let namespace = get_watch_namespace();
        assert_eq!(namespace, Some(test_namespace.to_string()));
        std::env::remove_var("WATCH_NAMESPACE");
    }

    #[test]
    fn test_get_watch_namespace_default_namespace() {
        // Test with "default" namespace
        std::env::set_var("WATCH_NAMESPACE", "default");
        let namespace = get_watch_namespace();
        assert_eq!(namespace, Some("default".to_string()));
        std::env::remove_var("WATCH_NAMESPACE");
    }

    #[test]
    fn test_get_watch_namespace_kube_system() {
        // Test with "kube-system" namespace
        std::env::set_var("WATCH_NAMESPACE", "kube-system");
        let namespace = get_watch_namespace();
        assert_eq!(namespace, Some("kube-system".to_string()));
        std::env::remove_var("WATCH_NAMESPACE");
    }

    #[test]
    fn test_environment_variable_isolation() {
        // Ensure tests don't interfere with each other
        std::env::remove_var("WATCH_NAMESPACE");
        assert!(get_watch_namespace().is_none());

        std::env::set_var("WATCH_NAMESPACE", "namespace1");
        assert_eq!(get_watch_namespace(), Some("namespace1".to_string()));

        std::env::set_var("WATCH_NAMESPACE", "namespace2");
        assert_eq!(get_watch_namespace(), Some("namespace2".to_string()));

        std::env::remove_var("WATCH_NAMESPACE");
        assert!(get_watch_namespace().is_none());
    }

    #[test]
    fn test_tracing_filter_components() {
        let filter = default_tracing_filter();
        let parts: Vec<&str> = filter.split(',').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], "mockforge_k8s_operator=info");
        assert_eq!(parts[1], "kube=info");
    }

    #[test]
    fn test_tracing_filter_levels() {
        let filter = default_tracing_filter();
        assert!(filter.contains("=info"));
    }

    // Integration-style tests (these would need a K8s cluster in real scenarios)
    #[test]
    fn test_namespace_scenarios() {
        // Test various namespace scenarios
        let scenarios = vec![
            ("default", Some("default".to_string())),
            ("kube-system", Some("kube-system".to_string())),
            ("kube-public", Some("kube-public".to_string())),
            ("production", Some("production".to_string())),
            ("staging", Some("staging".to_string())),
            ("dev", Some("dev".to_string())),
        ];

        for (namespace, expected) in scenarios {
            std::env::set_var("WATCH_NAMESPACE", namespace);
            let result = get_watch_namespace();
            assert_eq!(result, expected);
            std::env::remove_var("WATCH_NAMESPACE");
        }
    }

    #[test]
    fn test_operator_name_in_filter() {
        let filter = default_tracing_filter();
        assert!(filter.contains("mockforge_k8s_operator"));
    }

    #[test]
    fn test_kube_in_filter() {
        let filter = default_tracing_filter();
        assert!(filter.contains("kube"));
    }

    #[test]
    fn test_namespace_validation_characters() {
        // Test various valid Kubernetes namespace characters
        let valid_namespaces = vec![
            "abc",
            "test-namespace",
            "test123",
            "namespace-with-many-hyphens",
            "a",
            "0123456789",
        ];

        for namespace in valid_namespaces {
            std::env::set_var("WATCH_NAMESPACE", namespace);
            let result = get_watch_namespace();
            assert_eq!(result, Some(namespace.to_string()));
            std::env::remove_var("WATCH_NAMESPACE");
        }
    }

    #[test]
    fn test_multiple_namespace_switches() {
        // Test switching between namespaces
        std::env::set_var("WATCH_NAMESPACE", "ns1");
        assert_eq!(get_watch_namespace(), Some("ns1".to_string()));

        std::env::set_var("WATCH_NAMESPACE", "ns2");
        assert_eq!(get_watch_namespace(), Some("ns2".to_string()));

        std::env::set_var("WATCH_NAMESPACE", "ns3");
        assert_eq!(get_watch_namespace(), Some("ns3".to_string()));

        std::env::remove_var("WATCH_NAMESPACE");
    }

    #[test]
    fn test_watch_all_namespaces() {
        // When no namespace is set, operator should watch all namespaces
        std::env::remove_var("WATCH_NAMESPACE");
        let namespace = get_watch_namespace();
        assert!(namespace.is_none(), "None indicates watching all namespaces");
    }

    #[test]
    fn test_tracing_configuration() {
        // Test that tracing filter is properly formatted for env_filter
        let filter = default_tracing_filter();

        // Should be parseable by EnvFilter
        let result = tracing_subscriber::EnvFilter::try_new(&filter);
        assert!(result.is_ok(), "Filter should be valid for EnvFilter");
    }

    #[test]
    fn test_namespace_length() {
        // Test with various length namespaces
        let short_ns = "a";
        std::env::set_var("WATCH_NAMESPACE", short_ns);
        assert_eq!(get_watch_namespace(), Some(short_ns.to_string()));

        let long_ns = "this-is-a-very-long-namespace-name-for-testing";
        std::env::set_var("WATCH_NAMESPACE", long_ns);
        assert_eq!(get_watch_namespace(), Some(long_ns.to_string()));

        std::env::remove_var("WATCH_NAMESPACE");
    }

    #[test]
    fn test_env_var_case_sensitivity() {
        // Environment variable names are case-sensitive
        std::env::set_var("WATCH_NAMESPACE", "test");
        assert_eq!(get_watch_namespace(), Some("test".to_string()));

        // This should not affect WATCH_NAMESPACE
        std::env::set_var("watch_namespace", "other");
        assert_eq!(get_watch_namespace(), Some("test".to_string()));

        std::env::remove_var("WATCH_NAMESPACE");
        std::env::remove_var("watch_namespace");
    }
}
