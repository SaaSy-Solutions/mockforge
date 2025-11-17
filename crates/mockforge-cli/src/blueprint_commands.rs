//! Blueprint management CLI commands
//!
//! Blueprints are predefined app archetypes that provide:
//! - Pre-configured personas
//! - Reality defaults optimized for the use case
//! - Sample flows demonstrating common workflows
//! - Playground collections for testing

use clap::Subcommand;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Blueprint metadata structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintMetadata {
    pub manifest_version: String,
    pub name: String,
    pub version: String,
    pub title: String,
    pub description: String,
    pub author: String,
    #[serde(default)]
    pub author_email: Option<String>,
    pub category: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub setup: Option<BlueprintSetup>,
    #[serde(default)]
    pub compatibility: Option<BlueprintCompatibility>,
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    pub readme: Option<String>,
}

/// Blueprint setup configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintSetup {
    #[serde(default)]
    pub personas: Vec<PersonaInfo>,
    #[serde(default)]
    pub reality: Option<RealityInfo>,
    #[serde(default)]
    pub flows: Vec<FlowInfo>,
    #[serde(default)]
    pub playground: Option<PlaygroundInfo>,
}

/// Persona information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaInfo {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
}

/// Reality level information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealityInfo {
    pub level: String,
    #[serde(default)]
    pub description: Option<String>,
}

/// Flow information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowInfo {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
}

/// Playground information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaygroundInfo {
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub collection_file: String,
}

fn default_true() -> bool {
    true
}

/// Blueprint compatibility requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintCompatibility {
    #[serde(default = "default_min_version")]
    pub min_version: String,
    #[serde(default)]
    pub max_version: Option<String>,
    #[serde(default)]
    pub required_features: Vec<String>,
    #[serde(default)]
    pub protocols: Vec<String>,
}

fn default_min_version() -> String {
    "0.3.0".to_string()
}

/// Blueprint subcommands
#[derive(Subcommand)]
pub enum BlueprintCommands {
    /// List available blueprints
    List {
        /// Show detailed information
        #[arg(short, long)]
        detailed: bool,

        /// Filter by category
        #[arg(short, long)]
        category: Option<String>,
    },

    /// Create a new project from a blueprint
    Create {
        /// Project name
        name: String,

        /// Blueprint ID to use
        #[arg(short, long)]
        blueprint: String,

        /// Output directory (defaults to project name)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Overwrite existing directory
        #[arg(long)]
        force: bool,
    },

    /// Show blueprint information
    Info {
        /// Blueprint ID
        blueprint_id: String,
    },
}

/// Get the blueprints directory
fn get_blueprints_dir() -> PathBuf {
    // For now, use blueprints/ in the project root
    // In the future, this could be configurable or use a registry
    PathBuf::from("blueprints")
}

/// List all available blueprints
pub fn list_blueprints(detailed: bool, category: Option<String>) -> anyhow::Result<()> {
    let blueprints_dir = get_blueprints_dir();

    if !blueprints_dir.exists() {
        println!("No blueprints directory found at: {}", blueprints_dir.display());
        return Ok(());
    }

    let mut blueprints = Vec::new();

    // Scan for blueprint directories
    for entry in fs::read_dir(&blueprints_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let blueprint_yaml = path.join("blueprint.yaml");
            if blueprint_yaml.exists() {
                if let Ok(metadata) = load_blueprint_metadata(&blueprint_yaml) {
                    // Filter by category if specified
                    if let Some(ref cat) = category {
                        if metadata.category != *cat {
                            continue;
                        }
                    }
                    blueprints.push((path, metadata));
                }
            }
        }
    }

    if blueprints.is_empty() {
        println!("No blueprints found.");
        return Ok(());
    }

    // Sort by name
    blueprints.sort_by(|a, b| a.1.name.cmp(&b.1.name));

    println!("Available Blueprints:\n");

    for (path, metadata) in blueprints {
        if detailed {
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!("Name:        {}", metadata.name);
            println!("Title:       {}", metadata.title);
            println!("Version:     {}", metadata.version);
            println!("Category:    {}", metadata.category);
            println!("Description: {}", metadata.description.lines().next().unwrap_or(""));
            if !metadata.tags.is_empty() {
                println!("Tags:        {}", metadata.tags.join(", "));
            }
            println!("Path:        {}", path.display());
            println!();
        } else {
            println!("  • {} ({}) - {}", metadata.name, metadata.category, metadata.title);
        }
    }

    Ok(())
}

