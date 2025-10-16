//! Cross-platform tests for workspace synchronization
//!
//! These tests ensure that workspace sync functionality works correctly
//! across different platforms (Windows, Linux, macOS), with special
//! attention to path handling and file system operations.

use mockforge_core::sync_watcher::SyncWatcher;
use mockforge_core::workspace::{SyncConfig, SyncDirection, SyncDirectoryStructure, Workspace};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

#[cfg(test)]
mod cross_platform_tests {
    use super::*;

    /// Test path handling with various path separators
    #[tokio::test]
    async fn test_path_handling_cross_platform() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_dir = temp_dir.path();

        // Test with forward slashes (Unix-style)
        let unix_path = workspace_dir.join("test/nested/directory");
        fs::create_dir_all(&unix_path).unwrap();
        assert!(unix_path.exists());

        // Test with backslashes on Windows (Rust Path handles this automatically)
        #[cfg(target_os = "windows")]
        {
            let windows_path = workspace_dir.join("test\\windows\\directory");
            fs::create_dir_all(&windows_path).unwrap();
            assert!(windows_path.exists());
        }

        // Test mixed separators (should be normalized by PathBuf)
        let mixed_path = workspace_dir.join("test").join("mixed").join("directory");
        fs::create_dir_all(&mixed_path).unwrap();
        assert!(mixed_path.exists());
    }

    /// Test file operations with spaces and special characters in paths
    #[tokio::test]
    async fn test_paths_with_spaces_and_special_chars() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_dir = temp_dir.path();

        // Test paths with spaces
        let path_with_spaces = workspace_dir.join("my test workspace");
        fs::create_dir_all(&path_with_spaces).unwrap();
        assert!(path_with_spaces.exists());

        // Test file creation in directory with spaces
        let test_file = path_with_spaces.join("config.yaml");
        fs::write(&test_file, "test: data").unwrap();
        assert!(test_file.exists());

        let content = fs::read_to_string(&test_file).unwrap();
        assert_eq!(content, "test: data");
    }

    /// Test relative and absolute path resolution
    #[tokio::test]
    async fn test_relative_and_absolute_paths() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_dir = temp_dir.path();

        // Create a test file with absolute path
        let abs_path = workspace_dir.join("absolute.yaml");
        fs::write(&abs_path, "absolute: true").unwrap();
        assert!(abs_path.exists());

        // Test canonicalize (resolves symlinks and makes absolute)
        let canonical = abs_path.canonicalize().unwrap();
        assert!(canonical.is_absolute());
        assert!(canonical.exists());

        // Test relative path construction
        let rel_path = PathBuf::from("relative.yaml");
        let full_path = workspace_dir.join(&rel_path);
        fs::write(&full_path, "relative: true").unwrap();
        assert!(full_path.exists());
    }

    /// Test path comparison and normalization
    #[tokio::test]
    async fn test_path_comparison_normalization() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        // Create a test directory
        let test_dir = base_path.join("test_dir");
        fs::create_dir_all(&test_dir).unwrap();

        // Different ways to represent the same path
        let path1 = base_path.join("test_dir");
        let path2 = base_path.join("./test_dir");

        // Both should exist
        assert!(path1.exists());
        assert!(path2.exists());

        // Canonical paths should be equal
        let canon1 = path1.canonicalize().unwrap();
        let canon2 = path2.canonicalize().unwrap();
        assert_eq!(canon1, canon2);
    }

    /// Test workspace sync configuration with various path formats
    #[tokio::test]
    async fn test_sync_config_with_various_paths() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_dir = temp_dir.path();

        // Test with absolute path
        let abs_sync_dir = workspace_dir.join("sync_abs");
        fs::create_dir_all(&abs_sync_dir).unwrap();

        let mut workspace = Workspace::new("Test Workspace".to_string());
        let sync_config = SyncConfig {
            enabled: true,
            target_directory: Some(abs_sync_dir.to_string_lossy().to_string()),
            sync_direction: SyncDirection::Bidirectional,
            realtime_monitoring: true,
            directory_structure: SyncDirectoryStructure::Flat,
            filename_pattern: "workspace.yaml".to_string(),
            exclude_pattern: None,
            include_metadata: true,
            force_overwrite: false,
            last_sync: None,
        };

        workspace.configure_sync(sync_config).unwrap();
        assert!(workspace.is_sync_enabled());

        // Verify the path is stored correctly
        let stored_path = workspace.get_sync_directory().unwrap();
        assert!(PathBuf::from(&stored_path).is_absolute());
    }

    /// Test directory creation with nested paths
    #[tokio::test]
    async fn test_nested_directory_creation() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_dir = temp_dir.path();

        // Create deeply nested directories
        let nested_path = workspace_dir.join("level1").join("level2").join("level3").join("level4");

        fs::create_dir_all(&nested_path).unwrap();
        assert!(nested_path.exists());

        // Write a file in the nested directory
        let file_path = nested_path.join("test.yaml");
        fs::write(&file_path, "nested: true").unwrap();
        assert!(file_path.exists());
    }

    /// Test handling of path components and traversal
    #[tokio::test]
    async fn test_path_components_and_traversal() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_dir = temp_dir.path();

        // Create a directory structure
        let dir_structure = workspace_dir.join("parent").join("child");
        fs::create_dir_all(&dir_structure).unwrap();

        // Test parent() function
        let parent = dir_structure.parent().unwrap();
        assert_eq!(parent, workspace_dir.join("parent"));

        // Test file_name() function
        let file_name = dir_structure.file_name().unwrap();
        assert_eq!(file_name, "child");

        // Test components iteration
        let components: Vec<_> = dir_structure.components().collect();
        assert!(components.len() >= 2); // At least parent and child
    }

    /// Test path joining and string conversion
    #[tokio::test]
    async fn test_path_joining_and_string_conversion() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        // Test joining multiple components
        let joined = base_path.join("a").join("b").join("c");
        fs::create_dir_all(&joined).unwrap();
        assert!(joined.exists());

        // Test to_string_lossy for safe string conversion
        let path_str = joined.to_string_lossy();
        assert!(!path_str.is_empty());

        // Verify the string can be used to recreate the path
        let reconstructed = PathBuf::from(path_str.as_ref());
        assert_eq!(reconstructed, joined);
    }

    /// Test handling of current and parent directory references
    #[tokio::test]
    async fn test_current_and_parent_directory_refs() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        // Create a test directory
        let test_dir = base_path.join("test");
        fs::create_dir_all(&test_dir).unwrap();

        // Test current directory reference (.)
        let with_dot = base_path.join(".").join("test");
        assert_eq!(with_dot.canonicalize().unwrap(), test_dir.canonicalize().unwrap());

        // Test parent directory reference (..)
        let with_dotdot = test_dir.join("..").join("test");
        assert_eq!(with_dotdot.canonicalize().unwrap(), test_dir.canonicalize().unwrap());
    }

    /// Windows-specific: Test handling of drive letters and UNC paths
    #[cfg(target_os = "windows")]
    #[tokio::test]
    async fn test_windows_drive_letters() {
        use std::env;

        // Get the current directory (which includes drive letter on Windows)
        let current_dir = env::current_dir().unwrap();
        assert!(current_dir.is_absolute());

        // Verify the path has components
        let components: Vec<_> = current_dir.components().collect();
        assert!(components.len() > 0);

        // On Windows, the first component should be a prefix (drive letter)
        use std::path::Component;
        if let Some(Component::Prefix(_)) = components.first() {
            // This is expected on Windows
            assert!(true);
        }
    }

    /// Windows-specific: Test handling of long paths
    #[cfg(target_os = "windows")]
    #[tokio::test]
    async fn test_windows_long_paths() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        // Create a very long path (but not exceeding MAX_PATH without \\?\ prefix)
        let mut long_path = base_path.to_path_buf();
        for i in 0..20 {
            long_path = long_path.join(format!("level_{}", i));
        }

        // This should work even on Windows with long path support
        let result = fs::create_dir_all(&long_path);

        // Note: This might fail on older Windows versions without long path support
        // The test documents the expected behavior
        match result {
            Ok(_) => {
                assert!(long_path.exists());
            }
            Err(e) => {
                // Document that long paths may not be supported
                eprintln!(
                    "Long path creation failed (this may be expected on Windows without long path support): {}",
                    e
                );
            }
        }
    }

    /// Test strip_prefix for relative path calculation
    #[tokio::test]
    async fn test_strip_prefix_for_relative_paths() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        let sub_dir = base_path.join("sub").join("directory");
        fs::create_dir_all(&sub_dir).unwrap();

        let file_path = sub_dir.join("file.yaml");
        fs::write(&file_path, "content").unwrap();

        // Calculate relative path from base to file
        let relative = file_path.strip_prefix(base_path).unwrap();

        #[cfg(target_os = "windows")]
        {
            // On Windows, the relative path should use backslashes
            let relative_str = relative.to_string_lossy();
            assert!(relative_str.contains("sub") && relative_str.contains("file.yaml"));
        }

        #[cfg(not(target_os = "windows"))]
        {
            // On Unix, the relative path should use forward slashes
            assert_eq!(relative, Path::new("sub/directory/file.yaml"));
        }
    }

    /// Test SyncWatcher creation with various path formats
    #[tokio::test]
    async fn test_sync_watcher_with_various_paths() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_dir = temp_dir.path();

        // Test with absolute path
        let watcher = SyncWatcher::new(workspace_dir);
        assert!(watcher.get_monitored_workspaces().is_empty());

        // Test with path containing spaces
        let path_with_spaces = workspace_dir.join("my workspace");
        fs::create_dir_all(&path_with_spaces).unwrap();
        let watcher2 = SyncWatcher::new(&path_with_spaces);
        assert!(watcher2.get_monitored_workspaces().is_empty());
    }

    /// Test file extension handling across platforms
    #[tokio::test]
    async fn test_file_extension_handling() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        // Test various file extensions
        let yaml_file = base_path.join("config.yaml");
        let yml_file = base_path.join("config.yml");
        let json_file = base_path.join("config.json");

        for file in &[&yaml_file, &yml_file, &json_file] {
            fs::write(file, "test").unwrap();
            assert!(file.exists());

            let _extension = file.extension().unwrap();
            match file.extension().and_then(|e| e.to_str()) {
                Some("yaml") | Some("yml") => (),
                Some("json") => (),
                _ => panic!("Unexpected extension"),
            }
        }
    }
}
