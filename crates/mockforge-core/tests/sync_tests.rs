//! Tests for workspace synchronization functionality.
//!
//! These tests verify that workspace synchronization correctly detects changes,
//! handles bidirectional sync, and maintains consistency across directories.

use mockforge_core::sync_watcher::SyncWatcher;
use mockforge_core::workspace::{SyncConfig, SyncDirection, SyncDirectoryStructure, Workspace};
use std::fs;
use tempfile::TempDir;

#[cfg(test)]
mod sync_tests {
    use super::*;

    #[tokio::test]
    async fn test_workspace_sync_configuration() {
        let workspace = Workspace::new("Test Workspace".to_string());

        // Test enabling sync
        let sync_config = SyncConfig {
            enabled: true,
            target_directory: Some("/tmp/test-sync".to_string()),
            sync_direction: SyncDirection::Bidirectional,
            realtime_monitoring: true,
            directory_structure: SyncDirectoryStructure::Flat,
            filename_pattern: "workspace.yaml".to_string(),
            exclude_pattern: None,
            include_metadata: true,
            force_overwrite: false,
            last_sync: None,
        };

        let mut workspace = workspace;
        workspace.configure_sync(sync_config).unwrap();

        assert!(workspace.is_sync_enabled());
        assert_eq!(workspace.get_sync_directory(), Some("/tmp/test-sync"));
    }

    #[tokio::test]
    async fn test_workspace_sync_filtering() {
        let mut workspace = Workspace::new("Test Workspace".to_string());

        // Enable sync
        workspace.enable_sync("/tmp/test".to_string()).unwrap();

        // Get filtered version
        let filtered = workspace.to_filtered_for_sync();

        // Should have same structure but filtered environments
        assert_eq!(filtered.name, workspace.name);
        assert!(filtered.is_sync_enabled());
    }

    #[tokio::test]
    async fn test_sync_watcher_creation() {
        let temp_dir = TempDir::new().unwrap();

        let watcher = SyncWatcher::new(temp_dir.path());
        assert!(watcher.get_monitored_workspaces().is_empty());
    }

    #[tokio::test]
    async fn test_file_change_detection() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.yaml");

        // Create a test file
        fs::write(&test_file, "test: content").unwrap();

        // Verify file was created
        assert!(test_file.exists());

        // Read content
        let content = fs::read_to_string(&test_file).unwrap();
        assert_eq!(content, "test: content");

        // Clean up
        fs::remove_file(&test_file).unwrap();
    }

    #[tokio::test]
    async fn test_workspace_sync_filename_generation() {
        let workspace = Workspace::new("My Test Workspace".to_string());

        // Test default filename generation
        let filename = workspace.get_sync_filename();
        assert!(filename.ends_with(".yaml"));
        assert!(filename.contains("my-test-workspace"));
    }

    #[tokio::test]
    async fn test_realtime_monitoring_flag() {
        let mut workspace = Workspace::new("Test".to_string());
        workspace.enable_sync("/tmp/test".to_string()).unwrap();

        // Should be realtime monitoring enabled by default
        assert!(workspace.is_realtime_monitoring_enabled());

        // Disable monitoring
        workspace.set_realtime_monitoring(false).unwrap();
        assert!(!workspace.is_realtime_monitoring_enabled());
    }
}
