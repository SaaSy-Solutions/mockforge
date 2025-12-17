//! Initialize plugin manifest command

use crate::templates::PluginType;
use anyhow::{Context, Result};
use colored::*;
use std::path::Path;

pub async fn init_manifest(plugin_type_str: &str, output: Option<&Path>) -> Result<()> {
    // Parse plugin type
    let plugin_type = PluginType::from_str(plugin_type_str)?;

    // Determine output path
    let output_path = if let Some(out) = output {
        out.to_path_buf()
    } else {
        std::env::current_dir()?.join("plugin.yaml")
    };

    // Check if file already exists
    if output_path.exists() {
        anyhow::bail!(
            "File {} already exists. Remove it first or specify a different output path.",
            output_path.display()
        );
    }

    println!("{}", "Creating plugin manifest...".cyan().bold());
    println!("  {} {}", "Type:".bold(), plugin_type.as_str());
    println!("  {} {}", "Output:".bold(), output_path.display());

    // Generate manifest template
    let manifest_content = generate_manifest_template(plugin_type);

    // Write to file
    std::fs::write(&output_path, manifest_content)
        .with_context(|| format!("Failed to write manifest to {}", output_path.display()))?;

    println!();
    println!("{}", "✓ Manifest created!".green().bold());
    println!();
    println!("{}", "Edit the manifest to customize your plugin:".bold());
    println!("  - Update id, version, and name");
    println!("  - Set author information");
    println!("  - Configure capabilities and resource limits");

    Ok(())
}

fn generate_manifest_template(plugin_type: PluginType) -> String {
    format!(
        r#"id: my-plugin
version: 0.1.0
name: My {} Plugin
description: A custom {} plugin for MockForge

author:
  name: Your Name
  email: you@example.com

plugin_type: {}

capabilities:
  network: false
  filesystem: false

resource_limits:
  max_memory_bytes: 10485760  # 10MB
  max_cpu_time_ms: 5000       # 5 seconds
"#,
        plugin_type.as_str(),
        plugin_type.as_str(),
        plugin_type.as_str()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_generate_manifest_template_auth() {
        let manifest = generate_manifest_template(PluginType::Auth);
        assert!(manifest.contains("plugin_type: auth"));
        assert!(manifest.contains("My auth Plugin"));
        assert!(manifest.contains("id: my-plugin"));
        assert!(manifest.contains("capabilities:"));
        assert!(manifest.contains("resource_limits:"));
    }

    #[test]
    fn test_generate_manifest_template_template() {
        let manifest = generate_manifest_template(PluginType::Template);
        assert!(manifest.contains("plugin_type: template"));
        assert!(manifest.contains("A custom template plugin"));
    }

    #[test]
    fn test_generate_manifest_template_response() {
        let manifest = generate_manifest_template(PluginType::Response);
        assert!(manifest.contains("plugin_type: response"));
    }

    #[test]
    fn test_generate_manifest_template_datasource() {
        let manifest = generate_manifest_template(PluginType::DataSource);
        assert!(manifest.contains("plugin_type: datasource"));
    }

    #[test]
    fn test_generate_manifest_template_structure() {
        let manifest = generate_manifest_template(PluginType::Auth);

        // Check YAML structure
        assert!(manifest.contains("id:"));
        assert!(manifest.contains("version:"));
        assert!(manifest.contains("name:"));
        assert!(manifest.contains("description:"));
        assert!(manifest.contains("author:"));
        assert!(manifest.contains("  name:"));
        assert!(manifest.contains("  email:"));
        assert!(manifest.contains("plugin_type:"));
        assert!(manifest.contains("capabilities:"));
        assert!(manifest.contains("  network: false"));
        assert!(manifest.contains("  filesystem: false"));
        assert!(manifest.contains("resource_limits:"));
        assert!(manifest.contains("  max_memory_bytes: 10485760"));
        assert!(manifest.contains("  max_cpu_time_ms: 5000"));
    }

    #[tokio::test]
    async fn test_init_manifest_creates_file() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("test.yaml");

        let result = init_manifest("auth", Some(&output_path)).await;
        assert!(result.is_ok());
        assert!(output_path.exists());

        let content = std::fs::read_to_string(&output_path).unwrap();
        assert!(content.contains("plugin_type: auth"));
    }

    #[tokio::test]
    async fn test_init_manifest_default_path() {
        let temp_dir = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();

        std::env::set_current_dir(&temp_dir).unwrap();

        let result = init_manifest("template", None).await;
        assert!(result.is_ok());

        let default_path = temp_dir.path().join("plugin.yaml");
        assert!(default_path.exists());

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[tokio::test]
    async fn test_init_manifest_file_exists() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("exists.yaml");

        std::fs::write(&output_path, "existing content").unwrap();

        let result = init_manifest("auth", Some(&output_path)).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[tokio::test]
    async fn test_init_manifest_invalid_plugin_type() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("test.yaml");

        let result = init_manifest("invalid-type", Some(&output_path)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_init_manifest_all_plugin_types() {
        let temp_dir = TempDir::new().unwrap();

        for plugin_type in &["auth", "template", "response", "datasource"] {
            let output_path = temp_dir.path().join(format!("{}.yaml", plugin_type));
            let result = init_manifest(plugin_type, Some(&output_path)).await;
            assert!(result.is_ok(), "Failed for plugin type: {}", plugin_type);
            assert!(output_path.exists());
        }
    }
}
