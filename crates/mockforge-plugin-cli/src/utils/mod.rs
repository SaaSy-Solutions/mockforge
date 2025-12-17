//! Utility functions for the plugin CLI

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Check if cargo is installed
pub fn check_cargo() -> Result<()> {
    Command::new("cargo")
        .arg("--version")
        .output()
        .context("Failed to execute cargo. Is it installed and in PATH?")?;
    Ok(())
}

/// Check if wasm32-wasi target is installed
pub fn check_wasm_target() -> Result<bool> {
    let output = Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
        .context("Failed to check installed Rust targets")?;

    let output_str = String::from_utf8_lossy(&output.stdout);
    Ok(output_str.contains("wasm32-wasi"))
}

/// Install wasm32-wasi target
pub fn install_wasm_target() -> Result<()> {
    println!("Installing wasm32-wasi target...");
    let status = Command::new("rustup")
        .args(["target", "add", "wasm32-wasi"])
        .status()
        .context("Failed to install wasm32-wasi target")?;

    if !status.success() {
        anyhow::bail!("Failed to install wasm32-wasi target");
    }

    Ok(())
}

/// Find the plugin manifest in a directory
pub fn find_manifest(dir: &Path) -> Result<PathBuf> {
    let manifest_path = dir.join("plugin.yaml");
    if manifest_path.exists() {
        return Ok(manifest_path);
    }

    let manifest_path = dir.join("plugin.yml");
    if manifest_path.exists() {
        return Ok(manifest_path);
    }

    anyhow::bail!("No plugin.yaml or plugin.yml found in {}", dir.display())
}

/// Find the Cargo.toml in a directory
pub fn find_cargo_toml(dir: &Path) -> Result<PathBuf> {
    let cargo_path = dir.join("Cargo.toml");
    if cargo_path.exists() {
        return Ok(cargo_path);
    }

    anyhow::bail!("No Cargo.toml found in {}", dir.display())
}

/// Get the WASM output path for a project
pub fn get_wasm_output_path(project_dir: &Path, release: bool) -> Result<PathBuf> {
    let profile = if release { "release" } else { "debug" };
    let target_dir = project_dir.join("target").join("wasm32-wasi").join(profile);

    Ok(target_dir)
}

/// Read and parse plugin manifest
pub fn read_manifest(path: &Path) -> Result<serde_yaml::Value> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read manifest at {}", path.display()))?;

    let manifest: serde_yaml::Value = serde_yaml::from_str(&content)
        .with_context(|| format!("Failed to parse manifest at {}", path.display()))?;

    Ok(manifest)
}

/// Get plugin ID from manifest
pub fn get_plugin_id(manifest: &serde_yaml::Value) -> Result<String> {
    manifest
        .get("id")
        .and_then(|v| v.as_str())
        .map(String::from)
        .context("Plugin manifest must have an 'id' field")
}

/// Get plugin version from manifest
pub fn get_plugin_version(manifest: &serde_yaml::Value) -> Result<String> {
    manifest
        .get("version")
        .and_then(|v| v.as_str())
        .map(String::from)
        .context("Plugin manifest must have a 'version' field")
}

/// Get current directory
pub fn current_dir() -> Result<PathBuf> {
    std::env::current_dir().context("Failed to get current directory")
}

/// Ensure a directory exists
pub fn ensure_dir(path: &Path) -> Result<()> {
    if !path.exists() {
        std::fs::create_dir_all(path)
            .with_context(|| format!("Failed to create directory {}", path.display()))?;
    }
    Ok(())
}

/// Convert a string to a valid Rust identifier
#[allow(dead_code)]
pub fn to_rust_identifier(s: &str) -> String {
    s.replace(['-', ' '], "_").to_lowercase()
}

