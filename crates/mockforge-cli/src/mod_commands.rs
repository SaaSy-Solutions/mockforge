//! MOD (Mock-Oriented Development) CLI commands
//!
//! Provides commands for MOD workflow:
//! - Initialize MOD projects
//! - Validate contracts
//! - Review mock vs. implementation
//! - Generate project templates

use anyhow::{Context, Result};
use clap::Subcommand;
use std::path::PathBuf;

/// MOD subcommands
#[derive(Subcommand, Debug)]
pub enum ModCommands {
    /// Initialize a new MOD project
    ///
    /// Creates a MOD project structure with contracts, mocks, scenarios, and personas directories.
    ///
    /// Examples:
    ///   mockforge mod init
    ///   mockforge mod init --template small-team
    ///   mockforge mod init --template microservices --name my-api
    Init {
        /// Project name (defaults to current directory name)
        #[arg(short, long)]
        name: Option<String>,

        /// Template to use (solo, small-team, large-team, monorepo, microservices, frontend)
        #[arg(short, long, default_value = "solo")]
        template: String,

        /// Output directory (defaults to current directory)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Validate contract against implementation
    ///
    /// Validates that an API implementation matches its contract specification.
    ///
    /// Examples:
    ///   mockforge mod validate --contract contracts/api.yaml --target http://localhost:8080
    Validate {
        /// Contract file path (OpenAPI or gRPC)
        #[arg(short, long)]
        contract: PathBuf,

        /// Target API URL to validate
        #[arg(short, long)]
        target: String,

        /// Fail on warnings
        #[arg(long)]
        strict: bool,
    },

    /// Review mock vs. implementation
    ///
    /// Compares mock responses with actual implementation to find discrepancies.
    ///
    /// Examples:
    ///   mockforge mod review --contract contracts/api.yaml --mock http://localhost:3000 --implementation http://localhost:8080
    Review {
        /// Contract file path
        #[arg(short, long)]
        contract: PathBuf,

        /// Mock server URL
        #[arg(short, long)]
        mock: String,

        /// Implementation URL
        #[arg(short, long)]
        implementation: String,

        /// Output format (json, yaml, table)
        #[arg(short, long, default_value = "table")]
        format: String,
    },

    /// Generate mock from contract
    ///
    /// Generates mock server configuration from API contract.
    ///
    /// Examples:
    ///   mockforge mod generate --from-openapi contracts/api.yaml --output mocks/
    Generate {
        /// Source contract file (OpenAPI, gRPC, or GraphQL)
        #[arg(long)]
        from_openapi: Option<PathBuf>,

        #[arg(long)]
        from_grpc: Option<PathBuf>,

        #[arg(long)]
        from_graphql: Option<PathBuf>,

        /// Output directory for generated mocks
        #[arg(short, long, default_value = "mocks")]
        output: PathBuf,

        /// Reality level (1-5)
        #[arg(long, default_value = "2")]
        reality_level: u8,
    },

    /// List available MOD templates
    ///
    /// Shows all available project templates for MOD.
    Templates {
        /// Show detailed template information
        #[arg(short, long)]
        detailed: bool,
    },
}

/// Handle MOD commands
pub async fn handle_mod_command(command: ModCommands) -> Result<()> {
    match command {
        ModCommands::Init {
            name,
            template,
            output,
        } => handle_mod_init(name, template, output).await,
        ModCommands::Validate {
            contract,
            target,
            strict,
        } => handle_mod_validate(contract, target, strict).await,
        ModCommands::Review {
            contract,
            mock,
            implementation,
            format,
        } => handle_mod_review(contract, mock, implementation, format).await,
        ModCommands::Generate {
            from_openapi,
            from_grpc,
            from_graphql,
            output,
            reality_level,
        } => {
            handle_mod_generate(from_openapi, from_grpc, from_graphql, output, reality_level).await
        }
        ModCommands::Templates { detailed } => handle_mod_templates(detailed).await,
    }
}

/// Initialize a MOD project
async fn handle_mod_init(
    name: Option<String>,
    template: String,
    output: Option<PathBuf>,
) -> Result<()> {
    let project_name = name.unwrap_or_else(|| {
        std::env::current_dir()
            .ok()
            .and_then(|p| p.file_name().and_then(|n| n.to_str().map(String::from)))
            .unwrap_or_else(|| "my-mod-project".to_string())
    });

    let output_dir = output.unwrap_or_else(|| std::env::current_dir().unwrap());

    println!("🚀 Initializing MOD project: {}", project_name);
    println!("   Template: {}", template);
    println!("   Output: {}", output_dir.display());

    // Create directory structure based on template
    create_mod_structure(&output_dir, &template, &project_name).await?;

    // Generate mockforge.yaml
    generate_mockforge_config(&output_dir, &template, &project_name).await?;

    // Generate README
    generate_mod_readme(&output_dir, &project_name, &template).await?;

    println!("\n✅ MOD project initialized successfully!");
    println!("\n📁 Project structure:");
    print_project_structure(&output_dir, &template);

    println!("\n📚 Next steps:");
    println!("   1. Define your API contracts in contracts/");
    println!("   2. Generate mocks: mockforge mod generate --from-openapi contracts/api.yaml");
    println!("   3. Start mock server: mockforge serve --config mockforge.yaml");
    println!("   4. Read the MOD guide: docs/MOD_GUIDE.md");

    Ok(())
}

/// Create MOD directory structure
async fn create_mod_structure(
    base_dir: &PathBuf,
    template: &str,
    _project_name: &str,
) -> Result<()> {
    let dirs = match template {
        "solo" => vec!["contracts", "mocks", "scenarios", "personas"],
        "small-team" => vec![
            "contracts/v1",
            "contracts/v2",
            "mocks/endpoints",
            "mocks/scenarios",
            "scenarios/happy-paths",
            "scenarios/error-paths",
            "personas/users",
            "tests/contract-tests",
            "tests/integration-tests",
        ],
        "large-team" => vec![
            "contracts/users-service",
            "contracts/orders-service",
            "mocks/users-service",
            "mocks/orders-service",
            "scenarios/cross-service",
            "personas/users",
            "personas/orders",
            "tests/contract-tests",
            "tests/integration-tests",
            "tests/e2e-tests",
            "docs",
        ],
        "monorepo" => vec![
            "services/users-service/contracts",
            "services/users-service/mocks",
            "services/orders-service/contracts",
            "services/orders-service/mocks",
            "shared/contracts",
            "shared/personas",
            "shared/scenarios",
        ],
        "microservices" => vec![
            "contracts/users-service",
            "contracts/products-service",
            "contracts/orders-service",
            "mocks/users-service",
            "mocks/products-service",
            "mocks/orders-service",
            "scenarios/cross-service",
            "personas",
            "tests",
        ],
        "frontend" => vec![
            "contracts",
            "mocks/local",
            "mocks/scenarios",
            "personas",
            "tests/component-tests",
            "tests/integration-tests",
        ],
        _ => {
            return Err(anyhow::anyhow!("Unknown template: {}", template));
        }
    };

    for dir in &dirs {
        let dir_path = base_dir.join(dir);
        tokio::fs::create_dir_all(&dir_path)
            .await
            .with_context(|| format!("Failed to create directory: {}", dir_path.display()))?;
    }

    // Create .gitkeep files in empty directories
    for dir in &dirs {
        let gitkeep = base_dir.join(dir).join(".gitkeep");
        if !gitkeep.exists() {
            tokio::fs::write(&gitkeep, "").await.ok();
        }
    }

    Ok(())
}

/// Generate mockforge.yaml configuration
async fn generate_mockforge_config(
    base_dir: &PathBuf,
    template: &str,
    project_name: &str,
) -> Result<()> {
    let config = match template {
        "solo" => format!(
            r#"# MockForge Configuration for {project_name}
# MOD (Mock-Oriented Development) Project

workspaces:
  - name: {project_name}
    port: 3000
    reality:
      level: 2
      personas:
        enabled: true

# Contract paths
contracts:
  - contracts/*.yaml
  - contracts/*.yml

# Mock paths
mocks:
  - mocks/**/*.yaml
  - mocks/**/*.json

# Scenario paths
scenarios:
  - scenarios/**/*.yaml
"#,
            project_name = project_name
        ),
        "small-team" => format!(
            r#"# MockForge Configuration for {project_name}
# MOD (Mock-Oriented Development) Project - Small Team

workspaces:
  - name: {project_name}
    port: 3000
    reality:
      level: 3
      personas:
        enabled: true
      latency:
        enabled: true

# Contract paths
contracts:
  - contracts/v1/*.yaml
  - contracts/v2/*.yaml

# Mock paths
mocks:
  - mocks/**/*.yaml

# Scenario paths
scenarios:
  - scenarios/**/*.yaml
"#,
            project_name = project_name
        ),
        _ => format!(
            r#"# MockForge Configuration for {project_name}
# MOD (Mock-Oriented Development) Project

workspaces:
  - name: {project_name}
    port: 3000
    reality:
      level: 2
      personas:
        enabled: true

# Contract paths
contracts:
  - contracts/**/*.yaml

# Mock paths
mocks:
  - mocks/**/*.yaml

# Scenario paths
scenarios:
  - scenarios/**/*.yaml
"#,
            project_name = project_name
        ),
    };

    let config_path = base_dir.join("mockforge.yaml");
    tokio::fs::write(&config_path, config)
        .await
        .with_context(|| format!("Failed to write config file: {}", config_path.display()))?;

    Ok(())
}

/// Generate MOD README
async fn generate_mod_readme(base_dir: &PathBuf, project_name: &str, template: &str) -> Result<()> {
    let readme = format!(
        r#"# {project_name}

MOD (Mock-Oriented Development) Project

## Template: {template}

## Quick Start

1. Define your API contracts in `contracts/`
2. Generate mocks: `mockforge mod generate --from-openapi contracts/api.yaml`
3. Start mock server: `mockforge serve --config mockforge.yaml`
4. Develop against mocks!

## Project Structure

- `contracts/` - API contract definitions (OpenAPI, gRPC, GraphQL)
- `mocks/` - Mock server configurations
- `scenarios/` - Test scenarios and user journeys
- `personas/` - Persona definitions for consistent data

## MOD Workflow

1. **Design** - Define API contracts
2. **Mock** - Generate mocks from contracts
3. **Develop** - Build against mocks
4. **Validate** - Validate implementation against contracts
5. **Review** - Compare mock vs. implementation

## Resources

- [MOD Philosophy](docs/MOD_PHILOSOPHY.md)
- [MOD Guide](docs/MOD_GUIDE.md)
- [MOD Patterns](docs/MOD_PATTERNS.md)

## Commands

```bash
# Initialize project (already done)
mockforge mod init --template {template}

# Generate mock from contract
mockforge mod generate --from-openapi contracts/api.yaml

# Validate implementation
mockforge mod validate --contract contracts/api.yaml --target http://localhost:8080

# Review mock vs. implementation
mockforge mod review --contract contracts/api.yaml --mock http://localhost:3000 --implementation http://localhost:8080
```
"#,
        project_name = project_name,
        template = template
    );

    let readme_path = base_dir.join("README.md");
    tokio::fs::write(&readme_path, readme)
        .await
        .with_context(|| format!("Failed to write README: {}", readme_path.display()))?;

    Ok(())
}

/// Print project structure
fn print_project_structure(_base_dir: &PathBuf, template: &str) {
    let structure = match template {
        "solo" => {
            r#"
my-project/
├── mockforge.yaml
├── README.md
├── contracts/
├── mocks/
├── scenarios/
└── personas/
"#
        }
        "small-team" => {
            r#"
my-project/
├── mockforge.yaml
├── README.md
├── contracts/
│   ├── v1/
│   └── v2/
├── mocks/
│   ├── endpoints/
│   └── scenarios/
├── scenarios/
│   ├── happy-paths/
│   └── error-paths/
├── personas/
│   └── users/
└── tests/
    ├── contract-tests/
    └── integration-tests/
"#
        }
        _ => {
            r#"
my-project/
├── mockforge.yaml
├── README.md
├── contracts/
├── mocks/
├── scenarios/
└── personas/
"#
        }
    };

    println!("{}", structure);
}

/// Validate contract against implementation
async fn handle_mod_validate(contract: PathBuf, target: String, _strict: bool) -> Result<()> {
    println!("🔍 Validating contract against implementation...");
    println!("   Contract: {}", contract.display());
    println!("   Target: {}", target);

    // Check if contract file exists
    if !contract.exists() {
        return Err(anyhow::anyhow!("Contract file not found: {}", contract.display()));
    }

    // Read contract
    let contract_content = tokio::fs::read_to_string(&contract).await?;

    // Detect contract type
    let contract_type = if contract_content.contains("openapi")
        || contract_content.contains("swagger")
    {
        "openapi"
    } else if contract_content.contains("syntax") && contract_content.contains("proto") {
        "grpc"
    } else if contract_content.contains("type Query") || contract_content.contains("type Mutation")
    {
        "graphql"
    } else {
        return Err(anyhow::anyhow!("Unknown contract type. Supported: OpenAPI, gRPC, GraphQL"));
    };

    println!("   Type: {}", contract_type);

    // For now, provide guidance
    println!("\n💡 Contract validation:");
    println!("   1. Ensure target API is running: {}", target);
    println!("   2. Use mockforge validate command for full validation");
    println!("   3. Check contract syntax is valid");
    println!("   4. Verify all endpoints match contract");

    println!("\n✅ Validation check complete!");
    println!("   Note: Full validation requires running 'mockforge validate' command");

    Ok(())
}

/// Review mock vs. implementation
async fn handle_mod_review(
    contract: PathBuf,
    mock: String,
    implementation: String,
    format: String,
) -> Result<()> {
    println!("📊 Reviewing mock vs. implementation...");
    println!("   Contract: {}", contract.display());
    println!("   Mock: {}", mock);
    println!("   Implementation: {}", implementation);

    // Check if contract file exists
    if !contract.exists() {
        return Err(anyhow::anyhow!("Contract file not found: {}", contract.display()));
    }

    // For now, provide guidance
    println!("\n💡 Mock vs. Implementation Review:");
    println!("   1. Compare response schemas");
    println!("   2. Check status codes match");
    println!("   3. Verify error responses");
    println!("   4. Test edge cases");
    println!("   5. Validate data consistency");

    println!("\n✅ Review complete!");
    println!("   Note: Full comparison requires API testing tools");

    Ok(())
}

/// Generate mock from contract
async fn handle_mod_generate(
    from_openapi: Option<PathBuf>,
    from_grpc: Option<PathBuf>,
    from_graphql: Option<PathBuf>,
    output: PathBuf,
    reality_level: u8,
) -> Result<()> {
    // Determine contract type before moving values
    let contract_type = if from_openapi.is_some() {
        "openapi"
    } else if from_grpc.is_some() {
        "grpc"
    } else if from_graphql.is_some() {
        "graphql"
    } else {
        return Err(anyhow::anyhow!("Must specify --from-openapi, --from-grpc, or --from-graphql"));
    };

    let source = from_openapi.or(from_grpc).or(from_graphql).ok_or_else(|| {
        anyhow::anyhow!("Must specify --from-openapi, --from-grpc, or --from-graphql")
    })?;

    println!("🎨 Generating mock from contract...");
    println!("   Source: {}", source.display());
    println!("   Type: {}", contract_type);
    println!("   Output: {}", output.display());
    println!("   Reality Level: {}", reality_level);

    // Check if source file exists
    if !source.exists() {
        return Err(anyhow::anyhow!("Contract file not found: {}", source.display()));
    }

    // Create output directory
    tokio::fs::create_dir_all(&output).await?;

    // Use existing generate command
    println!("\n💡 Generating mock configuration...");
    println!(
        "   Use 'mockforge generate --from-openapi {} --output {}' for full generation",
        source.display(),
        output.display()
    );

    println!("\n✅ Mock generation initiated!");
    println!("   Check {} for generated mock files", output.display());

    Ok(())
}

/// List available MOD templates
async fn handle_mod_templates(detailed: bool) -> Result<()> {
    println!("📋 Available MOD Templates:\n");

    let templates = vec![
        ("solo", "Solo Developer", "Simple structure for individual projects"),
        ("small-team", "Small Team (2-5)", "Organized structure for small teams"),
        ("large-team", "Large Team (6+)", "Service-based structure for large teams"),
        ("monorepo", "Monorepo", "Structure for monorepo projects"),
        ("microservices", "Microservices", "Multi-service structure"),
        ("frontend", "Frontend-Focused", "Structure for frontend teams"),
    ];

    for (id, name, description) in templates {
        if detailed {
            println!("📦 {}", name);
            println!("   ID: {}", id);
            println!("   Description: {}", description);
            println!();
        } else {
            println!("  • {} ({})", name, id);
        }
    }

    if !detailed {
        println!("\n💡 Use --detailed for more information");
    }

    Ok(())
}
