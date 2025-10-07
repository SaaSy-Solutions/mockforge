//! Create new plugin project command

use crate::templates::{generate_project, PluginType, TemplateData};
use crate::utils::{ensure_dir, to_kebab_case};
use anyhow::{Context, Result};
use colored::*;
use std::path::Path;
use std::process::Command;

pub async fn create_plugin_project(
    name: &str,
    plugin_type_str: &str,
    output: Option<&Path>,
    author_name: Option<&str>,
    author_email: Option<&str>,
    init_git: bool,
) -> Result<()> {
    // Parse and validate plugin type
    let plugin_type = PluginType::from_str(plugin_type_str)?;

    // Extract plugin name from path if a path was provided
    let plugin_name = Path::new(name)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(name);

    // Determine output directory
    let plugin_id = to_kebab_case(plugin_name);
    let output_dir = if name.contains('/') || name.contains('\\') {
        // If name contains path separators, treat it as a full path
        Path::new(name).to_path_buf()
    } else if let Some(out) = output {
        out.join(&plugin_id)
    } else {
        std::env::current_dir()?.join(&plugin_id)
    };

    // Check if directory already exists
    if output_dir.exists() {
        anyhow::bail!(
            "Directory {} already exists. Choose a different name or location.",
            output_dir.display()
        );
    }

    println!("{}", "Creating new plugin project...".cyan().bold());
    println!("  {} {}", "Name:".bold(), plugin_name);
    println!("  {} {}", "Type:".bold(), plugin_type.as_str());
    println!("  {} {}", "Directory:".bold(), output_dir.display());
    println!();

    // Create output directory
    ensure_dir(&output_dir)?;

    // Prepare template data
    let template_data = TemplateData {
        plugin_name: plugin_name.to_string(),
        plugin_id: plugin_id.clone(),
        plugin_type,
        author_name: author_name.map(String::from),
        author_email: author_email.map(String::from),
    };

    // Generate project from template
    generate_project(&template_data, &output_dir)
        .context("Failed to generate project from template")?;

    println!("{}", "✓ Project files generated".green());

    // Initialize Git repository if requested
    if init_git {
        init_git_repo(&output_dir)?;
        println!("{}", "✓ Git repository initialized".green());
    }

    // Print next steps
    println!();
    println!("{}", "Next steps:".bold().green());
    println!("  1. cd {}", plugin_id);
    println!("  2. cargo build --target wasm32-wasi --release");
    println!("  3. cargo test");
    println!();
    println!("{}", "Or use the MockForge plugin CLI:".bold());
    println!("  mockforge-plugin build --release");
    println!("  mockforge-plugin test");
    println!("  mockforge-plugin package");

    Ok(())
}

fn init_git_repo(dir: &Path) -> Result<()> {
    let status = Command::new("git")
        .arg("init")
        .current_dir(dir)
        .status()
        .context("Failed to execute git init. Is git installed?")?;

    if !status.success() {
        anyhow::bail!("Git init failed");
    }

    // Create initial commit
    let _ = Command::new("git")
        .args(["add", "."])
        .current_dir(dir)
        .status();

    let _ = Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(dir)
        .status();

    Ok(())
}
