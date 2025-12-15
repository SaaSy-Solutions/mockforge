//! Scenario management CLI commands

use base64::{engine::general_purpose, Engine as _};
use clap::Subcommand;
use mockforge_core::behavioral_economics::BehaviorRule;
use mockforge_core::config::{load_config, save_config, ServerConfig};
use mockforge_scenarios::{
    DomainPackInstaller, InstallOptions, ScenarioInstaller, ScenarioRegistry,
    ScenarioReviewSubmission,
};
use mockforge_vbr::entities::Entity;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::Path;

#[derive(Subcommand)]
pub enum ScenarioCommands {
    /// Install a scenario from various sources
    Install {
        /// Scenario source (URL, Git repo, local path, or registry name)
        ///
        /// Examples:
        /// - ./scenarios/my-scenario
        /// - https://github.com/user/repo#main:scenarios/my-scenario
        /// - https://example.com/scenario.zip
        /// - ecommerce-store@1.0.0
        source: String,

        /// Force reinstall even if scenario exists
        #[arg(long)]
        force: bool,

        /// Skip validation checks
        #[arg(long)]
        skip_validation: bool,

        /// Expected SHA-256 checksum (for URL sources)
        #[arg(long)]
        checksum: Option<String>,
    },

    /// Uninstall a scenario
    Uninstall {
        /// Scenario name
        name: String,

        /// Scenario version (optional, defaults to latest)
        #[arg(long)]
        version: Option<String>,
    },

    /// List installed scenarios
    List {
        /// Show detailed information
        #[arg(short, long)]
        detailed: bool,
    },

    /// Show scenario information
    Info {
        /// Scenario name
        name: String,

        /// Scenario version (optional, defaults to latest)
        #[arg(long)]
        version: Option<String>,
    },

    /// Preview a scenario before installing
    Preview {
        /// Scenario source (URL, Git repo, local path, or registry name)
        ///
        /// Examples:
        /// - ./scenarios/my-scenario
        /// - https://github.com/user/repo#main:scenarios/my-scenario
        /// - https://example.com/scenario.zip
        /// - ecommerce-store@1.0.0
        source: String,
    },

    /// Apply scenario to current workspace
    Use {
        /// Scenario name
        name: String,

        /// Scenario version (optional, defaults to latest)
        #[arg(long)]
        version: Option<String>,

        /// Merge strategy for schema alignment (prefer-existing, prefer-scenario, intelligent, interactive)
        #[arg(long, default_value = "prefer-existing")]
        merge_strategy: String,

        /// Enable automatic schema alignment
        #[arg(long)]
        auto_align: bool,
    },

    /// Search for scenarios in the registry
    Search {
        /// Search query
        query: String,

        /// Category filter
        #[arg(long)]
        category: Option<String>,

        /// Maximum number of results
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },

    /// Publish a scenario to the registry
    Publish {
        /// Path to scenario directory
        path: String,

        /// Registry URL (optional)
        #[arg(long)]
        registry: Option<String>,
    },

    /// Update a scenario to the latest version
    Update {
        /// Scenario name to update (or --all for all scenarios)
        name: Option<String>,

        /// Update all installed scenarios
        #[arg(long)]
        all: bool,
    },

    /// Domain pack commands
    Pack {
        /// Pack subcommand
        #[command(subcommand)]
        command: PackCommands,
    },

    /// Review commands
    Review {
        /// Review subcommand
        #[command(subcommand)]
        command: ReviewCommands,
    },
}

#[derive(Subcommand)]
pub enum PackCommands {
    /// List installed domain packs
    List,

    /// Install a domain pack from a manifest file
    Install {
        /// Path to pack manifest file (pack.yaml or pack.json)
        manifest: String,
    },

    /// Show information about a domain pack
    Info {
        /// Pack name
        name: String,
    },

    /// Studio pack commands (full studio packs with personas, chaos rules, etc.)
    Studio {
        /// Studio pack subcommand
        #[command(subcommand)]
        command: StudioPackCommands,
    },
}

#[derive(Subcommand)]
pub enum StudioPackCommands {
    /// Install a studio pack (applies personas, chaos rules, contract diffs, reality blends)
    Install {
        /// Studio pack name (e.g., "fintech-fraud-lab", "ecommerce-peak-day", "healthcare-outage-drill")
        /// or path to pack manifest file
        pack_name: String,

        /// Workspace ID to install the pack to (defaults to "default")
        #[arg(short, long, default_value = "default")]
        workspace: String,
    },

    /// List available studio packs
    List,

