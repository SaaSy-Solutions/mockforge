//! Plugin management CLI commands

use clap::Subcommand;
use mockforge_plugin_loader::{
    InstallOptions, PluginInstaller, PluginLoaderConfig, PluginSource,
};
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum PluginCommands {
    /// Install a plugin from various sources
    Install {
        /// Plugin source (URL, Git repo, local path, or registry name)
        ///
        /// Examples:
        /// - https://example.com/plugin.zip
        /// - https://github.com/user/repo#v1.0.0
        /// - /path/to/local/plugin
        /// - auth-jwt@1.0.0
        source: String,

        /// Force reinstall even if plugin exists
        #[arg(long)]
        force: bool,

        /// Skip validation checks
        #[arg(long)]
        skip_validation: bool,

        /// Don't verify plugin signature
        #[arg(long)]
        no_verify: bool,

        /// Expected SHA-256 checksum (for URL sources)
        #[arg(long)]
        checksum: Option<String>,
    },

    /// Uninstall a plugin
    Uninstall {
        /// Plugin ID to uninstall
        plugin_id: String,
    },

    /// List installed plugins
    List {
        /// Show detailed information
        #[arg(short, long)]
        detailed: bool,
    },

    /// Show plugin information
    Info {
        /// Plugin ID
        plugin_id: String,
    },

    /// Update a plugin to the latest version
    Update {
        /// Plugin ID to update (or --all for all plugins)
        plugin_id: Option<String>,

        /// Update all installed plugins
        #[arg(long)]
        all: bool,
    },

    /// Validate a plugin without installing
    Validate {
        /// Plugin source (URL, Git repo, or local path)
        source: String,
    },

    /// Clear plugin download cache
    ClearCache {
        /// Show cache stats before clearing
        #[arg(long)]
        stats: bool,
    },

    /// Show cache statistics
    CacheStats {},

    /// Search for plugins in the registry (future)
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
}