/// Create a project from a blueprint
pub fn create_from_blueprint(
    name: String,
    blueprint_id: String,
    output: Option<PathBuf>,
    force: bool,
) -> anyhow::Result<()> {
    let blueprints_dir = get_blueprints_dir();
    let blueprint_path = blueprints_dir.join(&blueprint_id);

    if !blueprint_path.exists() {
        anyhow::bail!("Blueprint '{}' not found at: {}", blueprint_id, blueprint_path.display());
    }

    let blueprint_yaml = blueprint_path.join("blueprint.yaml");
    if !blueprint_yaml.exists() {
        anyhow::bail!("Blueprint metadata not found: {}", blueprint_yaml.display());
    }

    let metadata = load_blueprint_metadata(&blueprint_yaml)?;

    // Determine output directory
    let output_dir = output.unwrap_or_else(|| PathBuf::from(&name));

    if output_dir.exists() && !force {
        anyhow::bail!(
            "Directory '{}' already exists. Use --force to overwrite.",
            output_dir.display()
        );
    }

    // Create output directory
    if output_dir.exists() && force {
        fs::remove_dir_all(&output_dir)?;
    }
    fs::create_dir_all(&output_dir)?;

    println!("Creating project '{}' from blueprint '{}'...", name, blueprint_id);

    // Copy blueprint files
    copy_blueprint_files(&blueprint_path, &output_dir, &metadata)?;

    // Generate project-specific files
    generate_project_files(&output_dir, &name, &metadata)?;

    println!("✅ Project created successfully!");
    println!("\nNext steps:");
    println!("  1. cd {}", output_dir.display());
    println!("  2. Review mockforge.yaml configuration");
    println!("  3. Run: mockforge serve");

    Ok(())
}

/// Show blueprint information
pub fn show_blueprint_info(blueprint_id: String) -> anyhow::Result<()> {
    let blueprints_dir = get_blueprints_dir();
    let blueprint_path = blueprints_dir.join(&blueprint_id);
    let blueprint_yaml = blueprint_path.join("blueprint.yaml");

    if !blueprint_yaml.exists() {
        anyhow::bail!("Blueprint '{}' not found", blueprint_id);
    }

    let metadata = load_blueprint_metadata(&blueprint_yaml)?;

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Blueprint: {}", metadata.name);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("\nTitle:       {}", metadata.title);
    println!("Version:     {}", metadata.version);
    println!("Category:    {}", metadata.category);
    println!("Author:      {}", metadata.author);
    if let Some(email) = &metadata.author_email {
        println!("Email:       {}", email);
    }
    println!("\nDescription:");
    for line in metadata.description.lines() {
        println!("  {}", line);
    }

    if !metadata.tags.is_empty() {
        println!("\nTags: {}", metadata.tags.join(", "));
    }

    if let Some(setup) = &metadata.setup {
        if !setup.personas.is_empty() {
            println!("\nPersonas:");
            for persona in &setup.personas {
                println!("  • {} - {}", persona.id, persona.name);
                if let Some(desc) = &persona.description {
                    println!("    {}", desc);
                }
            }
        }

        if let Some(reality) = &setup.reality {
            println!("\nReality Level: {}", reality.level);
            if let Some(desc) = &reality.description {
                println!("  {}", desc);
            }
        }

        if !setup.flows.is_empty() {
            println!("\nSample Flows:");
            for flow in &setup.flows {
                println!("  • {} - {}", flow.id, flow.name);
                if let Some(desc) = &flow.description {
                    println!("    {}", desc);
                }
            }
        }
    }

    println!("\nPath: {}", blueprint_path.display());

    Ok(())
}