    /// Create a new studio pack from the current workspace configuration
    Create {
        /// Name for the new studio pack
        name: String,

        /// Output path for the pack manifest (defaults to ./{name}-pack.yaml)
        #[arg(short, long)]
        output: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum ReviewCommands {
    /// Submit a review for a scenario
    Submit {
        /// Scenario name
        scenario_name: String,

        /// Scenario version (optional)
        #[arg(long)]
        scenario_version: Option<String>,

        /// Rating (1-5)
        #[arg(short, long)]
        rating: u8,

        /// Review title (optional)
        #[arg(long)]
        title: Option<String>,

        /// Review comment/text
        #[arg(short, long)]
        comment: String,

        /// Reviewer name/username
        #[arg(long)]
        reviewer: String,

        /// Reviewer email (optional)
        #[arg(long)]
        reviewer_email: Option<String>,

        /// Mark as verified purchase (reviewer used the scenario)
        #[arg(long)]
        verified: bool,
    },

    /// List reviews for a scenario
    List {
        /// Scenario name
        scenario_name: String,

        /// Page number (0-indexed)
        #[arg(long, default_value = "0")]
        page: Option<usize>,

        /// Results per page
        #[arg(long, default_value = "20")]
        per_page: Option<usize>,
    },
}

/// Handle scenario commands
pub async fn handle_scenario_command(command: ScenarioCommands) -> anyhow::Result<()> {
    match command {
        ScenarioCommands::Install {
            source,
            force,
            skip_validation,
            checksum,
        } => {
            println!("📦 Installing scenario from: {}", source);

            let mut installer = ScenarioInstaller::new()?;
            installer.init().await?;

            let options = InstallOptions {
                force,
                skip_validation,
                expected_checksum: checksum,
            };

            match installer.install(&source, options).await {
                Ok(scenario_id) => {
                    println!("✅ Scenario installed successfully: {}", scenario_id);
                }
                Err(e) => {
                    eprintln!("❌ Failed to install scenario: {}", e);
                    std::process::exit(1);
                }
            }
        }

        ScenarioCommands::Uninstall { name, version } => {
            let version = version.unwrap_or_else(|| "latest".to_string());
            println!("🗑️  Uninstalling scenario: {}@{}", name, version);

            let mut installer = ScenarioInstaller::new()?;
            installer.init().await?;

            // If version is "latest", get the latest version
            let actual_version = if version == "latest" {
                installer
                    .get_latest(&name)
                    .map(|s| s.version.clone())
                    .ok_or_else(|| anyhow::anyhow!("Scenario '{}' not found", name))?
            } else {
                version
            };

            match installer.uninstall(&name, &actual_version).await {
                Ok(_) => {
                    println!("✅ Scenario uninstalled successfully");
                }
                Err(e) => {
                    eprintln!("❌ Failed to uninstall scenario: {}", e);
                    std::process::exit(1);
                }
            }
        }

        ScenarioCommands::List { detailed } => {
            println!("📋 Installed scenarios:");

            let mut installer = ScenarioInstaller::new()?;
            installer.init().await?;

            let scenarios = installer.list_installed();

            if scenarios.is_empty() {
                println!("  No scenarios installed");
            } else {
                for scenario in scenarios {
                    if detailed {
                        println!("  - {}@{}", scenario.name, scenario.version);
                        println!("    Path: {}", scenario.path.display());
                        println!("    Description: {}", scenario.manifest.description);
                        println!("    Category: {:?}", scenario.manifest.category);
                        println!("    Installed: {}", scenario.installed_at);
                    } else {
                        println!("  - {}@{}", scenario.name, scenario.version);
                    }
                }
            }
        }

        ScenarioCommands::Info { name, version } => {
            let mut installer = ScenarioInstaller::new()?;
            installer.init().await?;

            let scenario = if let Some(v) = version {
                installer.get(&name, &v)
            } else {
                installer.get_latest(&name)
            };

            match scenario {
                Some(s) => {
                    println!("ℹ️  Scenario information: {}@{}", s.name, s.version);
                    println!("   Title: {}", s.manifest.title);
                    println!("   Description: {}", s.manifest.description);
                    println!("   Author: {}", s.manifest.author);
                    println!("   Category: {:?}", s.manifest.category);
                    println!("   Tags: {}", s.manifest.tags.join(", "));
                    println!("   Path: {}", s.path.display());
                    println!("   Installed: {}", s.installed_at);
                    if let Some(updated) = s.updated_at {
                        println!("   Updated: {}", updated);
                    }
                }
                None => {
                    eprintln!("❌ Scenario '{}' not found", name);
                    std::process::exit(1);
                }
            }
        }

        ScenarioCommands::Preview { source } => {
            println!("👁️  Previewing scenario from: {}", source);

            let installer = ScenarioInstaller::new()?;

            match installer.preview(&source).await {
                Ok(preview) => {
                    println!("{}", preview.format_display());
                }
                Err(e) => {
                    eprintln!("❌ Failed to preview scenario: {}", e);
                    std::process::exit(1);
                }
            }
        }

        ScenarioCommands::Use {
            name,
            version,
            merge_strategy,
            auto_align,
        } => {
            let mut installer = ScenarioInstaller::new()?;
            installer.init().await?;

            let version_clone = version.clone();
            let scenario = if let Some(ref v) = version {
                installer.get(&name, v)
            } else {
                installer.get_latest(&name)
            };

            match scenario {
                Some(s) => {
                    println!("🎯 Applying scenario: {}@{}", s.name, s.version);

                    // Parse merge strategy
                    let alignment_config = if auto_align {
                        use mockforge_scenarios::schema_alignment::{
                            MergeStrategy, SchemaAlignmentConfig,
                        };
                        let strategy = match merge_strategy.as_str() {
                            "prefer-existing" => MergeStrategy::PreferExisting,
                            "prefer-scenario" => MergeStrategy::PreferScenario,
                            "intelligent" => MergeStrategy::Intelligent,
                            "interactive" => MergeStrategy::Interactive,
                            _ => {
                                eprintln!("❌ Invalid merge strategy: {}. Valid options: prefer-existing, prefer-scenario, intelligent, interactive", merge_strategy);
                                std::process::exit(1);
                            }
                        };
                        Some(SchemaAlignmentConfig {
                            merge_strategy: strategy,
                            validate_merged: true,
                            backup_existing: true,
                        })
                    } else {
                        None
                    };

                    match installer
                        .apply_to_workspace_with_alignment(
                            &name,
                            version_clone.as_deref(),
                            alignment_config,
                        )
                        .await
                    {
                        Ok(_) => {
                            println!("✅ Scenario applied successfully to workspace");
                            if auto_align {
                                println!(
                                    "   Schema alignment enabled (strategy: {})",
                                    merge_strategy
                                );
                            }
                            println!(
                                "   Files copied: config.yaml, openapi.json, fixtures/, examples/"
                            );

                            // Apply VBR entities if present
                            if let Ok(Some(vbr_entities)) =
                                installer.get_vbr_entities(&name, version_clone.as_deref())
                            {
                                if !vbr_entities.is_empty() {
                                    println!("\n🔧 Applying VBR entities...");
                                    match apply_vbr_entities_from_scenario(vbr_entities, &s.path)
                                        .await
                                    {
                                        Ok(count) => {
                                            println!("   ✅ Applied {} VBR entities", count);
                                        }
                                        Err(e) => {
                                            println!(
                                                "   ⚠️  Warning: Failed to apply VBR entities: {}",
                                                e
                                            );
                                            println!("   You can apply them manually using 'mockforge vbr' commands");
                                        }
                                    }
                                }
                            }

                            // Merge MockAI config if present
                            if let Ok(Some(mockai_config)) =
                                installer.get_mockai_config(&name, version_clone.as_deref())
                            {
                                println!("\n🤖 Merging MockAI configuration...");
                                match merge_mockai_config_from_scenario(mockai_config, &s.path)
                                    .await
                                {
                                    Ok(_) => {
                                        println!("   ✅ MockAI config merged into config.yaml");
                                    }
                                    Err(e) => {
                                        println!(
                                            "   ⚠️  Warning: Failed to merge MockAI config: {}",
                                            e
                                        );
                                        println!(
                                            "   MockAI config is available in the scenario package"
                                        );
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("❌ Failed to apply scenario: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                None => {
                    eprintln!("❌ Scenario '{}' not found", name);
                    std::process::exit(1);
                }
            }
        }

        ScenarioCommands::Search {
            query,
            category,
            limit,
        } => {
            println!("🔍 Searching for scenarios: {}", query);

            let registry = ScenarioRegistry::new("https://registry.mockforge.dev".to_string());

            let search_query = mockforge_scenarios::ScenarioSearchQuery {
                query: Some(query),
                category,
                tags: vec![],
                sort: mockforge_scenarios::ScenarioSortOrder::Relevance,
                page: 0,
                per_page: limit,
            };

            match registry.search(search_query).await {
                Ok(results) => {
                    if results.scenarios.is_empty() {
                        println!("  No scenarios found");
                    } else {
                        println!("  Found {} scenarios:", results.total);
                        for scenario in results.scenarios {
                            println!("  - {}@{}", scenario.name, scenario.version);
                            println!("    {}", scenario.description);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("❌ Failed to search scenarios: {}", e);
                    std::process::exit(1);
                }
            }
        }

        ScenarioCommands::Publish { path, registry } => {
            println!("📤 Publishing scenario from: {}", path);

            // Load scenario package
            let package = mockforge_scenarios::ScenarioPackage::from_directory(&path)
                .map_err(|e| anyhow::anyhow!("Failed to load scenario package: {}", e))?;

            // Validate package
            let validation = package
                .validate()
                .map_err(|e| anyhow::anyhow!("Package validation failed: {}", e))?;

            if !validation.is_valid {
                eprintln!("❌ Package validation failed:");
                for error in &validation.errors {
                    eprintln!("   - {}", error);
                }
                std::process::exit(1);
            }

            // Warn about warnings
            if !validation.warnings.is_empty() {
                println!("⚠️  Package validation warnings:");
                for warning in &validation.warnings {
                    println!("   - {}", warning);
                }
            }

            // Create package archive
            println!("   Creating package archive...");
            let (archive_path, checksum, size) = create_scenario_archive(&package)
                .map_err(|e| anyhow::anyhow!("Failed to create archive: {}", e))?;

            println!("   Package size: {} bytes", size);
            println!("   Checksum: {}", checksum);

            // Read archive as base64
            let archive_data = std::fs::read(&archive_path)
                .map_err(|e| anyhow::anyhow!("Failed to read archive: {}", e))?;
            let archive_base64 = general_purpose::STANDARD.encode(&archive_data);

            // Serialize manifest
            let manifest_json = serde_json::to_string(&package.manifest)
                .map_err(|e| anyhow::anyhow!("Failed to serialize manifest: {}", e))?;

            // Create publish request
            let publish_request = mockforge_scenarios::ScenarioPublishRequest {
                manifest: manifest_json,
                package: archive_base64,
                checksum,
                size,
            };

            // Get registry URL and token
            let registry_url =
                registry.unwrap_or_else(|| "https://registry.mockforge.dev".to_string());
            let token = std::env::var("MOCKFORGE_REGISTRY_TOKEN")
                .map_err(|_| anyhow::anyhow!("MOCKFORGE_REGISTRY_TOKEN environment variable not set. Required for publishing."))?;

            // Create registry client and publish
            let registry_client =
                mockforge_scenarios::ScenarioRegistry::with_token(registry_url, token);

            println!("   Publishing to registry...");
            match registry_client.publish(publish_request).await {
                Ok(response) => {
                    println!("✅ Scenario published successfully!");
                    println!("   Name: {}@{}", response.name, response.version);
                    println!("   Download URL: {}", response.download_url);
                    println!("   Published at: {}", response.published_at);
                }
                Err(e) => {
                    eprintln!("❌ Failed to publish scenario: {}", e);
                    std::process::exit(1);
                }
            }

            // Clean up temp archive
            let _ = std::fs::remove_file(&archive_path);
        }

        ScenarioCommands::Update { name, all } => {
            let mut installer = ScenarioInstaller::new()?;
            installer.init().await?;

            if all {
                println!("🔄 Updating all scenarios...");

                match installer.update_all().await {
                    Ok(updated) => {
                        if updated.is_empty() {
                            println!("✅ All scenarios are up to date");
                        } else {
                            println!("✅ Updated {} scenarios:", updated.len());
                            for item in updated {
                                println!("   - {}", item);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("❌ Failed to update scenarios: {}", e);
                        std::process::exit(1);
                    }
                }
            } else if let Some(n) = name {
                println!("🔄 Updating scenario: {}", n);

                // Get current scenario info
                let scenario = installer
                    .get_latest(&n)
                    .ok_or_else(|| anyhow::anyhow!("Scenario '{}' not found", n))?;

                let current_version = scenario.version.clone();
                let source_str = scenario.source.clone();

                // Parse source to check if it's from registry
                let source = mockforge_scenarios::ScenarioSource::parse(&source_str)?;

                match source {
                    mockforge_scenarios::ScenarioSource::Registry { .. } => {
                        // Update from registry
                        let mut installer_mut = installer;
                        match installer_mut.update_from_registry(&n, &current_version).await {
                            Ok(new_version) => {
                                if new_version == current_version {
                                    println!(
                                        "✅ Scenario is already up to date: {}@{}",
                                        n, current_version
                                    );
                                } else {
                                    println!(
                                        "✅ Scenario updated: {}@{} -> {}",
                                        n, current_version, new_version
                                    );
                                }
                            }
                            Err(e) => {
                                eprintln!("❌ Failed to update scenario: {}", e);
                                std::process::exit(1);
                            }
                        }
                    }
                    _ => {
                        // Reinstall from original source
                        println!("   Reinstalling from original source: {}", source_str);
                        let options = InstallOptions {
                            force: true,
                            skip_validation: false,
                            expected_checksum: None,
                        };

                        match installer.install(&source_str, options).await {
                            Ok(_) => {
                                println!("✅ Scenario updated successfully");
                            }
                            Err(e) => {
                                eprintln!("❌ Failed to update scenario: {}", e);
                                std::process::exit(1);
                            }
                        }
                    }
                }
            } else {
                eprintln!("❌ Please specify a scenario name or use --all");
                std::process::exit(1);
            }
        }

        ScenarioCommands::Pack { command } => match command {
            PackCommands::List => {
                println!("📦 Installed domain packs:");

                let pack_installer = DomainPackInstaller::new()?;
                pack_installer.init()?;

                let packs = pack_installer.list_installed()?;

                if packs.is_empty() {
                    println!("  No packs installed");
                } else {
                    for pack in packs {
                        println!(
                            "  - {}@{} ({})",
                            pack.manifest.name, pack.manifest.version, pack.manifest.domain
                        );
                        println!("    Title: {}", pack.manifest.title);
                        println!("    Scenarios: {}", pack.manifest.scenarios.len());
                    }
                }
            }

            PackCommands::Install { manifest } => {
                println!("📦 Installing domain pack from: {}", manifest);

                let pack_installer = DomainPackInstaller::new()?;
                pack_installer.init()?;

                let manifest_path = Path::new(&manifest);
                if !manifest_path.exists() {
                    eprintln!("❌ Pack manifest not found: {}", manifest);
                    std::process::exit(1);
                }

                match pack_installer.install_from_manifest(manifest_path) {
                    Ok(pack_info) => {
                        println!(
                            "✅ Pack installed successfully: {}@{}",
                            pack_info.manifest.name, pack_info.manifest.version
                        );
                        println!("   Domain: {}", pack_info.manifest.domain);
                        println!("   Scenarios: {}", pack_info.manifest.scenarios.len());
                        println!("\n   To install scenarios from this pack, use:");
                        for scenario in &pack_info.manifest.scenarios {
                            println!("     mockforge scenario install {}", scenario.source);
                        }
                    }
                    Err(e) => {
                        eprintln!("❌ Failed to install pack: {}", e);
                        std::process::exit(1);
                    }
                }
            }

            PackCommands::Info { name } => {
                let pack_installer = DomainPackInstaller::new()?;
                pack_installer.init()?;

                match pack_installer.get_pack(&name)? {
                    Some(pack_info) => {
                        println!(
                            "ℹ️  Pack information: {}@{}",
                            pack_info.manifest.name, pack_info.manifest.version
                        );
                        println!("   Title: {}", pack_info.manifest.title);
                        println!("   Description: {}", pack_info.manifest.description);
                        println!("   Domain: {}", pack_info.manifest.domain);
                        println!("   Author: {}", pack_info.manifest.author);
                        println!("   Scenarios ({}):", pack_info.manifest.scenarios.len());
                        for scenario in &pack_info.manifest.scenarios {
                            println!("     - {} ({})", scenario.name, scenario.source);
                            if let Some(ref desc) = scenario.description {
                                println!("       {}", desc);
                            }
                        }
                        // Show studio pack components if present
                        if !pack_info.manifest.personas.is_empty() {
                            println!("   Personas ({}):", pack_info.manifest.personas.len());
                            for persona in &pack_info.manifest.personas {
                                println!("     - {} ({})", persona.name, persona.id);
                            }
                        }
                        if !pack_info.manifest.chaos_rules.is_empty() {
                            println!("   Chaos Rules ({}):", pack_info.manifest.chaos_rules.len());
                            for rule in &pack_info.manifest.chaos_rules {
                                println!("     - {}", rule.name);
                            }
                        }
                        if !pack_info.manifest.contract_diffs.is_empty() {
                            println!(
                                "   Contract Diffs ({}):",
                                pack_info.manifest.contract_diffs.len()
                            );
                            for diff in &pack_info.manifest.contract_diffs {
                                println!("     - {}", diff.name);
                            }
                        }
                        if !pack_info.manifest.reality_blends.is_empty() {
                            println!(
                                "   Reality Blends ({}):",
                                pack_info.manifest.reality_blends.len()
                            );
                            for blend in &pack_info.manifest.reality_blends {
                                println!(
                                    "     - {} ({}% real)",
                                    blend.name,
                                    (blend.reality_ratio * 100.0) as u32
                                );
                            }
                        }
                    }
                    None => {
                        eprintln!("❌ Pack '{}' not found", name);
                        std::process::exit(1);
                    }
                }
            }

            PackCommands::Studio { command } => {
                use mockforge_scenarios::StudioPackInstaller;

                let packs_dir = dirs::data_dir()
                    .ok_or_else(|| anyhow::anyhow!("Failed to get data directory"))?
                    .join("mockforge")
                    .join("packs");
                let studio_installer = StudioPackInstaller::new(packs_dir);

                match command {
                    StudioPackCommands::Install {
                        pack_name,
                        workspace,
                    } => {
                        println!("🎨 Installing studio pack: {}", pack_name);

                        // Check if it's a pre-built pack or a path
                        let manifest = if pack_name == "fintech-fraud-lab" {
                            #[cfg(feature = "studio-packs")]
                            {
                                Some(mockforge_scenarios::create_fintech_fraud_lab_pack())
                            }
                            #[cfg(not(feature = "studio-packs"))]
                            {
                                return Err(anyhow::anyhow!("studio-packs feature not enabled"));
                            }
                        } else if pack_name == "ecommerce-peak-day" {
                            #[cfg(feature = "studio-packs")]
                            {
                                Some(mockforge_scenarios::create_ecommerce_peak_day_pack())
                            }
                            #[cfg(not(feature = "studio-packs"))]
                            {
                                return Err(anyhow::anyhow!("studio-packs feature not enabled"));
                            }
                        } else if pack_name == "healthcare-outage-drill" {
                            #[cfg(feature = "studio-packs")]
                            {
                                Some(mockforge_scenarios::create_healthcare_outage_drill_pack())
                            }
                            #[cfg(not(feature = "studio-packs"))]
                            {
                                return Err(anyhow::anyhow!("studio-packs feature not enabled"));
                            }
                        } else {
                            // Try to load from file
                            let manifest_path = Path::new(&pack_name);
                            if manifest_path.exists() {
                                Some(mockforge_scenarios::DomainPackManifest::from_file(
                                    manifest_path,
                                )?)
                            } else {
                                eprintln!("❌ Studio pack '{}' not found", pack_name);
                                eprintln!("   Available pre-built packs: fintech-fraud-lab, ecommerce-peak-day, healthcare-outage-drill");
                                eprintln!("   Or provide a path to a pack manifest file");
                                std::process::exit(1);
                            }
                        };

                        if let Some(manifest) = manifest {
                            match studio_installer
                                .install_studio_pack(&manifest, Some(&workspace))
                                .await
                            {
                                Ok(result) => {
                                    println!("✅ Studio pack installed successfully!");
                                    println!("   Scenarios: {}", result.scenarios_installed);
                                    println!("   Personas: {}", result.personas_configured);
                                    println!("   Chaos Rules: {}", result.chaos_rules_applied);
                                    println!(
                                        "   Contract Diffs: {}",
                                        result.contract_diffs_configured
                                    );
                                    println!(
                                        "   Reality Blends: {}",
                                        result.reality_blends_configured
                                    );
                                    if result.workspace_config_applied {
                                        println!("   Workspace Config: Applied");
                                    }
                                    if !result.errors.is_empty() {
                                        println!("\n⚠️  Warnings:");
                                        for error in &result.errors {
                                            println!("   - {}", error);
                                        }
                                    }
                                }
                                Err(e) => {
                                    eprintln!("❌ Failed to install studio pack: {}", e);
                                    std::process::exit(1);
                                }
                            }
                        }
                    }

                    StudioPackCommands::List => {
                        println!("🎨 Available studio packs:");
                        println!("   Pre-built packs:");
                        println!(
                            "     - fintech-fraud-lab: Fraud detection and prevention scenarios"
                        );
                        println!("     - ecommerce-peak-day: High-traffic e-commerce scenarios");
                        println!(
                            "     - healthcare-outage-drill: Healthcare system outage scenarios"
                        );
                        println!("\n   To install a pack, use:");
                        println!("     mockforge scenario pack studio install <pack-name>");
                    }

                    StudioPackCommands::Create { name, output } => {
                        println!("🎨 Creating studio pack: {}", name);

                        // Load current workspace configuration
                        let config_path = std::env::current_dir()
                            .ok()
                            .map(|d| d.join("mockforge.yaml"))
                            .filter(|p| p.exists());

                        let config = if let Some(ref path) = config_path {
                            match load_config(path).await {
                                Ok(c) => c,
                                Err(e) => {
                                    eprintln!("⚠️  Failed to load config: {}. Using defaults.", e);
                                    ServerConfig::default()
                                }
                            }
                        } else {
                            println!("⚠️  No mockforge.yaml found. Creating pack with minimal configuration.");
                            ServerConfig::default()
                        };

                        // Extract personas from config (from mockai.intelligent_behavior.personas)
                        let personas = config
                            .mockai
                            .intelligent_behavior
                            .personas
                            .personas
                            .iter()
                            .map(|persona| mockforge_scenarios::domain_pack::StudioPersona {
                                id: persona.name.clone(),
                                name: persona.name.clone(),
                                domain: "general".to_string(), // Personas don't have domain in this structure
                                traits: HashMap::new(), // Personas don't have traits in this structure
                                backstory: None,
                                relationships: HashMap::new(),
                                metadata: HashMap::new(),
                            })
                            .collect();

                        // Extract chaos rules from config (chaos rules are not directly in ServerConfig)
                        // For now, create empty list - chaos rules would need to be extracted from a different source
                        let chaos_rules = Vec::new();

                        // Extract contract diffs from config (fitness rules as contract diffs)
                        let contract_diffs = config
                            .contracts
                            .fitness_rules
                            .iter()
                            .map(|rule| {
                                // Convert fitness rule to contract diff format
                                let drift_budget = serde_json::json!({
                                    "rule_type": rule.rule_type,
                                    "scope": rule.scope,
                                    "max_percent_increase": rule.max_percent_increase,
                                    "max_fields": rule.max_fields,
                                    "max_depth": rule.max_depth,
                                });

                                mockforge_scenarios::domain_pack::StudioContractDiff {
                                    name: rule.name.clone(),
                                    description: None,
                                    drift_budget,
                                    endpoint_patterns: vec![rule.scope.clone()],
                                }
                            })
                            .collect();

                        // Extract reality blends from config (reality level is not directly in ServerConfig)
                        // For now, use default moderate realism
                        let reality_blends = {
                            let reality_ratio = 0.5; // Default to moderate realism

                            vec![mockforge_scenarios::domain_pack::StudioRealityBlend {
                                name: "default".to_string(),
                                description: Some("Default reality blend from config".to_string()),
                                reality_ratio,
                                continuum_config: serde_json::json!({"level": 3}),
                                field_rules: Vec::new(),
                            }]
                        };

                        // Create studio pack manifest
                        let manifest = mockforge_scenarios::domain_pack::DomainPackManifest {
                            manifest_version: "1.0".to_string(),
                            name: name.clone(),
                            version: "1.0.0".to_string(),
                            title: format!("Studio Pack: {}", name),
                            description: format!("Exported studio pack from workspace: {}", name),
                            domain: "general".to_string(),
                            author: std::env::var("USER").unwrap_or_else(|_| "unknown".to_string()),
                            scenarios: Vec::new(), // Scenarios would need to be extracted separately
                            tags: vec!["exported".to_string(), "workspace".to_string()],
                            metadata: HashMap::new(),
                            personas,
                            chaos_rules,
                            contract_diffs,
                            reality_blends,
                            workspace_config: serde_json::to_value(&config).ok(),
                        };

                        // Determine output path
                        let output_path = output
                            .map(std::path::PathBuf::from)
                            .unwrap_or_else(|| std::path::PathBuf::from(format!("{}.yaml", name)));

                        // Write manifest to file
                        let yaml_content = serde_yaml::to_string(&manifest)
                            .map_err(|e| anyhow::anyhow!("Failed to serialize manifest: {}", e))?;

                        std::fs::write(&output_path, yaml_content)
                            .map_err(|e| anyhow::anyhow!("Failed to write manifest: {}", e))?;

                        println!("✅ Studio pack created successfully!");
                        println!("   Output: {}", output_path.display());
                        println!("   Personas: {}", manifest.personas.len());
                        println!("   Chaos rules: {}", manifest.chaos_rules.len());
                        println!("   Contract diffs: {}", manifest.contract_diffs.len());
                        println!("   Reality blends: {}", manifest.reality_blends.len());
                    }
                }
            }
        },

        ScenarioCommands::Review { command } => match command {
            ReviewCommands::Submit {
                scenario_name,
                scenario_version,
                rating,
                title,
                comment,
                reviewer,
                reviewer_email,
                verified,
            } => {
                println!("⭐ Submitting review for scenario: {}", scenario_name);

                let registry = ScenarioRegistry::new("https://registry.mockforge.dev".to_string());

                let review = ScenarioReviewSubmission {
                    scenario_name,
                    scenario_version,
                    rating,
                    title,
                    comment,
                    reviewer,
                    reviewer_email,
                    verified_purchase: verified,
                };

                match registry.submit_review(review).await {
                    Ok(submitted_review) => {
                        println!("✅ Review submitted successfully!");
                        println!("   Review ID: {}", submitted_review.id);
                        println!("   Rating: {}/5", submitted_review.rating);
                        if let Some(ref review_title) = submitted_review.title {
                            println!("   Title: {}", review_title);
                        }
                    }
                    Err(e) => {
                        eprintln!("❌ Failed to submit review: {}", e);
                        std::process::exit(1);
                    }
                }
            }

            ReviewCommands::List {
                scenario_name,
                page,
                per_page,
            } => {
                println!("📝 Reviews for scenario: {}", scenario_name);

                let registry = ScenarioRegistry::new("https://registry.mockforge.dev".to_string());

                match registry.get_reviews(&scenario_name, page, per_page).await {
                    Ok(reviews) => {
                        if reviews.is_empty() {
                            println!("  No reviews found");
                        } else {
                            println!("  Found {} reviews:", reviews.len());
                            for review in reviews {
                                println!("  - {} ({}/5)", review.reviewer, review.rating);
                                if let Some(ref title) = review.title {
                                    println!("    Title: {}", title);
                                }
                                println!("    Comment: {}", review.comment);
                                println!("    Date: {}", review.created_at);
                                if review.verified_purchase {
                                    println!("    ✓ Verified purchase");
                                }
                                if review.helpful_count > 0 {
                                    println!("    👍 {} helpful", review.helpful_count);
                                }
                                println!();
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("❌ Failed to get reviews: {}", e);
                        std::process::exit(1);
                    }
                }
            }
        },
    }

    Ok(())
}

/// Create a scenario package archive (ZIP format)
fn create_scenario_archive(
    package: &mockforge_scenarios::ScenarioPackage,
) -> anyhow::Result<(std::path::PathBuf, String, u64)> {
    use zip::write::FileOptions;
    use zip::ZipWriter;

    // Create temporary file for archive
    let temp_file = tempfile::NamedTempFile::new()?;
    let archive_path = temp_file.path().to_path_buf();
    drop(temp_file); // Close file so we can write to it

    // Create ZIP archive
    let file = fs::File::create(&archive_path)?;
    let mut zip = ZipWriter::new(file);
    let options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);

    // Add all files from package
    for file_path in &package.files {
        let full_path = package.root.join(file_path);

        if full_path.is_dir() {
            continue; // Skip directories, we'll add files individually
        }

        if !full_path.exists() {
            continue; // Skip missing files
        }

        // Get relative path for archive
        let archive_name = file_path.to_string_lossy().replace('\\', "/");

        // Read file contents
        let file_contents = fs::read(&full_path)?;

        // Add to ZIP
        zip.start_file(&archive_name, options)?;
        zip.write_all(&file_contents)?;
    }

    // Add all files from directories recursively
    add_directory_to_zip(&mut zip, &package.root, &package.root, options)?;

    // Finish writing the ZIP (consumes the writer)
    let _file = zip.finish()?;

    // Calculate checksum
    let archive_data = fs::read(&archive_path)?;
    let checksum = calculate_checksum(&archive_data);
    let size = archive_data.len() as u64;

    Ok((archive_path, checksum, size))
}

/// Recursively add directory contents to ZIP
fn add_directory_to_zip(
    zip: &mut zip::ZipWriter<fs::File>,
    base: &Path,
    dir: &Path,
    options: zip::write::FileOptions<()>,
) -> anyhow::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name();

        // Skip hidden files and common ignore patterns
        if file_name.to_string_lossy().starts_with('.') {
            continue;
        }

        if path.is_dir() {
            // Recursively add subdirectory
            add_directory_to_zip(zip, base, &path, options)?;
        } else {
            // Add file
            let relative_path = path
                .strip_prefix(base)
                .map_err(|e| anyhow::anyhow!("Failed to compute relative path: {}", e))?;
            let archive_name = relative_path.to_string_lossy().replace('\\', "/");

            let file_contents = fs::read(&path)?;
            zip.start_file(&archive_name, options)?;
            zip.write_all(&file_contents)?;
        }
    }

    Ok(())
}

/// Calculate SHA-256 checksum
fn calculate_checksum(data: &[u8]) -> String {
    use ring::digest::{Context, SHA256};
    let mut context = Context::new(&SHA256);
    context.update(data);
    let digest = context.finish();
    hex::encode(digest.as_ref())
}

/// Apply VBR entities from a scenario
///
/// This function converts scenario VBR entity definitions to actual VBR entities
/// and applies them. Since this requires both mockforge-scenarios and mockforge-vbr,
/// it's implemented in the CLI layer.
async fn apply_vbr_entities_from_scenario(
    entities: &[mockforge_scenarios::VbrEntityDefinition],
    scenario_path: &std::path::Path,
) -> anyhow::Result<usize> {
    use mockforge_vbr::schema::VbrSchemaDefinition;

    // For now, we'll create a simple VBR config and try to register entities
    // In a full implementation, this would integrate with an existing VBR engine
    // or create a new one based on workspace configuration

    let mut applied_count = 0;

    for entity_def in entities {
        // Try to convert JSON schema to VbrSchemaDefinition
        // The schema is stored as JSON, so we need to parse it
        match serde_json::from_value::<VbrSchemaDefinition>(entity_def.schema.clone()) {
            Ok(vbr_schema) => {
                // Create entity with state machine if provided
                // Convert state machine to the type expected by Entity::with_state_machine
                let entity = if let Some(ref state_machine) = entity_def.state_machine {
                    // Convert from mockforge_scenarios::StateMachine to mockforge_core::intelligent_behavior::rules::StateMachine
                    use mockforge_core::intelligent_behavior::rules::StateMachine as CoreStateMachine;
                    let core_state_machine: CoreStateMachine =
                        serde_json::from_value(serde_json::to_value(state_machine)?)?;
                    Entity::with_state_machine(
                        entity_def.name.clone(),
                        vbr_schema,
                        core_state_machine,
                    )
                } else {
                    Entity::new(entity_def.name.clone(), vbr_schema)
                };

                // Note: In a full implementation, we would:
                // 1. Load or create a VBR engine from workspace config
                // 2. Register the entity with the engine
                // 3. Create database tables
                // 4. Seed data if seed_data_path is provided
                //
                // For now, we'll just validate that the entity can be created
                // The actual registration would happen when the server starts
                // or via explicit VBR commands

                println!("   - Entity '{}' ready for registration", entity_def.name);

                // If seed data path is provided, note it
                if let Some(ref seed_path) = entity_def.seed_data_path {
                    let full_seed_path = scenario_path.join(seed_path);
                    if full_seed_path.exists() {
                        println!("     Seed data: {}", seed_path);
                    } else {
                        println!("     ⚠️  Seed data file not found: {}", seed_path);
                    }
                }

                applied_count += 1;
            }
            Err(e) => {
                // If direct deserialization fails, try to convert from JSON Schema format
                // This is a simplified conversion - in production, you'd want a more robust converter
                println!(
                    "   ⚠️  Warning: Could not parse entity schema for '{}': {}",
                    entity_def.name, e
                );
                println!("     Entity definition will need manual conversion");
            }
        }
    }

    if applied_count > 0 {
        println!(
            "\n   💡 Note: VBR entities are prepared but need to be registered with a VBR engine."
        );
        println!(
            "   Use 'mockforge vbr create entity <name>' or start MockForge with VBR enabled."
        );
    }

    Ok(applied_count)
}

/// Merge MockAI configuration from a scenario into existing config.yaml
async fn merge_mockai_config_from_scenario(
    mockai_config_def: &mockforge_scenarios::MockAIConfigDefinition,
    scenario_path: &std::path::Path,
) -> anyhow::Result<()> {
    let current_dir = std::env::current_dir()
        .map_err(|e| anyhow::anyhow!("Failed to get current directory: {}", e))?;

    let config_path = current_dir.join("config.yaml");

    // Load existing config or create default
    let mut config = if config_path.exists() {
        load_config(&config_path)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to load config.yaml: {}", e))?
    } else {
        ServerConfig::default()
    };

    // Merge MockAI config from scenario
    // The config is stored as JSON, so we need to convert it
    let mockai_config_json = &mockai_config_def.config;

    // Try to deserialize as MockAIConfig
    if let Ok(scenario_mockai) =
        serde_json::from_value::<mockforge_core::config::MockAIConfig>(mockai_config_json.clone())
    {
        // Merge: prefer scenario config over existing
        config.mockai = scenario_mockai;
    } else {
        // If direct deserialization fails, try manual merging
        // Extract key fields from JSON
        if let Some(enabled) = mockai_config_json.get("enabled").and_then(|v| v.as_bool()) {
            config.mockai.enabled = enabled;
        }

        // Try to merge intelligent_behavior if present
        if let Some(behavior_json) = mockai_config_json.get("intelligent_behavior") {
            if let Ok(behavior_config) = serde_json::from_value::<
                mockforge_core::intelligent_behavior::IntelligentBehaviorConfig,
            >(behavior_json.clone())
            {
                config.mockai.intelligent_behavior = behavior_config;
            }
        }

        // Merge other boolean flags
        if let Some(auto_learn) = mockai_config_json.get("auto_learn").and_then(|v| v.as_bool()) {
            config.mockai.auto_learn = auto_learn;
        }
        if let Some(mutation_detection) =
            mockai_config_json.get("mutation_detection").and_then(|v| v.as_bool())
        {
            config.mockai.mutation_detection = mutation_detection;
        }
        if let Some(ai_validation_errors) =
            mockai_config_json.get("ai_validation_errors").and_then(|v| v.as_bool())
        {
            config.mockai.ai_validation_errors = ai_validation_errors;
        }
        if let Some(intelligent_pagination) =
            mockai_config_json.get("intelligent_pagination").and_then(|v| v.as_bool())
        {
            config.mockai.intelligent_pagination = intelligent_pagination;
        }
    }

    // Load behavior rules if provided
    if let Some(ref rules_path) = mockai_config_def.behavior_rules_path {
        let full_rules_path = scenario_path.join(rules_path);
        if full_rules_path.exists() {
            println!("   Behavior rules file: {}", rules_path);
            // Note: Behavior rules would be loaded when MockAI is initialized
            // This is just informational for now
        }
    }

    // Load example pairs if provided
    if let Some(ref pairs_path) = mockai_config_def.example_pairs_path {
        let full_pairs_path = scenario_path.join(pairs_path);
        if full_pairs_path.exists() {
            println!("   Example pairs file: {}", pairs_path);
            // Note: Example pairs would be loaded when MockAI is initialized
            // This is just informational for now
        }
    }

    // Save merged config
    save_config(&config_path, &config)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to save config.yaml: {}", e))?;

    Ok(())
}

/// Reality Profile Pack commands
#[derive(Subcommand)]
pub enum RealityProfileCommands {
    /// Install a reality profile pack
    Install {
        /// Pack name or path to manifest file
        pack_name: String,
    },
    /// List installed reality profile packs
    List,
    /// Apply a reality profile pack to a workspace
    Apply {
        /// Pack name
        pack_name: String,
        /// Workspace ID (defaults to "default")
        #[arg(short, long, default_value = "default")]
        workspace: String,
    },
    /// Show information about a reality profile pack
    Info {
        /// Pack name
        pack_name: String,
    },
}

/// Behavioral Economics Rule commands
#[derive(Subcommand)]
pub enum BehaviorRuleCommands {
    /// Add a new behavior rule
    Add {
        /// Rule name
        #[arg(short, long)]
        name: String,
        /// Rule type (declarative or scriptable)
        #[arg(long, default_value = "declarative")]
        rule_type: String,
        /// Condition type (latency, load, pricing, fraud, segment, error-rate)
        #[arg(short, long)]
        condition: String,
        /// Condition threshold or value
        #[arg(long)]
        threshold: Option<String>,
        /// Endpoint pattern (e.g., "/api/users/*", "*" for all endpoints)
        #[arg(long, default_value = "*")]
        endpoint: String,
        /// Action type (modify-conversion-rate, decline-transaction, increase-churn, change-status, modify-latency, trigger-chaos)
        #[arg(short, long)]
        action: String,
        /// Action parameter (e.g., multiplier for conversion rate, status code for change-status)
        #[arg(long)]
        parameter: Option<String>,
        /// Priority (higher = evaluated first)
        #[arg(short, long, default_value = "100")]
        priority: u32,
        /// Script content (for scriptable rules)
        #[arg(long)]
        script: Option<String>,
        /// Script language (javascript, wasm)
        #[arg(long)]
        script_language: Option<String>,
    },
    /// List all behavior rules
    List,
    /// Remove a behavior rule
    Remove {
        /// Rule name
        name: String,
    },
    /// Enable behavioral economics engine
    Enable,
    /// Disable behavioral economics engine
    Disable,
    /// Show current status
    Status,
}

/// Drift Learning commands
#[derive(Subcommand)]
pub enum DriftLearningCommands {
    /// Enable drift learning
    Enable {
        /// Learning sensitivity (0.0 to 1.0)
        #[arg(short, long, default_value = "0.2")]
        sensitivity: f64,
        /// Minimum samples before learning starts
        #[arg(long, default_value = "10")]
        min_samples: usize,
        /// Learning mode (behavioral, statistical, hybrid)
        #[arg(short, long, default_value = "behavioral")]
        mode: String,
        /// Enable persona adaptation
        #[arg(long, default_value = "true")]
        persona_adaptation: bool,
        /// Enable traffic pattern mirroring
        #[arg(long, default_value = "true")]
        traffic_mirroring: bool,
    },
    /// Disable drift learning
    Disable,
    /// Show current drift learning status
    Status,
    /// Configure per-endpoint learning
    Endpoint {
        /// Endpoint pattern
        endpoint: String,
        /// Enable or disable learning for this endpoint
        #[arg(short, long)]
        enable: bool,
    },
    /// Configure per-persona learning
    Persona {
        /// Persona ID
        persona_id: String,
        /// Enable or disable learning for this persona
        #[arg(short, long)]
        enable: bool,
    },
}

/// Handle reality profile pack commands
pub async fn handle_reality_profile_command(command: RealityProfileCommands) -> anyhow::Result<()> {
    use mockforge_scenarios::RealityProfilePackInstaller;

    let installer = RealityProfilePackInstaller::new()?;
    installer.init()?;

    match command {
        RealityProfileCommands::Install { pack_name } => {
            println!("📦 Installing reality profile pack: {}", pack_name);

            // Check if it's a pre-built pack or a path
            let manifest = if pack_name == "ecommerce-peak-season" {
                Some(mockforge_scenarios::create_ecommerce_peak_season_pack())
            } else if pack_name == "fintech-fraud" {
                Some(mockforge_scenarios::create_fintech_fraud_pack())
            } else if pack_name == "healthcare-hl7" {
                Some(mockforge_scenarios::create_healthcare_hl7_pack())
            } else if pack_name == "iot-fleet-chaos" {
                Some(mockforge_scenarios::create_iot_fleet_chaos_pack())
            } else {
                // Try as a file path
                let path = std::path::Path::new(&pack_name);
                if path.exists() {
                    Some(mockforge_scenarios::RealityProfilePackManifest::from_file(path)?)
                } else {
                    None
                }
            };

            if let Some(manifest) = manifest {
                // Save manifest to temp file and install
                let temp_dir = std::env::temp_dir();
                let temp_manifest = temp_dir.join(format!("{}.yaml", manifest.name));
                manifest.to_file(&temp_manifest)?;
                installer.install_from_manifest(&temp_manifest)?;
                println!("✅ Reality profile pack installed: {}", manifest.name);
            } else {
                anyhow::bail!("Pack not found: {}", pack_name);
            }
        }
        RealityProfileCommands::List => {
            let packs = installer.list_installed()?;
            if packs.is_empty() {
                println!("No reality profile packs installed");
            } else {
                println!("Installed reality profile packs:");
                for pack in packs {
                    println!(
                        "  - {} v{} ({})",
                        pack.manifest.name, pack.manifest.version, pack.manifest.domain
                    );
                }
            }
        }
        RealityProfileCommands::Apply {
            pack_name,
            workspace,
        } => {
            println!("🎯 Applying reality profile pack: {} to workspace: {}", pack_name, workspace);

            let pack_info = installer.get_pack(&pack_name)?;
            if let Some(pack_info) = pack_info {
                let result = installer
                    .apply_reality_profile_pack(&pack_info.manifest, Some(&workspace))
                    .await?;
                println!("✅ Reality profile pack applied successfully!");
                println!("   Personas: {}", result.personas_configured);
                println!("   Chaos Rules: {}", result.chaos_rules_applied);
                println!("   Latency Curves: {}", result.latency_curves_applied);
                println!("   Error Distributions: {}", result.error_distributions_applied);
                println!("   Data Mutations: {}", result.data_mutation_behaviors_applied);
                println!("   Protocol Behaviors: {}", result.protocol_behaviors_applied);
            } else {
                anyhow::bail!("Pack not found: {}", pack_name);
            }
        }
        RealityProfileCommands::Info { pack_name } => {
            let pack_info = installer.get_pack(&pack_name)?;
            if let Some(pack_info) = pack_info {
                println!("Reality Profile Pack: {}", pack_info.manifest.name);
                println!("Version: {}", pack_info.manifest.version);
                println!("Domain: {}", pack_info.manifest.domain);
                println!("Description: {}", pack_info.manifest.description);
                println!("Personas: {}", pack_info.manifest.personas.len());
                println!("Chaos Rules: {}", pack_info.manifest.chaos_rules.len());
                println!("Latency Curves: {}", pack_info.manifest.latency_curves.len());
                println!("Error Distributions: {}", pack_info.manifest.error_distributions.len());
            } else {
                anyhow::bail!("Pack not found: {}", pack_name);
            }
        }
    }

    Ok(())
}

/// Helper function to get config file path
fn get_config_path() -> std::path::PathBuf {
    std::env::current_dir()
        .ok()
        .map(|d| {
            let yaml_path = d.join("mockforge.yaml");
            if yaml_path.exists() {
                yaml_path
            } else {
                let yml_path = d.join("mockforge.yml");
                if yml_path.exists() {
                    yml_path
                } else {
                    d.join("mockforge.yaml") // Default to .yaml for new files
                }
            }
        })
        .unwrap_or_else(|| std::path::PathBuf::from("mockforge.yaml"))
}

/// Helper function to load config with fallback to default
async fn load_config_with_fallback() -> anyhow::Result<(ServerConfig, std::path::PathBuf)> {
    let config_path = get_config_path();
    let config = if config_path.exists() {
        load_config(&config_path)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to load config: {}", e))?
    } else {
        ServerConfig::default()
    };
    Ok((config, config_path))
}

/// Helper function to save behavior rule to config
async fn save_behavior_rule_to_config(rule: BehaviorRule) -> anyhow::Result<()> {
    let (mut config, config_path) = load_config_with_fallback().await?;

    // Check if rule already exists and remove it
    config.behavioral_economics.rules.retain(|r| r.name != rule.name);

    // Add the new rule
    config.behavioral_economics.rules.push(rule);

    // Save config
    save_config(&config_path, &config)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to save config: {}", e))?;

    Ok(())
}

/// Helper function to remove behavior rule from config
async fn remove_behavior_rule_from_config(rule_name: &str) -> anyhow::Result<bool> {
    let (mut config, config_path) = load_config_with_fallback().await?;

    let initial_len = config.behavioral_economics.rules.len();
    config.behavioral_economics.rules.retain(|r| r.name != rule_name);
    let removed = config.behavioral_economics.rules.len() < initial_len;

    if removed {
        save_config(&config_path, &config)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to save config: {}", e))?;
    }

    Ok(removed)
}

/// Helper function to update behavioral economics engine enabled status
async fn update_behavioral_economics_enabled(enabled: bool) -> anyhow::Result<()> {
    let (mut config, config_path) = load_config_with_fallback().await?;

    config.behavioral_economics.enabled = enabled;

    save_config(&config_path, &config)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to save config: {}", e))?;

    Ok(())
}

/// Handle behavioral economics rule commands
pub async fn handle_behavior_rule_command(command: BehaviorRuleCommands) -> anyhow::Result<()> {
    use mockforge_core::behavioral_economics::{BehaviorAction, BehaviorCondition, BehaviorRule};

    match command {
        BehaviorRuleCommands::Add {
            name,
            rule_type,
            condition,
            threshold,
            endpoint,
            action,
            parameter,
            priority,
            script,
            script_language,
        } => {
            println!("➕ Adding behavior rule: {}", name);
            println!("   Endpoint: {}", endpoint);

            // Parse condition
            let behavior_condition = match condition.as_str() {
                "latency" => {
                    let threshold_ms = threshold.and_then(|t| t.parse::<u64>().ok()).unwrap_or(400);
                    BehaviorCondition::LatencyThreshold {
                        endpoint: endpoint.clone(),
                        threshold_ms,
                    }
                }
                "load" => {
                    let threshold_rps =
                        threshold.and_then(|t| t.parse::<f64>().ok()).unwrap_or(100.0);
                    BehaviorCondition::LoadPressure { threshold_rps }
                }
                "error-rate" => {
                    let error_threshold =
                        threshold.and_then(|t| t.parse::<f64>().ok()).unwrap_or(0.1);
                    BehaviorCondition::ErrorRate {
                        endpoint: endpoint.clone(),
                        threshold: error_threshold,
                    }
                }
                "always" => BehaviorCondition::Always,
                _ => anyhow::bail!("Unknown condition type: {}", condition),
            };

            // Parse action
            let behavior_action = match action.as_str() {
                "modify-conversion-rate" => {
                    let multiplier = parameter.and_then(|p| p.parse::<f64>().ok()).unwrap_or(0.8);
                    BehaviorAction::ModifyConversionRate { multiplier }
                }
                "decline-transaction" => {
                    let reason = parameter.unwrap_or_else(|| "behavioral_rule".to_string());
                    BehaviorAction::DeclineTransaction { reason }
                }
                "increase-churn" => {
                    let factor = parameter.and_then(|p| p.parse::<f64>().ok()).unwrap_or(1.5);
                    BehaviorAction::IncreaseChurnProbability { factor }
                }
                "change-status" => {
                    let status = parameter.and_then(|p| p.parse::<u16>().ok()).unwrap_or(500);
                    BehaviorAction::ChangeResponseStatus { status }
                }
                "noop" => BehaviorAction::NoOp,
                _ => anyhow::bail!("Unknown action type: {}", action),
            };

            // Create rule
            let rule = if rule_type == "scriptable" {
                if script.is_none() || script_language.is_none() {
                    anyhow::bail!("Scriptable rules require --script and --script-language");
                }
                BehaviorRule::scriptable(
                    name,
                    behavior_condition,
                    behavior_action,
                    priority,
                    script.unwrap(),
                    script_language.unwrap(),
                )
            } else {
                BehaviorRule::declarative(name, behavior_condition, behavior_action, priority)
            };

            // Save to config
            save_behavior_rule_to_config(rule.clone()).await?;

            println!("✅ Behavior rule added: {}", rule.name);
            println!("   Type: {:?}", rule.rule_type);
            println!("   Priority: {}", rule.priority);
        }
        BehaviorRuleCommands::List => {
            println!("Behavior rules:");
            let (config, _) = load_config_with_fallback().await?;

            if config.behavioral_economics.rules.is_empty() {
                println!("  No behavior rules configured");
            } else {
                for rule in &config.behavioral_economics.rules {
                    println!(
                        "  - {} ({:?}, priority: {})",
                        rule.name, rule.rule_type, rule.priority
                    );
                    match &rule.condition {
                        BehaviorCondition::LatencyThreshold {
                            endpoint,
                            threshold_ms,
                        } => {
                            println!("    Condition: latency > {}ms on {}", threshold_ms, endpoint);
                        }
                        BehaviorCondition::LoadPressure { threshold_rps } => {
                            println!("    Condition: load > {} req/s", threshold_rps);
                        }
                        BehaviorCondition::Always => {
                            println!("    Condition: always");
                        }
                        _ => {
                            println!("    Condition: {:?}", rule.condition);
                        }
                    }
                    println!("    Action: {:?}", rule.action);
                }
            }
        }
        BehaviorRuleCommands::Remove { name } => {
            println!("🗑️  Removing behavior rule: {}", name);
            match remove_behavior_rule_from_config(&name).await {
                Ok(true) => {
                    println!("✅ Rule removed");
                }
                Ok(false) => {
                    println!("⚠️  Rule '{}' not found", name);
                }
                Err(e) => {
                    anyhow::bail!("Failed to remove rule: {}", e);
                }
            }
        }
        BehaviorRuleCommands::Enable => {
            println!("✅ Behavioral economics engine enabled");
            update_behavioral_economics_enabled(true).await?;
        }
        BehaviorRuleCommands::Disable => {
            println!("❌ Behavioral economics engine disabled");
            update_behavioral_economics_enabled(false).await?;
        }
        BehaviorRuleCommands::Status => {
            println!("Behavioral Economics Engine Status:");
            let (config, _) = load_config_with_fallback().await?;

            println!("  Enabled: {}", config.behavioral_economics.enabled);
            println!("  Rules: {}", config.behavioral_economics.rules.len());
            println!("  Global Sensitivity: {}", config.behavioral_economics.global_sensitivity);
            println!(
                "  Evaluation Interval: {}ms",
                config.behavioral_economics.evaluation_interval_ms
            );

            if !config.behavioral_economics.rules.is_empty() {
                println!("\n  Active Rules:");
                for rule in &config.behavioral_economics.rules {
                    println!("    - {} (priority: {})", rule.name, rule.priority);
                }
            }
        }
    }

    Ok(())
}

/// Helper function to convert LearningMode to DriftLearningMode
fn learning_mode_to_drift_mode(
    mode: &str,
) -> anyhow::Result<mockforge_core::config::DriftLearningMode> {
    match mode {
        "behavioral" => Ok(mockforge_core::config::DriftLearningMode::Behavioral),
        "statistical" => Ok(mockforge_core::config::DriftLearningMode::Statistical),
        "hybrid" => Ok(mockforge_core::config::DriftLearningMode::Hybrid),
        _ => anyhow::bail!("Unknown learning mode: {}", mode),
    }
}

/// Helper function to update drift learning config
async fn update_drift_learning_config<F>(updater: F) -> anyhow::Result<()>
where
    F: FnOnce(&mut mockforge_core::config::DriftLearningConfig),
{
    let (mut config, config_path) = load_config_with_fallback().await?;

    updater(&mut config.drift_learning);

    save_config(&config_path, &config)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to save config: {}", e))?;

    Ok(())
}

/// Handle drift learning commands
pub async fn handle_drift_learning_command(command: DriftLearningCommands) -> anyhow::Result<()> {
    match command {
        DriftLearningCommands::Enable {
            sensitivity,
            min_samples,
            mode,
            persona_adaptation,
            traffic_mirroring,
        } => {
            println!("🧠 Enabling drift learning");
            println!("   Sensitivity: {}", sensitivity);
            println!("   Min Samples: {}", min_samples);
            println!("   Mode: {}", mode);
            println!("   Persona Adaptation: {}", persona_adaptation);
            println!("   Traffic Mirroring: {}", traffic_mirroring);

            let drift_mode = learning_mode_to_drift_mode(&mode)?;

            update_drift_learning_config(|config| {
                config.enabled = true;
                config.mode = drift_mode;
                config.sensitivity = sensitivity;
                config.min_samples = min_samples as u64;
                config.persona_adaptation = persona_adaptation;
                // Note: traffic_mirroring is not in DriftLearningConfig, may need to be added
            })
            .await?;

            println!("✅ Drift learning enabled");
        }
        DriftLearningCommands::Disable => {
            println!("❌ Disabling drift learning");
            update_drift_learning_config(|config| {
                config.enabled = false;
            })
            .await?;
            println!("✅ Drift learning disabled");
        }
        DriftLearningCommands::Status => {
            println!("Drift Learning Status:");
            let (config, _) = load_config_with_fallback().await?;

            println!("  Enabled: {}", config.drift_learning.enabled);
            println!("  Mode: {:?}", config.drift_learning.mode);
            println!("  Sensitivity: {}", config.drift_learning.sensitivity);
            println!("  Decay: {}", config.drift_learning.decay);
            println!("  Min Samples: {}", config.drift_learning.min_samples);
            println!("  Persona Adaptation: {}", config.drift_learning.persona_adaptation);
            println!(
                "  Persona Learning Configs: {}",
                config.drift_learning.persona_learning.len()
            );
            println!(
                "  Endpoint Learning Configs: {}",
                config.drift_learning.endpoint_learning.len()
            );

            if !config.drift_learning.persona_learning.is_empty() {
                println!("\n  Persona Learning:");
                for (persona_id, enabled) in &config.drift_learning.persona_learning {
                    println!(
                        "    - {}: {}",
                        persona_id,
                        if *enabled { "enabled" } else { "disabled" }
                    );
                }
            }

            if !config.drift_learning.endpoint_learning.is_empty() {
                println!("\n  Endpoint Learning:");
                for (endpoint, enabled) in &config.drift_learning.endpoint_learning {
                    println!(
                        "    - {}: {}",
                        endpoint,
                        if *enabled { "enabled" } else { "disabled" }
                    );
                }
            }
        }
        DriftLearningCommands::Endpoint { endpoint, enable } => {
            println!(
                "{} learning for endpoint: {}",
                if enable { "Enabling" } else { "Disabling" },
                endpoint
            );
            update_drift_learning_config(|config| {
                config.endpoint_learning.insert(endpoint, enable);
            })
            .await?;
            println!("✅ Endpoint learning configuration updated");
        }
        DriftLearningCommands::Persona { persona_id, enable } => {
            println!(
                "{} learning for persona: {}",
                if enable { "Enabling" } else { "Disabling" },
                persona_id
            );
            update_drift_learning_config(|config| {
                config.persona_learning.insert(persona_id, enable);
            })
            .await?;
            println!("✅ Persona learning configuration updated");
        }
    }

    Ok(())
}
