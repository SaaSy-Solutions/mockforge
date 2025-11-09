//! Scenario management CLI commands

use base64::{engine::general_purpose, Engine as _};
use clap::Subcommand;
use mockforge_core::config::{load_config, save_config, ServerConfig};
use mockforge_scenarios::{
    DomainPackInstaller, InstallOptions, ScenarioInstaller, ScenarioRegistry,
    ScenarioReviewSubmission,
};
use mockforge_vbr::entities::Entity;
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
                    }
                    None => {
                        eprintln!("❌ Pack '{}' not found", name);
                        std::process::exit(1);
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
                let entity = if let Some(ref state_machine) = entity_def.state_machine {
                    Entity::with_state_machine(
                        entity_def.name.clone(),
                        vbr_schema,
                        state_machine.clone(),
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