/// Load blueprint metadata from YAML file
fn load_blueprint_metadata(path: &Path) -> anyhow::Result<BlueprintMetadata> {
    let content = fs::read_to_string(path)?;
    let metadata: BlueprintMetadata = serde_yaml::from_str(&content)?;
    Ok(metadata)
}

/// Copy blueprint files to output directory
fn copy_blueprint_files(
    blueprint_path: &Path,
    output_dir: &Path,
    metadata: &BlueprintMetadata,
) -> anyhow::Result<()> {
    // Copy config.yaml if it exists
    let config_src = blueprint_path.join("config.yaml");
    if config_src.exists() {
        let config_dst = output_dir.join("mockforge.yaml");
        fs::copy(&config_src, &config_dst)?;
        println!("  ✓ Created mockforge.yaml");
    }

    // Copy personas directory
    let personas_src = blueprint_path.join("personas");
    if personas_src.exists() {
        let personas_dst = output_dir.join("personas");
        copy_directory(&personas_src, &personas_dst)?;
        println!("  ✓ Copied personas/");
    }

    // Copy flows directory
    let flows_src = blueprint_path.join("flows");
    if flows_src.exists() {
        let flows_dst = output_dir.join("flows");
        copy_directory(&flows_src, &flows_dst)?;
        println!("  ✓ Copied flows/");
    }

    // Copy playground directory
    let playground_src = blueprint_path.join("playground");
    if playground_src.exists() {
        let playground_dst = output_dir.join("playground");
        copy_directory(&playground_src, &playground_dst)?;
        println!("  ✓ Copied playground/");
    }

    // Copy other files listed in metadata
    for file in &metadata.files {
        if file == "blueprint.yaml" || file == "config.yaml" {
            continue; // Already handled
        }

        let src = blueprint_path.join(file);
        if src.exists() {
            if src.is_file() {
                let dst = output_dir.join(file);
                if let Some(parent) = dst.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(&src, &dst)?;
            } else if src.is_dir() {
                let dst = output_dir.join(file);
                copy_directory(&src, &dst)?;
            }
        }
    }

    Ok(())
}

/// Copy a directory recursively
fn copy_directory(src: &Path, dst: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = path.file_name().unwrap();
        let dst_path = dst.join(file_name);

        if path.is_dir() {
            copy_directory(&path, &dst_path)?;
        } else {
            fs::copy(&path, &dst_path)?;
        }
    }

    Ok(())
}

/// Generate project-specific files
fn generate_project_files(
    output_dir: &Path,
    name: &str,
    metadata: &BlueprintMetadata,
) -> anyhow::Result<()> {
    // Generate README if it doesn't exist
    let readme_path = output_dir.join("README.md");
    if !readme_path.exists() {
        let readme_content = generate_readme(name, metadata);
        fs::write(&readme_path, readme_content)?;
        println!("  ✓ Created README.md");
    }

    Ok(())
}

/// Generate README content
fn generate_readme(name: &str, metadata: &BlueprintMetadata) -> String {
    format!(
        r#"# {}

{}

This project was created from the **{}** blueprint.

## Quick Start

```bash
# Start the mock server
mockforge serve

# Or with a specific config
mockforge serve --config mockforge.yaml
```

## What's Included

{}

## Documentation

For more information, visit: https://docs.mockforge.dev
"#,
        name,
        metadata.description,
        metadata.title,
        if let Some(setup) = &metadata.setup {
            let mut sections = Vec::new();

            if !setup.personas.is_empty() {
                sections.push(format!(
                    "### Personas\n\nThis blueprint includes {} predefined personas.",
                    setup.personas.len()
                ));
            }

            if !setup.flows.is_empty() {
                sections.push(format!(
                    "### Sample Flows\n\n{} sample flows demonstrating common workflows.",
                    setup.flows.len()
                ));
            }

            if setup.playground.as_ref().map(|p| p.enabled).unwrap_or(false) {
                sections.push(
                    "### Playground Collection\n\nA playground collection for testing endpoints."
                        .to_string(),
                );
            }

            sections.join("\n\n")
        } else {
            "This blueprint provides a complete setup for your use case.".to_string()
        }
    )
}
