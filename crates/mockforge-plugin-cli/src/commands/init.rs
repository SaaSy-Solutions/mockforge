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
