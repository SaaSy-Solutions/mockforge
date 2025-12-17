//! Show plugin information command

use crate::utils::{
    current_dir, find_cargo_toml, find_manifest, get_plugin_id, get_plugin_version, read_manifest,
};
use anyhow::Result;
use colored::*;
use std::path::Path;

pub async fn show_plugin_info(path: Option<&Path>) -> Result<()> {
    // Determine project directory
    let project_dir = if let Some(p) = path {
        p.to_path_buf()
    } else {
        current_dir()?
    };

    println!("{}", "Plugin Information".cyan().bold());
    println!("{}", "==================".cyan().bold());
    println!();

    // Read manifest
    let manifest_path = find_manifest(&project_dir)?;
    let manifest = read_manifest(&manifest_path)?;

    // Display basic information
    if let Ok(id) = get_plugin_id(&manifest) {
        println!("{:>15}: {}", "ID".bold(), id);
    }

    if let Ok(version) = get_plugin_version(&manifest) {
        println!("{:>15}: {}", "Version".bold(), version);
    }

    if let Some(name) = manifest.get("name").and_then(|v| v.as_str()) {
        println!("{:>15}: {}", "Name".bold(), name);
    }

    if let Some(desc) = manifest.get("description").and_then(|v| v.as_str()) {
        println!("{:>15}: {}", "Description".bold(), desc);
    }

    if let Some(plugin_type) = manifest.get("plugin_type").and_then(|v| v.as_str()) {
        println!("{:>15}: {}", "Type".bold(), plugin_type);
    }

    // Author information
    if let Some(author) = manifest.get("author") {
        if let Some(name) = author.get("name").and_then(|v| v.as_str()) {
            println!("{:>15}: {}", "Author".bold(), name);
        }
        if let Some(email) = author.get("email").and_then(|v| v.as_str()) {
            println!("{:>15}: {}", "Email".bold(), email);
        }
    }

    // Capabilities
    if let Some(caps) = manifest.get("capabilities") {
        println!();
        println!("{}", "Capabilities:".bold());

        if let Some(network) = caps.get("network").and_then(|v| v.as_bool()) {
            println!("  {:>13}: {}", "Network", if network { "✓".green() } else { "✗".red() });
        }

        if let Some(fs) = caps.get("filesystem").and_then(|v| v.as_bool()) {
            println!("  {:>13}: {}", "Filesystem", if fs { "✓".green() } else { "✗".red() });
        }
    }

    // Resource limits
    if let Some(limits) = manifest.get("resource_limits") {
        println!();
        println!("{}", "Resource Limits:".bold());

        if let Some(mem) = limits.get("max_memory_bytes").and_then(|v| v.as_u64()) {
            let mem_mb = mem as f64 / 1024.0 / 1024.0;
            println!("  {:>13}: {:.1} MB", "Memory", mem_mb);
        }

        if let Some(cpu) = limits.get("max_cpu_time_ms").and_then(|v| v.as_u64()) {
            let cpu_sec = cpu as f64 / 1000.0;
            println!("  {:>13}: {:.1} seconds", "CPU Time", cpu_sec);
        }
    }

    // File locations
    println!();
    println!("{}", "Files:".bold());
    println!("  {:>13}: {}", "Manifest", manifest_path.display());

    if let Ok(cargo_path) = find_cargo_toml(&project_dir) {
        println!("  {:>13}: {}", "Cargo.toml", cargo_path.display());
    }

    // Check for built WASM
    let plugin_id = get_plugin_id(&manifest).unwrap_or_else(|_| "unknown".to_string());
    let plugin_lib = plugin_id.replace('-', "_");

    let release_wasm = project_dir
        .join("target")
        .join("wasm32-wasi")
        .join("release")
        .join(format!("{}.wasm", plugin_lib));

    let debug_wasm = project_dir
        .join("target")
        .join("wasm32-wasi")
        .join("debug")
        .join(format!("{}.wasm", plugin_lib));

    println!();
    println!("{}", "Build Status:".bold());

    if release_wasm.exists() {
        println!("  {:>13}: {} ({})", "Release", "✓".green(), release_wasm.display());
    } else {
        println!("  {:>13}: {}", "Release", "Not built".yellow());
    }

    if debug_wasm.exists() {
        println!("  {:>13}: {} ({})", "Debug", "✓".green(), debug_wasm.display());
    } else {
        println!("  {:>13}: {}", "Debug", "Not built".yellow());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_plugin_project_with_manifest(dir: &Path, manifest_content: &str) {
        fs::write(dir.join("plugin.yaml"), manifest_content).unwrap();
        fs::write(dir.join("Cargo.toml"), "[package]\nname=\"test\"").unwrap();
    }

    #[tokio::test]
    async fn test_show_plugin_info_basic() {
        let temp_dir = TempDir::new().unwrap();

        let manifest_content = r#"id: test-plugin
version: 1.0.0
name: Test Plugin
description: A test plugin for testing
plugin_type: auth
"#;
        create_plugin_project_with_manifest(temp_dir.path(), manifest_content);

        let result = show_plugin_info(Some(temp_dir.path())).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_show_plugin_info_with_author() {
        let temp_dir = TempDir::new().unwrap();

        let manifest_content = r#"id: author-plugin
version: 2.0.0
name: Author Plugin
plugin_type: template
author:
  name: John Doe
  email: john@example.com
"#;
        create_plugin_project_with_manifest(temp_dir.path(), manifest_content);

        let result = show_plugin_info(Some(temp_dir.path())).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_show_plugin_info_with_capabilities() {
        let temp_dir = TempDir::new().unwrap();

        let manifest_content = r#"id: cap-plugin
version: 1.0.0
plugin_type: response
capabilities:
  network: true
  filesystem: false
"#;
        create_plugin_project_with_manifest(temp_dir.path(), manifest_content);

        let result = show_plugin_info(Some(temp_dir.path())).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_show_plugin_info_with_resource_limits() {
        let temp_dir = TempDir::new().unwrap();

        let manifest_content = r#"id: limits-plugin
version: 1.5.0
plugin_type: datasource
resource_limits:
  max_memory_bytes: 20971520
  max_cpu_time_ms: 10000
"#;
        create_plugin_project_with_manifest(temp_dir.path(), manifest_content);

        let result = show_plugin_info(Some(temp_dir.path())).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_show_plugin_info_with_wasm_files() {
        let temp_dir = TempDir::new().unwrap();

        let manifest_content = r#"id: wasm-plugin
version: 1.0.0
plugin_type: auth
"#;
        create_plugin_project_with_manifest(temp_dir.path(), manifest_content);

        // Create WASM files
        let release_dir = temp_dir.path().join("target/wasm32-wasi/release");
        fs::create_dir_all(&release_dir).unwrap();
        fs::write(release_dir.join("wasm_plugin.wasm"), b"release wasm").unwrap();

        let debug_dir = temp_dir.path().join("target/wasm32-wasi/debug");
        fs::create_dir_all(&debug_dir).unwrap();
        fs::write(debug_dir.join("wasm_plugin.wasm"), b"debug wasm").unwrap();

        let result = show_plugin_info(Some(temp_dir.path())).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_show_plugin_info_no_manifest() {
        let temp_dir = TempDir::new().unwrap();

        let result = show_plugin_info(Some(temp_dir.path())).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_show_plugin_info_invalid_yaml() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("plugin.yaml"), "invalid yaml: [[[").unwrap();

        let result = show_plugin_info(Some(temp_dir.path())).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_show_plugin_info_minimal_manifest() {
        let temp_dir = TempDir::new().unwrap();

        // Minimal valid manifest
        let manifest_content = "id: minimal\nversion: 0.1.0";
        create_plugin_project_with_manifest(temp_dir.path(), manifest_content);

        let result = show_plugin_info(Some(temp_dir.path())).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_show_plugin_info_yml_extension() {
        let temp_dir = TempDir::new().unwrap();

        let manifest_content = "id: yml-plugin\nversion: 1.0.0\nname: YML Plugin";
        fs::write(temp_dir.path().join("plugin.yml"), manifest_content).unwrap();
        fs::write(temp_dir.path().join("Cargo.toml"), "[package]\nname=\"test\"").unwrap();

        let result = show_plugin_info(Some(temp_dir.path())).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_show_plugin_info_all_plugin_types() {
        for plugin_type in &["auth", "template", "response", "datasource"] {
            let temp_dir = TempDir::new().unwrap();

            let manifest_content =
                format!("id: {}-plugin\nversion: 1.0.0\nplugin_type: {}", plugin_type, plugin_type);
            create_plugin_project_with_manifest(temp_dir.path(), &manifest_content);

            let result = show_plugin_info(Some(temp_dir.path())).await;
            assert!(result.is_ok(), "Failed for plugin type: {}", plugin_type);
        }
    }

    #[tokio::test]
    async fn test_show_plugin_info_complex_manifest() {
        let temp_dir = TempDir::new().unwrap();

        let manifest_content = r#"id: complex-plugin
version: 3.2.1
name: Complex Plugin Example
description: A comprehensive plugin with all features
plugin_type: auth
author:
  name: Jane Developer
  email: jane@company.com
capabilities:
  network: true
  filesystem: true
resource_limits:
  max_memory_bytes: 52428800
  max_cpu_time_ms: 15000
"#;
        create_plugin_project_with_manifest(temp_dir.path(), manifest_content);

        let result = show_plugin_info(Some(temp_dir.path())).await;
        assert!(result.is_ok());
    }
}
