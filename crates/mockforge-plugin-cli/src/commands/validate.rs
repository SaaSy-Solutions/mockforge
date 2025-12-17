//! Validate plugin command

use crate::utils::{current_dir, find_cargo_toml, find_manifest, get_plugin_id, read_manifest};
use anyhow::Result;
use colored::*;
use std::path::Path;

pub async fn validate_plugin(path: Option<&Path>) -> Result<()> {
    // Determine project directory
    let project_dir = if let Some(p) = path {
        p.to_path_buf()
    } else {
        current_dir()?
    };

    println!("{}", "Validating plugin...".cyan().bold());
    println!();

    let mut errors: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    // Check for plugin.yaml
    match find_manifest(&project_dir) {
        Ok(manifest_path) => {
            println!("{} Plugin manifest found", "✓".green());

            // Validate manifest contents
            match read_manifest(&manifest_path) {
                Ok(manifest) => {
                    println!("{} Manifest is valid YAML", "✓".green());

                    // Check required fields
                    if get_plugin_id(&manifest).is_ok() {
                        println!("{} Plugin ID is present", "✓".green());
                    } else {
                        errors.push("Plugin manifest missing 'id' field".to_string());
                    }

                    if manifest.get("version").is_some() {
                        println!("{} Version is present", "✓".green());
                    } else {
                        errors.push("Plugin manifest missing 'version' field".to_string());
                    }

                    if manifest.get("name").is_some() {
                        println!("{} Name is present", "✓".green());
                    } else {
                        warnings
                            .push("Plugin manifest missing 'name' field (recommended)".to_string());
                    }

                    if manifest.get("plugin_type").is_some() {
                        println!("{} Plugin type is specified", "✓".green());
                    } else {
                        errors.push("Plugin manifest missing 'plugin_type' field".to_string());
                    }

                    if manifest.get("author").is_some() {
                        println!("{} Author information present", "✓".green());
                    } else {
                        warnings.push(
                            "Plugin manifest missing 'author' field (recommended)".to_string(),
                        );
                    }
                }
                Err(e) => {
                    errors.push(format!("Invalid YAML: {}", e));
                }
            }
        }
        Err(_) => {
            errors.push("No plugin.yaml or plugin.yml found".to_string());
        }
    }

    // Check for Cargo.toml
    match find_cargo_toml(&project_dir) {
        Ok(cargo_path) => {
            println!("{} Cargo.toml found", "✓".green());

            // Read and validate Cargo.toml
            if let Ok(content) = std::fs::read_to_string(&cargo_path) {
                if content.contains("crate-type") && content.contains("cdylib") {
                    println!("{} Configured as cdylib", "✓".green());
                } else {
                    warnings
                        .push("Cargo.toml should have [lib] crate-type = [\"cdylib\"]".to_string());
                }

                if content.contains("mockforge-plugin-sdk") {
                    println!("{} SDK dependency present", "✓".green());
                } else {
                    warnings.push(
                        "Consider using mockforge-plugin-sdk for easier development".to_string(),
                    );
                }
            }
        }
        Err(_) => {
            errors.push("No Cargo.toml found".to_string());
        }
    }

    // Check for src/lib.rs
    let lib_path = project_dir.join("src").join("lib.rs");
    if lib_path.exists() {
        println!("{} src/lib.rs found", "✓".green());
    } else {
        errors.push("No src/lib.rs found".to_string());
    }

    // Print summary
    println!();
    println!("{}", "Validation Summary".bold());
    println!("{}", "==================".bold());

    if errors.is_empty() && warnings.is_empty() {
        println!("{}", "✓ All checks passed!".green().bold());
    } else {
        if !errors.is_empty() {
            println!();
            println!("{} Errors:", "✗".red().bold());
            for error in &errors {
                println!("  {} {}", "•".red(), error);
            }
        }

        if !warnings.is_empty() {
            println!();
            println!("{} Warnings:", "⚠".yellow().bold());
            for warning in &warnings {
                println!("  {} {}", "•".yellow(), warning);
            }
        }

        if !errors.is_empty() {
            anyhow::bail!("Validation failed with {} error(s)", errors.len());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_valid_plugin_project(dir: &Path) {
        // Create manifest
        let manifest_content = r#"id: test-plugin
version: 1.0.0
name: Test Plugin
description: A test plugin
plugin_type: auth
author:
  name: Test Author
  email: test@example.com
capabilities:
  network: false
  filesystem: false
"#;
        fs::write(dir.join("plugin.yaml"), manifest_content).unwrap();

        // Create Cargo.toml
        let cargo_content = r#"[package]
name = "test-plugin"
version = "1.0.0"

[lib]
crate-type = ["cdylib"]

[dependencies]
mockforge-plugin-sdk = "0.1"
"#;
        fs::write(dir.join("Cargo.toml"), cargo_content).unwrap();

        // Create src/lib.rs
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::write(dir.join("src/lib.rs"), "// Plugin code").unwrap();
    }

    #[tokio::test]
    async fn test_validate_plugin_valid_project() {
        let temp_dir = TempDir::new().unwrap();
        create_valid_plugin_project(temp_dir.path());

        let result = validate_plugin(Some(temp_dir.path())).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validate_plugin_no_manifest() {
        let temp_dir = TempDir::new().unwrap();

        let result = validate_plugin(Some(temp_dir.path())).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_plugin_invalid_yaml() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("plugin.yaml"), "invalid: yaml: [[[").unwrap();

        let result = validate_plugin(Some(temp_dir.path())).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_plugin_missing_id() {
        let temp_dir = TempDir::new().unwrap();
        let manifest_content = "version: 1.0.0\nname: Test";
        fs::write(temp_dir.path().join("plugin.yaml"), manifest_content).unwrap();
        fs::write(temp_dir.path().join("Cargo.toml"), "[package]\nname=\"test\"").unwrap();
        fs::create_dir_all(temp_dir.path().join("src")).unwrap();
        fs::write(temp_dir.path().join("src/lib.rs"), "").unwrap();

        let result = validate_plugin(Some(temp_dir.path())).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_plugin_missing_version() {
        let temp_dir = TempDir::new().unwrap();
        let manifest_content = "id: test-plugin\nname: Test";
        fs::write(temp_dir.path().join("plugin.yaml"), manifest_content).unwrap();
        fs::write(temp_dir.path().join("Cargo.toml"), "[package]\nname=\"test\"").unwrap();
        fs::create_dir_all(temp_dir.path().join("src")).unwrap();
        fs::write(temp_dir.path().join("src/lib.rs"), "").unwrap();

        let result = validate_plugin(Some(temp_dir.path())).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_plugin_missing_plugin_type() {
        let temp_dir = TempDir::new().unwrap();
        let manifest_content = "id: test-plugin\nversion: 1.0.0\nname: Test";
        fs::write(temp_dir.path().join("plugin.yaml"), manifest_content).unwrap();
        fs::write(temp_dir.path().join("Cargo.toml"), "[package]\nname=\"test\"").unwrap();
        fs::create_dir_all(temp_dir.path().join("src")).unwrap();
        fs::write(temp_dir.path().join("src/lib.rs"), "").unwrap();

        let result = validate_plugin(Some(temp_dir.path())).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_plugin_no_cargo_toml() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(
            temp_dir.path().join("plugin.yaml"),
            "id: test\nversion: 1.0.0\nplugin_type: auth",
        )
        .unwrap();
        fs::create_dir_all(temp_dir.path().join("src")).unwrap();
        fs::write(temp_dir.path().join("src/lib.rs"), "").unwrap();

        let result = validate_plugin(Some(temp_dir.path())).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_plugin_no_lib_rs() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(
            temp_dir.path().join("plugin.yaml"),
            "id: test\nversion: 1.0.0\nplugin_type: auth",
        )
        .unwrap();
        fs::write(temp_dir.path().join("Cargo.toml"), "[package]\nname=\"test\"").unwrap();

        let result = validate_plugin(Some(temp_dir.path())).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_plugin_yml_extension() {
        let temp_dir = TempDir::new().unwrap();

        let manifest_content = r#"id: test-plugin
version: 1.0.0
plugin_type: auth
"#;
        fs::write(temp_dir.path().join("plugin.yml"), manifest_content).unwrap();
        fs::write(temp_dir.path().join("Cargo.toml"), "[package]\nname=\"test\"").unwrap();
        fs::create_dir_all(temp_dir.path().join("src")).unwrap();
        fs::write(temp_dir.path().join("src/lib.rs"), "").unwrap();

        let result = validate_plugin(Some(temp_dir.path())).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validate_plugin_missing_optional_fields() {
        let temp_dir = TempDir::new().unwrap();

        // Minimal valid manifest (missing name and author)
        let manifest_content = "id: test\nversion: 1.0.0\nplugin_type: auth";
        fs::write(temp_dir.path().join("plugin.yaml"), manifest_content).unwrap();

        let cargo_content = r#"[package]
name = "test"
[lib]
crate-type = ["cdylib"]
[dependencies]
mockforge-plugin-sdk = "0.1"
"#;
        fs::write(temp_dir.path().join("Cargo.toml"), cargo_content).unwrap();
        fs::create_dir_all(temp_dir.path().join("src")).unwrap();
        fs::write(temp_dir.path().join("src/lib.rs"), "").unwrap();

        // Should succeed but with warnings
        let result = validate_plugin(Some(temp_dir.path())).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validate_plugin_cargo_toml_no_cdylib() {
        let temp_dir = TempDir::new().unwrap();

        fs::write(
            temp_dir.path().join("plugin.yaml"),
            "id: test\nversion: 1.0.0\nplugin_type: auth",
        )
        .unwrap();

        // Cargo.toml without cdylib
        let cargo_content = "[package]\nname = \"test\"";
        fs::write(temp_dir.path().join("Cargo.toml"), cargo_content).unwrap();

        fs::create_dir_all(temp_dir.path().join("src")).unwrap();
        fs::write(temp_dir.path().join("src/lib.rs"), "").unwrap();

        // Should succeed but with warnings
        let result = validate_plugin(Some(temp_dir.path())).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validate_plugin_cargo_toml_no_sdk() {
        let temp_dir = TempDir::new().unwrap();

        fs::write(
            temp_dir.path().join("plugin.yaml"),
            "id: test\nversion: 1.0.0\nplugin_type: auth",
        )
        .unwrap();

        let cargo_content = r#"[package]
name = "test"
[lib]
crate-type = ["cdylib"]
"#;
        fs::write(temp_dir.path().join("Cargo.toml"), cargo_content).unwrap();

        fs::create_dir_all(temp_dir.path().join("src")).unwrap();
        fs::write(temp_dir.path().join("src/lib.rs"), "").unwrap();

        // Should succeed but with warnings
        let result = validate_plugin(Some(temp_dir.path())).await;
        assert!(result.is_ok());
    }
}