/// Handle plugin commands
pub async fn handle_plugin_command(command: PluginCommands) -> anyhow::Result<()> {
    match command {
        PluginCommands::Install {
            source,
            force,
            skip_validation,
            no_verify,
            checksum,
        } => {
            println!("🔌 Installing plugin from: {}", source);

            let config = PluginLoaderConfig::default();
            let installer = PluginInstaller::new(config)?;

            let options = InstallOptions {
                force,
                skip_validation,
                verify_signature: !no_verify,
                expected_checksum: checksum,
            };

            match installer.install(&source, options).await {
                Ok(plugin_id) => {
                    println!("✅ Plugin installed successfully: {}", plugin_id);
                }
                Err(e) => {
                    eprintln!("❌ Failed to install plugin: {}", e);
                    std::process::exit(1);
                }
            }
        }

        PluginCommands::Uninstall { plugin_id } => {
            println!("🗑️  Uninstalling plugin: {}", plugin_id);

            let config = PluginLoaderConfig::default();
            let installer = PluginInstaller::new(config)?;

            let plugin_id = mockforge_plugin_core::PluginId::new(&plugin_id);

            match installer.uninstall(&plugin_id).await {
                Ok(_) => {
                    println!("✅ Plugin uninstalled successfully");
                }
                Err(e) => {
                    eprintln!("❌ Failed to uninstall plugin: {}", e);
                    std::process::exit(1);
                }
            }
        }

        PluginCommands::List { detailed } => {
            println!("📋 Installed plugins:");

            let config = PluginLoaderConfig::default();
            let installer = PluginInstaller::new(config)?;

            let plugins = installer.list_installed().await;

            if plugins.is_empty() {
                println!("  No plugins installed");
            } else {
                for plugin_id in plugins {
                    if detailed {
                        println!("  - {} (detailed info coming soon)", plugin_id);
                    } else {
                        println!("  - {}", plugin_id);
                    }
                }
            }
        }

        PluginCommands::Info { plugin_id } => {
            println!("ℹ️  Plugin information: {}", plugin_id);
            println!("  (Detailed plugin info coming soon)");
        }

        PluginCommands::Update { plugin_id, all } => {
            if all {
                println!("🔄 Updating all plugins...");
                let config = PluginLoaderConfig::default();
                let installer = PluginInstaller::new(config)?;

                match installer.update_all().await {
                    Ok(updated) => {
                        println!("✅ Updated {} plugins", updated.len());
                        for id in updated {
                            println!("  - {}", id);
                        }
                    }
                    Err(e) => {
                        eprintln!("❌ Failed to update plugins: {}", e);
                        std::process::exit(1);
                    }
                }
            } else if let Some(id) = plugin_id {
                println!("🔄 Updating plugin: {}", id);

                let config = PluginLoaderConfig::default();
                let installer = PluginInstaller::new(config)?;
                let plugin_id = mockforge_plugin_core::PluginId::new(&id);

                match installer.update(&plugin_id).await {
                    Ok(_) => {
                        println!("✅ Plugin updated successfully");
                    }
                    Err(e) => {
                        eprintln!("❌ Failed to update plugin: {}", e);
                        std::process::exit(1);
                    }
                }
            } else {
                eprintln!("❌ Please specify a plugin ID or use --all");
                std::process::exit(1);
            }
        }

        PluginCommands::Validate { source } => {
            println!("🔍 Validating plugin: {}", source);

            let config = PluginLoaderConfig::default();
            let installer = PluginInstaller::new(config)?;

            // Parse source to get plugin directory
            let plugin_source = PluginSource::parse(&source)?;

            match plugin_source {
                PluginSource::Local(path) => {
                    // For local paths, validate directly
                    let loader = mockforge_plugin_loader::PluginLoader::new(config);
                    match loader.validate_plugin(&path).await {
                        Ok(manifest) => {
                            println!("✅ Plugin is valid!");
                            println!("   ID: {}", manifest.info.id);
                            println!("   Version: {}", manifest.info.version);
                            println!("   Name: {}", manifest.info.name);
                        }
                        Err(e) => {
                            eprintln!("❌ Plugin validation failed: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                _ => {
                    println!("ℹ️  Validation for remote sources requires downloading first");
                    println!("   Use: mockforge plugin install {} --skip-validation", source);
                }
            }
        }

        PluginCommands::ClearCache { stats } => {
            let config = PluginLoaderConfig::default();
            let installer = PluginInstaller::new(config)?;

            if stats {
                match installer.get_cache_stats().await {
                    Ok(stats) => {
                        println!("📊 Cache statistics before clearing:");
                        println!("   Download cache: {}", stats.download_cache_formatted());
                        println!("   Git cache: {}", stats.git_cache_formatted());
                        println!("   Total: {}", stats.total_formatted());
                    }
                    Err(e) => {
                        eprintln!("⚠️  Failed to get cache stats: {}", e);
                    }
                }
                println!();
            }

            println!("🗑️  Clearing plugin caches...");

            match installer.clear_caches().await {
                Ok(_) => {
                    println!("✅ Caches cleared successfully");
                }
                Err(e) => {
                    eprintln!("❌ Failed to clear caches: {}", e);
                    std::process::exit(1);
                }
            }
        }

        PluginCommands::CacheStats {} => {
            let config = PluginLoaderConfig::default();
            let installer = PluginInstaller::new(config)?;

            match installer.get_cache_stats().await {
                Ok(stats) => {
                    println!("📊 Plugin cache statistics:");
                    println!("   Download cache: {}", stats.download_cache_formatted());
                    println!("   Git cache: {}", stats.git_cache_formatted());
                    println!("   Total: {}", stats.total_formatted());
                }
                Err(e) => {
                    eprintln!("❌ Failed to get cache stats: {}", e);
                    std::process::exit(1);
                }
            }
        }

        PluginCommands::Search { query, category, limit } => {
            println!("🔍 Searching plugins: {}", query);
            if let Some(cat) = category {
                println!("   Category: {}", cat);
            }
            println!("   (Plugin search not yet implemented)");
            println!("   Future: Will search the plugin marketplace");
        }
    }

    Ok(())
}
