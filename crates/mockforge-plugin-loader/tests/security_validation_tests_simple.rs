//! Simplified security validation tests for the plugin loader
//!
//! This test suite covers basic security aspects using the actual MockForge plugin types

use mockforge_plugin_core::{PluginAuthor, PluginId, PluginInfo, PluginManifest, PluginVersion};
use mockforge_plugin_loader::{PluginLoader, PluginLoaderConfig};
use tempfile::TempDir;

#[cfg(test)]
mod basic_validation {
    use super::*;

    #[tokio::test]
    async fn test_valid_manifest_structure() {
        let temp_dir = TempDir::new().unwrap();
        let config = PluginLoaderConfig {
            plugin_dirs: vec![temp_dir.path().to_string_lossy().to_string()],
            ..Default::default()
        };
        let _loader = PluginLoader::new(config);

        let manifest = create_valid_manifest("test-plugin", "1.0.0");

        // Basic validation should pass
        let result = manifest.validate();
        assert!(result.is_ok(), "Valid manifest should pass validation");
    }

    #[tokio::test]
    async fn test_invalid_plugin_id() {
        let manifest = create_manifest_with_empty_id();

        let result = manifest.validate();
        assert!(result.is_err(), "Empty plugin ID should fail validation");

        let error_message = result.unwrap_err();
        assert!(error_message.contains("Plugin ID cannot be empty"));
    }

    #[tokio::test]
    async fn test_invalid_plugin_name() {
        let manifest = create_manifest_with_empty_name();

        let result = manifest.validate();
        assert!(result.is_err(), "Empty plugin name should fail validation");

        let error_message = result.unwrap_err();
        assert!(error_message.contains("Plugin name cannot be empty"));
    }

    #[tokio::test]
    async fn test_plugin_dependency_validation() {
        let mut manifest = create_valid_manifest("test-plugin", "1.0.0");

        // Add a dependency
        manifest
            .dependencies
            .insert(PluginId::new("dependency-plugin"), PluginVersion::new(1, 0, 0));

        let result = manifest.validate();
        assert!(result.is_ok(), "Manifest with valid dependencies should pass");
    }

    #[tokio::test]
    async fn test_dangerous_plugin_capabilities() {
        let mut manifest = create_valid_manifest("dangerous-plugin", "1.0.0");

        // Add potentially dangerous capabilities
        manifest.capabilities.push("filesystem.write_all".to_string());
        manifest.capabilities.push("network.raw_socket".to_string());
        manifest.capabilities.push("system.execute".to_string());

        // The validation should still pass at the manifest level,
        // but the loader should reject it based on security policies
        let result = manifest.validate();
        assert!(result.is_ok(), "Manifest validation should focus on structure");

        // Security validation would happen at the loader level
        let temp_dir = TempDir::new().unwrap();
        let config = PluginLoaderConfig {
            plugin_dirs: vec![temp_dir.path().to_string_lossy().to_string()],
            ..Default::default()
        };
        let loader = PluginLoader::new(config);

        // This would be where security policies are enforced
        // (Implementation depends on actual loader security features)
        let _validator = loader.validator();
        let _capability_validation_result =
            _validator.validate_capabilities(&manifest.capabilities);
    }

    fn create_valid_manifest(id: &str, version: &str) -> PluginManifest {
        let plugin_id = PluginId::new(id);
        let plugin_version = PluginVersion::parse(version).unwrap();
        let author = PluginAuthor::new("Test Author");

        let info = PluginInfo::new(
            plugin_id,
            plugin_version,
            &format!("Test Plugin {}", id),
            "A test plugin for validation",
            author,
        );

        PluginManifest::new(info)
            .with_capability("test.basic")
            .with_capability("mock.response")
    }

    fn create_manifest_with_empty_id() -> PluginManifest {
        let plugin_id = PluginId::new(""); // Empty ID
        let plugin_version = PluginVersion::new(1, 0, 0);
        let author = PluginAuthor::new("Test Author");

        let info = PluginInfo::new(
            plugin_id,
            plugin_version,
            "Test Plugin",
            "A test plugin with empty ID",
            author,
        );

        PluginManifest::new(info)
    }

    fn create_manifest_with_empty_name() -> PluginManifest {
        let plugin_id = PluginId::new("test-plugin");
        let plugin_version = PluginVersion::new(1, 0, 0);
        let author = PluginAuthor::new("Test Author");

        let info = PluginInfo::new(
            plugin_id,
            plugin_version,
            "", // Empty name
            "A test plugin with empty name",
            author,
        );

        PluginManifest::new(info)
    }
}

#[cfg(test)]
mod loader_security {
    use super::*;

    #[tokio::test]
    async fn test_plugin_directory_validation() {
        // Test with non-existent directory
        let config = PluginLoaderConfig {
            plugin_dirs: vec!["/nonexistent/path".to_string()],
            ..Default::default()
        };

        let loader = PluginLoader::new(config);
        // Should handle non-existent directories gracefully
        let stats = loader.get_load_stats().await;
        assert_eq!(stats.discovered, 0); // No plugins discovered from non-existent path
    }

    #[tokio::test]
    async fn test_plugin_loading_limits() {
        let temp_dir = TempDir::new().unwrap();
        let config = PluginLoaderConfig {
            plugin_dirs: vec![temp_dir.path().to_string_lossy().to_string()],
            max_plugins: 2, // Limit to 2 plugins
            ..Default::default()
        };

        let loader = PluginLoader::new(config);

        // Test that the max_plugins configuration is respected
        // This would be tested in actual plugin loading scenarios
        let stats = loader.get_load_stats().await;
        assert_eq!(stats.discovered, 0); // No plugins in empty directory
    }

    #[tokio::test]
    async fn test_concurrent_plugin_operations() {
        let temp_dir = TempDir::new().unwrap();
        let config = PluginLoaderConfig {
            plugin_dirs: vec![temp_dir.path().to_string_lossy().to_string()],
            ..Default::default()
        };

        let loader = std::sync::Arc::new(PluginLoader::new(config));
        let mut handles = vec![];

        // Test concurrent plugin stats access
        for _i in 0..10 {
            let loader_clone = std::sync::Arc::clone(&loader);
            let handle = tokio::spawn(async move {
                let _stats = loader_clone.get_load_stats().await;
                // Each stats access should complete without panicking
                Ok::<(), ()>(())
            });
            handles.push(handle);
        }

        // Wait for all operations to complete
        for handle in handles {
            let result = handle.await.unwrap();
            // Each operation should complete without panicking
            assert!(result.is_ok());
        }
    }

    #[allow(dead_code)]
    fn create_valid_manifest(id: &str, version: &str) -> PluginManifest {
        let plugin_id = PluginId::new(id);
        let plugin_version = PluginVersion::parse(version).unwrap();
        let author = PluginAuthor::new("Test Author");

        let info = PluginInfo::new(
            plugin_id,
            plugin_version,
            &format!("Test Plugin {}", id),
            "A test plugin for security testing",
            author,
        );

        PluginManifest::new(info).with_capability("test.basic")
    }
}