/// Convert a string to kebab-case
pub fn to_kebab_case(s: &str) -> String {
    s.replace(['_', ' '], "-").to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // to_rust_identifier tests
    #[test]
    fn test_to_rust_identifier_hyphen() {
        assert_eq!(to_rust_identifier("my-plugin"), "my_plugin");
    }

    #[test]
    fn test_to_rust_identifier_space() {
        assert_eq!(to_rust_identifier("my plugin"), "my_plugin");
    }

    #[test]
    fn test_to_rust_identifier_uppercase() {
        assert_eq!(to_rust_identifier("My-Plugin"), "my_plugin");
    }

    #[test]
    fn test_to_rust_identifier_mixed() {
        assert_eq!(to_rust_identifier("My Plugin-Name"), "my_plugin_name");
    }

    #[test]
    fn test_to_rust_identifier_already_valid() {
        assert_eq!(to_rust_identifier("my_plugin"), "my_plugin");
    }

    // to_kebab_case tests
    #[test]
    fn test_to_kebab_case_underscore() {
        assert_eq!(to_kebab_case("my_plugin"), "my-plugin");
    }

    #[test]
    fn test_to_kebab_case_space() {
        assert_eq!(to_kebab_case("my plugin"), "my-plugin");
    }

    #[test]
    fn test_to_kebab_case_uppercase() {
        assert_eq!(to_kebab_case("My_Plugin"), "my-plugin");
    }

    #[test]
    fn test_to_kebab_case_mixed() {
        assert_eq!(to_kebab_case("My Plugin_Name"), "my-plugin-name");
    }

    #[test]
    fn test_to_kebab_case_already_valid() {
        assert_eq!(to_kebab_case("my-plugin"), "my-plugin");
    }

    // find_manifest tests
    #[test]
    fn test_find_manifest_yaml() {
        let temp_dir = TempDir::new().unwrap();
        let manifest_path = temp_dir.path().join("plugin.yaml");
        fs::write(&manifest_path, "id: test-plugin\nversion: 1.0.0").unwrap();

        let result = find_manifest(temp_dir.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), manifest_path);
    }

    #[test]
    fn test_find_manifest_yml() {
        let temp_dir = TempDir::new().unwrap();
        let manifest_path = temp_dir.path().join("plugin.yml");
        fs::write(&manifest_path, "id: test-plugin\nversion: 1.0.0").unwrap();

        let result = find_manifest(temp_dir.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), manifest_path);
    }

    #[test]
    fn test_find_manifest_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let result = find_manifest(temp_dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_find_manifest_prefers_yaml() {
        let temp_dir = TempDir::new().unwrap();
        let yaml_path = temp_dir.path().join("plugin.yaml");
        let yml_path = temp_dir.path().join("plugin.yml");
        fs::write(&yaml_path, "id: yaml-plugin").unwrap();
        fs::write(&yml_path, "id: yml-plugin").unwrap();

        let result = find_manifest(temp_dir.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), yaml_path);
    }

    // find_cargo_toml tests
    #[test]
    fn test_find_cargo_toml_exists() {
        let temp_dir = TempDir::new().unwrap();
        let cargo_path = temp_dir.path().join("Cargo.toml");
        fs::write(&cargo_path, "[package]\nname = \"test\"").unwrap();

        let result = find_cargo_toml(temp_dir.path());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), cargo_path);
    }

    #[test]
    fn test_find_cargo_toml_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let result = find_cargo_toml(temp_dir.path());
        assert!(result.is_err());
    }

    // get_wasm_output_path tests
    #[test]
    fn test_get_wasm_output_path_release() {
        let temp_dir = TempDir::new().unwrap();
        let result = get_wasm_output_path(temp_dir.path(), true).unwrap();
        assert!(result.ends_with("target/wasm32-wasi/release"));
    }

    #[test]
    fn test_get_wasm_output_path_debug() {
        let temp_dir = TempDir::new().unwrap();
        let result = get_wasm_output_path(temp_dir.path(), false).unwrap();
        assert!(result.ends_with("target/wasm32-wasi/debug"));
    }

    // read_manifest tests
    #[test]
    fn test_read_manifest_valid() {
        let temp_dir = TempDir::new().unwrap();
        let manifest_path = temp_dir.path().join("plugin.yaml");
        fs::write(&manifest_path, "id: test-plugin\nversion: 1.0.0\nname: Test Plugin").unwrap();

        let result = read_manifest(&manifest_path);
        assert!(result.is_ok());
        let manifest = result.unwrap();
        assert_eq!(manifest["id"].as_str(), Some("test-plugin"));
        assert_eq!(manifest["version"].as_str(), Some("1.0.0"));
    }

    #[test]
    fn test_read_manifest_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let manifest_path = temp_dir.path().join("nonexistent.yaml");
        let result = read_manifest(&manifest_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_read_manifest_invalid_yaml() {
        let temp_dir = TempDir::new().unwrap();
        let manifest_path = temp_dir.path().join("plugin.yaml");
        fs::write(&manifest_path, "this: is: not: valid: yaml: [").unwrap();

        let result = read_manifest(&manifest_path);
        assert!(result.is_err());
    }

    // get_plugin_id tests
    #[test]
    fn test_get_plugin_id_valid() {
        let manifest: serde_yaml::Value =
            serde_yaml::from_str("id: my-plugin\nversion: 1.0.0").unwrap();
        let result = get_plugin_id(&manifest);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "my-plugin");
    }

    #[test]
    fn test_get_plugin_id_missing() {
        let manifest: serde_yaml::Value = serde_yaml::from_str("version: 1.0.0").unwrap();
        let result = get_plugin_id(&manifest);
        assert!(result.is_err());
    }

    // get_plugin_version tests
    #[test]
    fn test_get_plugin_version_valid() {
        let manifest: serde_yaml::Value =
            serde_yaml::from_str("id: my-plugin\nversion: 2.1.0").unwrap();
        let result = get_plugin_version(&manifest);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "2.1.0");
    }

    #[test]
    fn test_get_plugin_version_missing() {
        let manifest: serde_yaml::Value = serde_yaml::from_str("id: my-plugin").unwrap();
        let result = get_plugin_version(&manifest);
        assert!(result.is_err());
    }

    // ensure_dir tests
    #[test]
    fn test_ensure_dir_creates_new() {
        let temp_dir = TempDir::new().unwrap();
        let new_dir = temp_dir.path().join("new/nested/dir");
        assert!(!new_dir.exists());

        let result = ensure_dir(&new_dir);
        assert!(result.is_ok());
        assert!(new_dir.exists());
    }

    #[test]
    fn test_ensure_dir_existing() {
        let temp_dir = TempDir::new().unwrap();
        let result = ensure_dir(temp_dir.path());
        assert!(result.is_ok());
    }

    // current_dir tests
    #[test]
    fn test_current_dir() {
        let result = current_dir();
        assert!(result.is_ok());
        assert!(result.unwrap().exists());
    }
}
