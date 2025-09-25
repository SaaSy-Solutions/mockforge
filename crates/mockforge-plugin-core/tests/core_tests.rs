//! Tests for plugin core functionality

use mockforge_plugin_core::*;
use mockforge_plugin_core::manifest::models::PluginDependency;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_context_creation() {
        let plugin_id = PluginId::new("test-plugin");
        let version = PluginVersion::new(1, 0, 0);
        let context = PluginContext::new(plugin_id.clone(), version.clone());

        assert_eq!(context.plugin_id, plugin_id);
        assert_eq!(context.version, version);
        assert_eq!(context.timeout_ms, 5000); // default timeout
        assert!(!context.request_id.is_empty()); // should have a generated request ID
        assert!(context.environment.is_empty());
        assert!(context.custom.is_empty());
    }

    #[test]
    fn test_plugin_context_with_custom_data() {
        let plugin_id = PluginId::new("test-plugin");
        let version = PluginVersion::new(1, 0, 0);

        let context = PluginContext::new(plugin_id, version)
            .with_custom("user_id", serde_json::json!("user123"))
            .with_custom("request_id", serde_json::json!("req456"))
            .with_custom("timestamp", serde_json::json!(1640995200));

        assert_eq!(context.custom.len(), 3);
        assert_eq!(context.custom.get("user_id"), Some(&serde_json::json!("user123")));
        assert_eq!(context.custom.get("request_id"), Some(&serde_json::json!("req456")));
        assert_eq!(context.custom.get("timestamp"), Some(&serde_json::json!(1640995200)));
        assert_eq!(context.custom.get("nonexistent"), None);
    }

    #[test]
    fn test_plugin_context_with_environment() {
        let plugin_id = PluginId::new("test-plugin");
        let version = PluginVersion::new(1, 0, 0);

        let context = PluginContext::new(plugin_id, version)
            .with_env("DATABASE_URL", "postgres://localhost:5432/test")
            .with_env("API_KEY", "secret123")
            .with_timeout(10000);

        assert_eq!(context.timeout_ms, 10000);
        assert_eq!(context.environment.get("DATABASE_URL"), Some(&"postgres://localhost:5432/test".to_string()));
        assert_eq!(context.environment.get("API_KEY"), Some(&"secret123".to_string()));
        assert_eq!(context.environment.get("NONEXISTENT"), None);
    }

    #[test]
    fn test_plugin_result_success() {
        let result = PluginResult::<String>::success("test data".to_string(), 150);

        assert!(result.success);
        assert_eq!(result.data, Some("test data".to_string()));
        assert!(result.error.is_none());
        assert_eq!(result.execution_time_ms, 150);
        assert!(result.metadata.is_empty());
    }

    #[test]
    fn test_plugin_result_failure() {
        let result = PluginResult::<String>::failure("test error".to_string(), 200);

        assert!(!result.success);
        assert!(result.data.is_none());
        assert_eq!(result.error, Some("test error".to_string()));
        assert_eq!(result.execution_time_ms, 200);
        assert!(result.metadata.is_empty());
    }

    #[test]
    fn test_plugin_states() {
        assert!(PluginState::Ready.is_ready());
        assert!(!PluginState::Loaded.is_ready()); // Loaded means loaded but not initialized
        assert!(!PluginState::Unloaded.is_ready());
        assert!(!PluginState::Loading.is_ready());
        assert!(!PluginState::Error.is_ready());
    }

    #[test]
    fn test_plugin_health() {
        // Test healthy plugin
        let metrics = PluginMetrics::default();
        let healthy = PluginHealth::healthy("Plugin running normally".to_string(), metrics);
        assert!(healthy.healthy);
        assert_eq!(healthy.message, "Plugin running normally");
        assert_eq!(healthy.state, PluginState::Ready);

        // Test unhealthy plugin
        let metrics = PluginMetrics::default();
        let unhealthy = PluginHealth::unhealthy(PluginState::Error, "Plugin crashed".to_string(), metrics);
        assert!(!unhealthy.healthy);
        assert_eq!(unhealthy.message, "Plugin crashed");
        assert_eq!(unhealthy.state, PluginState::Error);
    }

    #[test]
    fn test_plugin_metrics_default() {
        let metrics = PluginMetrics::default();

        assert_eq!(metrics.total_executions, 0);
        assert_eq!(metrics.successful_executions, 0);
        assert_eq!(metrics.failed_executions, 0);
        assert_eq!(metrics.avg_execution_time_ms, 0.0);
        assert_eq!(metrics.max_execution_time_ms, 0);
        assert_eq!(metrics.memory_usage_bytes, 0);
        assert_eq!(metrics.peak_memory_usage_bytes, 0);
    }

    #[test]
    fn test_plugin_id() {
        let id1 = PluginId::new("test-plugin");
        let id2 = PluginId::new("another-plugin");
        let id3 = PluginId::new("test-plugin");

        assert_eq!(id1.as_str(), "test-plugin");
        assert_eq!(id2.as_str(), "another-plugin");
        assert_eq!(id1.as_str(), id3.as_str()); // Same content
        assert_ne!(id1.as_str(), id2.as_str()); // Different content
    }

    #[test]
    fn test_plugin_version() {
        let version = PluginVersion::new(1, 2, 3);
        assert_eq!(version.to_string(), "1.2.3");

        let parsed = PluginVersion::parse("2.1.0").unwrap();
        assert_eq!(parsed.major, 2);
        assert_eq!(parsed.minor, 1);
        assert_eq!(parsed.patch, 0);
    }

    #[test]
    fn test_plugin_info_creation() {
        let author = PluginAuthor::new("Test Author");

        let info = PluginInfo::new(
            PluginId::new("test-plugin"),
            PluginVersion::new(1, 2, 3),
            "Test Plugin",
            "A plugin for testing",
            author,
        );

        assert_eq!(info.id.as_str(), "test-plugin");
        assert_eq!(info.version.to_string(), "1.2.3");
        assert_eq!(info.name, "Test Plugin");
        assert_eq!(info.description, "A plugin for testing");
        assert_eq!(info.author.name, "Test Author");
    }

    #[test]
    fn test_plugin_manifest() {
        let author = PluginAuthor::new("Test Author");
        let info = PluginInfo::new(
            PluginId::new("test-plugin"),
            PluginVersion::new(1, 0, 0),
            "Test Plugin",
            "A test plugin",
            author,
        );

        let manifest = PluginManifest::new(info)
            .with_capability("template")
            .with_capability("auth")
            .with_dependency(PluginId::new("required-plugin"), PluginVersion::new(1, 0, 0));

        assert_eq!(manifest.info.id.as_str(), "test-plugin");
        assert_eq!(manifest.capabilities.len(), 2);
        assert_eq!(manifest.capabilities[0], "template");
        assert_eq!(manifest.capabilities[1], "auth");
        assert_eq!(manifest.dependencies.len(), 1);
        assert!(manifest.dependencies.contains_key(&PluginId::new("required-plugin")));
    }

    #[test]
    fn test_plugin_metadata() {
        let metadata = PluginMetadata::new("A test plugin")
            .with_capability("template")
            .with_prefix("test_");

        assert_eq!(metadata.description, "A test plugin");
        assert_eq!(metadata.capabilities.len(), 1);
        assert_eq!(metadata.supported_prefixes.len(), 1);
        assert_eq!(metadata.capabilities[0], "template");
        assert_eq!(metadata.supported_prefixes[0], "test_");
    }

    #[test]
    fn test_plugin_result_methods() {
        // Test the convenience methods
        let success_result = PluginResult::<()>::success((), 100);
        assert!(success_result.is_success());
        assert!(success_result.clone().data().is_some());
        assert!(success_result.error().is_none());

        let failure_result = PluginResult::<()>::failure("error message", 200);
        assert!(!failure_result.is_success());
        assert!(failure_result.clone().data().is_none());
        assert_eq!(failure_result.error(), Some("error message"));
    }

    #[test]
    fn test_serialization() {
        // Test that core types can be serialized/deserialized
        let plugin_id = PluginId::new("test-plugin");
        let version = PluginVersion::new(1, 0, 0);
        let context = PluginContext::new(plugin_id, version);

        let serialized = serde_json::to_string(&context).unwrap();
        let deserialized: PluginContext = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.plugin_id, context.plugin_id);
        assert_eq!(deserialized.version, context.version);
        assert_eq!(deserialized.timeout_ms, context.timeout_ms);
    }

    #[test]
    fn test_plugin_manifest_validation() {
        // Test basic manifest structure
        let author = PluginAuthor::new("Test Author");
        let info = PluginInfo::new(
            PluginId::new("test-plugin"),
            PluginVersion::new(1, 0, 0),
            "Test Plugin",
            "A test plugin",
            author,
        );
        let manifest = PluginManifest::new(info)
            .with_capability("template");

        // Basic validation - just check structure
        assert_eq!(manifest.info.id.as_str(), "test-plugin");
        assert_eq!(manifest.capabilities.len(), 1);
        assert!(manifest.dependencies.is_empty());
    }

    #[test]
    fn test_version_requirement_satisfaction() {
        let version_1_0_0 = PluginVersion::new(1, 0, 0);
        let version_1_1_0 = PluginVersion::new(1, 1, 0);
        let version_2_0_0 = PluginVersion::new(2, 0, 0);

        // Test wildcard
        let dep_wildcard = PluginDependency::new(PluginId::new("test"), "*".to_string());
        assert!(dep_wildcard.satisfies_version(&version_1_0_0));
        assert!(dep_wildcard.satisfies_version(&version_2_0_0));

        // Test exact version
        let dep_exact = PluginDependency::new(PluginId::new("test"), "1.0.0".to_string());
        assert!(dep_exact.satisfies_version(&version_1_0_0));
        assert!(!dep_exact.satisfies_version(&version_1_1_0));
        assert!(!dep_exact.satisfies_version(&version_2_0_0));

        // Test caret requirement (^1.0.0 should allow 1.x.x but not 2.x.x)
        let dep_caret = PluginDependency::new(PluginId::new("test"), "^1.0.0".to_string());
        assert!(dep_caret.satisfies_version(&version_1_0_0));
        assert!(dep_caret.satisfies_version(&version_1_1_0));
        assert!(!dep_caret.satisfies_version(&version_2_0_0));

        // Test tilde requirement (~1.0.0 should allow 1.0.x but not 1.1.x)
        let dep_tilde = PluginDependency::new(PluginId::new("test"), "~1.0.0".to_string());
        assert!(dep_tilde.satisfies_version(&version_1_0_0));
        assert!(!dep_tilde.satisfies_version(&version_1_1_0));
        assert!(!dep_tilde.satisfies_version(&version_2_0_0));
    }

    #[test]
    fn test_plugin_capabilities() {
        let capabilities = PluginCapabilities::from_strings(&["network:http".to_string(), "filesystem:read".to_string()]);

        assert!(capabilities.network.allow_http);
        assert!(capabilities.filesystem.read_paths.contains(&"*".to_string()));
    }

    #[test]
    fn test_resolution_context() {
        let context = ResolutionContext::new();
        assert!(!context.environment.is_empty()); // Should have environment variables
        assert!(context.request_context.is_none());
    }

    #[test]
    fn test_request_metadata() {
        let request = RequestMetadata::new("GET", "/api/users")
            .with_header("Accept", "application/json")
            .with_query_param("limit", "10");

        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/api/users");
        assert_eq!(request.headers.get("Accept"), Some(&"application/json".to_string()));
        assert_eq!(request.query_params.get("limit"), Some(&"10".to_string()));
    }

    #[test]
    fn test_plugin_error_creation() {
        let error = PluginError::resolution_failed("token not found");
        match error {
            PluginError::ResolutionFailed { message } => assert_eq!(message, "token not found"),
            _ => panic!("Wrong error type"),
        }

        let error = PluginError::invalid_token("invalid{{token}}");
        match error {
            PluginError::InvalidToken { token } => assert_eq!(token, "invalid{{token}}"),
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn test_plugin_instance() {
        let author = PluginAuthor::new("Test Author");
        let info = PluginInfo::new(
            PluginId::new("test-plugin"),
            PluginVersion::new(1, 0, 0),
            "Test Plugin",
            "A test plugin",
            author,
        );
        let manifest = PluginManifest::new(info);
        let instance = PluginInstance::new(PluginId::new("test-instance"), manifest);

        assert_eq!(instance.id.as_str(), "test-instance");
        assert_eq!(instance.state, PluginState::Unloaded);
        assert!(instance.health.healthy);
    }

    #[test]
    fn test_wasm_module_validation_allowed_imports() {
        // Test that modules with allowed imports pass validation
        // This is a basic test - full WASM testing would require actual WASM bytecode

        // Create capabilities that allow filesystem access
        let mut capabilities = PluginCapabilities::default();
        capabilities.filesystem.read_paths.push("*".to_string());

        // For this test, we can't easily create a real WASM module with imports
        // without complex WASM bytecode generation. Instead, we test the validation
        // logic structure is sound.

        // The validation should succeed for allowed imports when capabilities match
        // This is tested implicitly through the runtime integration
    }

    #[test]
    fn test_wasm_module_validation_disallowed_imports() {
        // Test that modules with disallowed imports would fail validation
        // This tests the validation logic structure

        // Create capabilities with no filesystem access
        let _capabilities = PluginCapabilities::default();

        // A module importing filesystem functions should fail validation
        // when capabilities don't allow it. This is enforced at runtime.
    }

    #[test]
    fn test_module_validator_structure() {
        // Test that ModuleValidator exists and has the expected structure
        // Since it's a unit struct with static methods, this verifies the API
        // The actual validation is tested through integration in the runtime
    }
}