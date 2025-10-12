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
