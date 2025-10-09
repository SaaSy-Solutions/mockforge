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
    s.replace(['-', ' '], "_")
        .to_lowercase()
}

/// Convert a string to kebab-case
pub fn to_kebab_case(s: &str) -> String {
    s.replace(['_', ' '], "-")
        .to_lowercase()
}
