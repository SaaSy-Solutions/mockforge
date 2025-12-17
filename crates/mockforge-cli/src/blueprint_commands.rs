//! Blueprint management CLI commands
//!
//! Blueprints are predefined app archetypes that provide:
//! - Pre-configured personas
//! - Reality defaults optimized for the use case
//! - Sample flows demonstrating common workflows
//! - Playground collections for testing

use clap::Subcommand;
use serde::{Deserialize, Serialize};
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
    #[serde(default)]
    pub contracts: Vec<ContractInfo>,
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
    pub scenarios: Vec<ScenarioInfo>,
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

/// Scenario information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioInfo {
    pub id: String,
    pub name: String,
    pub r#type: String, // happy_path, known_failure, slow_path
    #[serde(default)]
    pub description: Option<String>,
    pub file: String,
}

/// Contract information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractInfo {
    pub file: String,
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

        if !setup.scenarios.is_empty() {
            println!("\nScenarios:");
            for scenario in &setup.scenarios {
                println!("  • {} ({}) - {}", scenario.id, scenario.r#type, scenario.name);
                if let Some(desc) = &scenario.description {
                    println!("    {}", desc);
                }
            }
        }
    }

    if !metadata.contracts.is_empty() {
        println!("\nContract Schemas:");
        for contract in &metadata.contracts {
            println!("  • {}", contract.file);
            if let Some(desc) = &contract.description {
                println!("    {}", desc);
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

    // Copy scenarios directory
    let scenarios_src = blueprint_path.join("scenarios");
    if scenarios_src.exists() {
        let scenarios_dst = output_dir.join("scenarios");
        copy_directory(&scenarios_src, &scenarios_dst)?;
        println!("  ✓ Copied scenarios/");
    }

    // Copy contracts directory
    let contracts_src = blueprint_path.join("contracts");
    if contracts_src.exists() {
        let contracts_dst = output_dir.join("contracts");
        copy_directory(&contracts_src, &contracts_dst)?;
        println!("  ✓ Copied contracts/");
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

#[cfg(test)]
mod tests {
    use super::*;

    fn create_minimal_metadata() -> BlueprintMetadata {
        BlueprintMetadata {
            manifest_version: "1.0".to_string(),
            name: "test-blueprint".to_string(),
            version: "1.0.0".to_string(),
            title: "Test Blueprint".to_string(),
            description: "A test blueprint for testing".to_string(),
            author: "Test Author".to_string(),
            author_email: None,
            category: "testing".to_string(),
            tags: vec![],
            setup: None,
            compatibility: None,
            files: vec![],
            readme: None,
            contracts: vec![],
        }
    }

    fn create_full_metadata() -> BlueprintMetadata {
        BlueprintMetadata {
            manifest_version: "1.0".to_string(),
            name: "full-blueprint".to_string(),
            version: "2.0.0".to_string(),
            title: "Full Blueprint".to_string(),
            description: "A fully configured blueprint".to_string(),
            author: "Full Author".to_string(),
            author_email: Some("author@example.com".to_string()),
            category: "e-commerce".to_string(),
            tags: vec!["api".to_string(), "mock".to_string()],
            setup: Some(BlueprintSetup {
                personas: vec![PersonaInfo {
                    id: "admin".to_string(),
                    name: "Admin User".to_string(),
                    description: Some("Administrator persona".to_string()),
                }],
                reality: Some(RealityInfo {
                    level: "standard".to_string(),
                    description: Some("Standard reality level".to_string()),
                }),
                flows: vec![FlowInfo {
                    id: "checkout".to_string(),
                    name: "Checkout Flow".to_string(),
                    description: Some("Complete checkout process".to_string()),
                }],
                scenarios: vec![ScenarioInfo {
                    id: "happy".to_string(),
                    name: "Happy Path".to_string(),
                    r#type: "happy_path".to_string(),
                    description: Some("Normal flow".to_string()),
                    file: "happy.yaml".to_string(),
                }],
                playground: Some(PlaygroundInfo {
                    enabled: true,
                    collection_file: "playground.json".to_string(),
                }),
            }),
            compatibility: Some(BlueprintCompatibility {
                min_version: "0.3.0".to_string(),
                max_version: Some("1.0.0".to_string()),
                required_features: vec!["vbr".to_string()],
                protocols: vec!["http".to_string(), "grpc".to_string()],
            }),
            files: vec!["config.yaml".to_string()],
            readme: Some("README.md".to_string()),
            contracts: vec![ContractInfo {
                file: "schema.json".to_string(),
                description: Some("Main API schema".to_string()),
            }],
        }
    }

    #[test]
    fn test_default_true() {
        assert!(default_true());
    }

    #[test]
    fn test_default_min_version() {
        assert_eq!(default_min_version(), "0.3.0");
    }

    #[test]
    fn test_get_blueprints_dir() {
        let dir = get_blueprints_dir();
        assert_eq!(dir, PathBuf::from("blueprints"));
    }

    #[test]
    fn test_blueprint_metadata_serialization() {
        let metadata = create_minimal_metadata();
        let json = serde_json::to_string(&metadata).unwrap();

        assert!(json.contains("\"name\":\"test-blueprint\""));
        assert!(json.contains("\"version\":\"1.0.0\""));
        assert!(json.contains("\"category\":\"testing\""));
    }

    #[test]
    fn test_blueprint_metadata_deserialization() {
        let yaml = r#"
manifest_version: "1.0"
name: "test-blueprint"
version: "1.0.0"
title: "Test Blueprint"
description: "A test blueprint"
author: "Test Author"
category: "testing"
"#;
        let metadata: BlueprintMetadata = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(metadata.name, "test-blueprint");
        assert_eq!(metadata.version, "1.0.0");
        assert_eq!(metadata.category, "testing");
        assert!(metadata.tags.is_empty()); // default
    }

    #[test]
    fn test_blueprint_metadata_full_deserialization() {
        let yaml = r#"
manifest_version: "1.0"
name: "full-blueprint"
version: "2.0.0"
title: "Full Blueprint"
description: "Full description"
author: "Author"
author_email: "author@example.com"
category: "e-commerce"
tags:
  - api
  - mock
setup:
  personas:
    - id: admin
      name: Admin User
      description: Administrator
  reality:
    level: standard
  flows:
    - id: checkout
      name: Checkout Flow
  scenarios:
    - id: happy
      name: Happy Path
      type: happy_path
      file: happy.yaml
  playground:
    enabled: true
    collection_file: playground.json
compatibility:
  min_version: "0.3.0"
  max_version: "1.0.0"
  required_features:
    - vbr
  protocols:
    - http
files:
  - config.yaml
contracts:
  - file: schema.json
    description: API Schema
"#;
        let metadata: BlueprintMetadata = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(metadata.name, "full-blueprint");
        assert_eq!(metadata.author_email, Some("author@example.com".to_string()));
        assert_eq!(metadata.tags.len(), 2);

        let setup = metadata.setup.unwrap();
        assert_eq!(setup.personas.len(), 1);
        assert_eq!(setup.personas[0].id, "admin");
        assert!(setup.reality.is_some());
        assert_eq!(setup.flows.len(), 1);
        assert_eq!(setup.scenarios.len(), 1);
        assert!(setup.playground.is_some());

        let compat = metadata.compatibility.unwrap();
        assert_eq!(compat.min_version, "0.3.0");
        assert_eq!(compat.max_version, Some("1.0.0".to_string()));
    }

    #[test]
    fn test_persona_info_serialization() {
        let persona = PersonaInfo {
            id: "user".to_string(),
            name: "Regular User".to_string(),
            description: Some("A regular user persona".to_string()),
        };

        let json = serde_json::to_string(&persona).unwrap();
        assert!(json.contains("\"id\":\"user\""));
        assert!(json.contains("\"name\":\"Regular User\""));
        assert!(json.contains("\"description\":\"A regular user persona\""));
    }

    #[test]
    fn test_persona_info_clone() {
        let persona = PersonaInfo {
            id: "user".to_string(),
            name: "Regular User".to_string(),
            description: None,
        };

        let cloned = persona.clone();
        assert_eq!(persona.id, cloned.id);
        assert_eq!(persona.name, cloned.name);
    }

    #[test]
    fn test_reality_info() {
        let reality = RealityInfo {
            level: "high".to_string(),
            description: Some("High fidelity mode".to_string()),
        };

        let json = serde_json::to_string(&reality).unwrap();
        let parsed: RealityInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.level, "high");
        assert_eq!(parsed.description, Some("High fidelity mode".to_string()));
    }

    #[test]
    fn test_flow_info() {
        let flow = FlowInfo {
            id: "login".to_string(),
            name: "Login Flow".to_string(),
            description: None,
        };

        let json = serde_json::to_string(&flow).unwrap();
        assert!(json.contains("\"id\":\"login\""));
        assert!(json.contains("\"name\":\"Login Flow\""));
    }

    #[test]
    fn test_scenario_info() {
        let scenario = ScenarioInfo {
            id: "error".to_string(),
            name: "Error Scenario".to_string(),
            r#type: "known_failure".to_string(),
            description: Some("Tests error handling".to_string()),
            file: "error_scenario.yaml".to_string(),
        };

        let json = serde_json::to_string(&scenario).unwrap();
        assert!(json.contains("\"id\":\"error\""));
        assert!(json.contains("\"type\":\"known_failure\""));
        assert!(json.contains("\"file\":\"error_scenario.yaml\""));
    }

    #[test]
    fn test_contract_info() {
        let contract = ContractInfo {
            file: "openapi.yaml".to_string(),
            description: Some("OpenAPI specification".to_string()),
        };

        let json = serde_json::to_string(&contract).unwrap();
        let parsed: ContractInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.file, "openapi.yaml");
        assert_eq!(parsed.description, Some("OpenAPI specification".to_string()));
    }

    #[test]
    fn test_playground_info_with_default() {
        let yaml = r#"collection_file: "test.json""#;
        let playground: PlaygroundInfo = serde_yaml::from_str(yaml).unwrap();

        assert!(playground.enabled); // default_true
        assert_eq!(playground.collection_file, "test.json");
    }

    #[test]
    fn test_playground_info_explicit_disabled() {
        let yaml = r#"
enabled: false
collection_file: "test.json"
"#;
        let playground: PlaygroundInfo = serde_yaml::from_str(yaml).unwrap();

        assert!(!playground.enabled);
    }

    #[test]
    fn test_blueprint_compatibility_defaults() {
        let yaml = r#"{}"#;
        let compat: BlueprintCompatibility = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(compat.min_version, "0.3.0"); // default_min_version
        assert!(compat.max_version.is_none());
        assert!(compat.required_features.is_empty());
        assert!(compat.protocols.is_empty());
    }

    #[test]
    fn test_blueprint_setup_default() {
        let yaml = r#"{}"#;
        let setup: BlueprintSetup = serde_yaml::from_str(yaml).unwrap();

        assert!(setup.personas.is_empty());
        assert!(setup.reality.is_none());
        assert!(setup.flows.is_empty());
        assert!(setup.scenarios.is_empty());
        assert!(setup.playground.is_none());
    }

    #[test]
    fn test_generate_readme_minimal() {
        let metadata = create_minimal_metadata();
        let readme = generate_readme("MyProject", &metadata);

        assert!(readme.contains("# MyProject"));
        assert!(readme.contains("A test blueprint for testing"));
        assert!(readme.contains("**Test Blueprint** blueprint"));
        assert!(readme.contains("mockforge serve"));
    }

    #[test]
    fn test_generate_readme_with_setup() {
        let metadata = create_full_metadata();
        let readme = generate_readme("FullProject", &metadata);

        assert!(readme.contains("# FullProject"));
        assert!(readme.contains("### Personas"));
        assert!(readme.contains("1 predefined personas"));
        assert!(readme.contains("### Sample Flows"));
        assert!(readme.contains("### Playground Collection"));
    }

    #[test]
    fn test_generate_readme_without_playground() {
        let mut metadata = create_full_metadata();
        if let Some(ref mut setup) = metadata.setup {
            setup.playground = None;
        }

        let readme = generate_readme("TestProject", &metadata);

        assert!(readme.contains("### Personas"));
        assert!(!readme.contains("### Playground Collection"));
    }

    #[test]
    fn test_blueprint_metadata_debug() {
        let metadata = create_minimal_metadata();
        let debug_str = format!("{:?}", metadata);

        assert!(debug_str.contains("BlueprintMetadata"));
        assert!(debug_str.contains("test-blueprint"));
    }

    #[test]
    fn test_blueprint_commands_enum_variants() {
        // Test that all variants can be constructed
        let _list = BlueprintCommands::List {
            detailed: true,
            category: Some("testing".to_string()),
        };

        let _create = BlueprintCommands::Create {
            name: "test".to_string(),
            blueprint: "test-blueprint".to_string(),
            output: Some(PathBuf::from("output")),
            force: true,
        };

        let _info = BlueprintCommands::Info {
            blueprint_id: "test".to_string(),
        };
    }

    #[test]
    fn test_metadata_round_trip_yaml() {
        let original = create_full_metadata();
        let yaml = serde_yaml::to_string(&original).unwrap();
        let parsed: BlueprintMetadata = serde_yaml::from_str(&yaml).unwrap();

        assert_eq!(original.name, parsed.name);
        assert_eq!(original.version, parsed.version);
        assert_eq!(original.author_email, parsed.author_email);
        assert_eq!(original.tags, parsed.tags);
    }

    #[test]
    fn test_metadata_round_trip_json() {
        let original = create_minimal_metadata();
        let json = serde_json::to_string(&original).unwrap();
        let parsed: BlueprintMetadata = serde_json::from_str(&json).unwrap();

        assert_eq!(original.name, parsed.name);
        assert_eq!(original.version, parsed.version);
        assert_eq!(original.category, parsed.category);
    }

    #[test]
    fn test_all_structs_implement_clone() {
        let metadata = create_full_metadata();
        let cloned = metadata.clone();
        assert_eq!(metadata.name, cloned.name);

        let setup = metadata.setup.clone().unwrap();
        let setup_cloned = setup.clone();
        assert_eq!(setup.personas.len(), setup_cloned.personas.len());

        if let Some(compat) = metadata.compatibility.clone() {
            let compat_cloned = compat.clone();
            assert_eq!(compat.min_version, compat_cloned.min_version);
        }
    }

    #[test]
    fn test_scenario_types() {
        let types = ["happy_path", "known_failure", "slow_path", "edge_case"];

        for scenario_type in types {
            let scenario = ScenarioInfo {
                id: "test".to_string(),
                name: "Test Scenario".to_string(),
                r#type: scenario_type.to_string(),
                description: None,
                file: "test.yaml".to_string(),
            };

            let json = serde_json::to_string(&scenario).unwrap();
            assert!(json.contains(&format!("\"type\":\"{}\"", scenario_type)));
        }
    }

    #[test]
    fn test_metadata_with_empty_optional_arrays() {
        let yaml = r#"
manifest_version: "1.0"
name: "minimal"
version: "1.0.0"
title: "Minimal"
description: "Minimal blueprint"
author: "Author"
category: "test"
tags: []
files: []
contracts: []
"#;
        let metadata: BlueprintMetadata = serde_yaml::from_str(yaml).unwrap();

        assert!(metadata.tags.is_empty());
        assert!(metadata.files.is_empty());
        assert!(metadata.contracts.is_empty());
    }
}
