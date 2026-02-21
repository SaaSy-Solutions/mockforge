//! Plugin management CLI commands

use clap::Subcommand;
use mockforge_plugin_loader::{InstallOptions, PluginInstaller, PluginLoaderConfig, PluginSource};
use std::fs;
use std::path::{Path, PathBuf};

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

    /// Initialize a new plugin project
    Init {
        /// Plugin name
        name: String,

        /// Plugin type (template, auth, datasource, response, webhook, chaos)
        #[arg(short, long, default_value = "template")]
        plugin_type: String,

        /// Output directory (defaults to plugin name)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Overwrite existing directory
        #[arg(long)]
        force: bool,
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
            let _installer = PluginInstaller::new(config.clone())?;

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

        PluginCommands::Search {
            query,
            category,
            limit,
        } => {
            println!("🔍 Searching plugins: {}", query);
            if let Some(cat) = &category {
                println!("   Category: {}", cat);
            }

            let config = PluginLoaderConfig::default();
            let installer = PluginInstaller::new(config)?;
            let query_lower = query.to_lowercase();
            let category_lower = category.as_ref().map(|c| c.to_lowercase());

            let mut matches = installer
                .list_plugins_with_metadata()
                .await
                .into_iter()
                .filter(|(plugin_id, metadata)| {
                    let source_kind = match &metadata.source {
                        PluginSource::Local(_) => "local",
                        PluginSource::Url { .. } => "url",
                        PluginSource::Git(_) => "git",
                        PluginSource::Registry { .. } => "registry",
                    };

                    let source_text = format!("{:?}", metadata.source).to_lowercase();
                    let searchable = format!(
                        "{} {} {} {}",
                        plugin_id,
                        metadata.version,
                        source_kind,
                        source_text
                    )
                    .to_lowercase();
                    let query_match = searchable.contains(&query_lower);
                    let category_match = category_lower
                        .as_ref()
                        .is_none_or(|cat| source_kind.contains(cat));

                    query_match && category_match
                })
                .collect::<Vec<_>>();

            matches.sort_by(|a, b| a.0.as_str().cmp(b.0.as_str()));

            if matches.is_empty() {
                println!("   No installed plugins matched your query");
                return Ok(());
            }

            println!("   Found {} installed plugin(s):", matches.len().min(limit));
            for (plugin_id, metadata) in matches.into_iter().take(limit) {
                let source_kind = match &metadata.source {
                    PluginSource::Local(_) => "local",
                    PluginSource::Url { .. } => "url",
                    PluginSource::Git(_) => "git",
                    PluginSource::Registry { .. } => "registry",
                };
                println!(
                    "   - {} v{} [{}] source={:?}",
                    plugin_id, metadata.version, source_kind, metadata.source
                );
            }
        }
        PluginCommands::Init {
            name,
            plugin_type,
            output,
            force,
        } => {
            handle_plugin_init(name, plugin_type, output, force).await?;
        }
    }

    Ok(())
}

/// Handle plugin initialization
async fn handle_plugin_init(
    name: String,
    plugin_type: String,
    output: Option<PathBuf>,
    force: bool,
) -> anyhow::Result<()> {
    use std::fs;

    use std::process::Command;

    // Validate plugin type
    let valid_types = [
        "template",
        "auth",
        "datasource",
        "response",
        "webhook",
        "chaos",
    ];
    if !valid_types.contains(&plugin_type.as_str()) {
        anyhow::bail!(
            "Invalid plugin type: {}. Valid types: {}",
            plugin_type,
            valid_types.join(", ")
        );
    }

    // Determine output directory
    let output_dir = output.unwrap_or_else(|| PathBuf::from(&name));

    if output_dir.exists() && !force {
        anyhow::bail!(
            "Directory '{}' already exists. Use --force to overwrite.",
            output_dir.display()
        );
    }

    // Remove existing directory if force is enabled
    if output_dir.exists() && force {
        fs::remove_dir_all(&output_dir)?;
    }

    println!("🚀 Creating new {} plugin: {}", plugin_type, name);

    // Try to use cargo-generate if available
    let cargo_generate_available = Command::new("cargo-generate").arg("--version").output().is_ok();

    if cargo_generate_available {
        // Use cargo-generate to scaffold the project
        use_cargo_generate(&name, &plugin_type, &output_dir).await?;
    } else {
        // Fall back to manual file copying
        println!("⚠️  cargo-generate not found, using manual template copying");
        println!(
            "   Install cargo-generate for better template support: cargo install cargo-generate"
        );

        // Get template directory
        let template_dir = get_template_dir()?;

        // Copy template files
        copy_template_files(&template_dir, &output_dir, &name, &plugin_type)?;

        // Generate plugin-specific code
        generate_plugin_code(&output_dir, &name, &plugin_type)?;
    }

    println!("✅ Plugin project created successfully!");
    println!("\nNext steps:");
    println!("  1. cd {}", output_dir.display());
    println!("  2. Review and customize src/lib.rs");
    println!("  3. Update plugin.yaml with your configuration");
    println!("  4. Build: cargo build --target wasm32-wasi --release");
    println!("  5. Test: cargo test");
    println!("  6. Install: mockforge plugin install .");

    Ok(())
}

/// Use cargo-generate to scaffold a plugin project
async fn use_cargo_generate(
    plugin_name: &str,
    plugin_type: &str,
    output_dir: &Path,
) -> anyhow::Result<()> {
    use std::process::Command;

    // Get template directory
    let template_dir = get_template_dir()?;

    // Convert to absolute path
    let template_path = template_dir
        .canonicalize()
        .or_else(|_| std::env::current_dir().map(|cwd| cwd.join(&template_dir)))?;

    println!("  📦 Using cargo-generate to scaffold plugin...");

    // Generate a title from the name (capitalize first letter of each word)
    let plugin_title = plugin_name
        .split('-')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    // Build cargo-generate command with non-interactive mode
    let mut cmd = Command::new("cargo-generate");
    cmd.arg("generate")
        .arg("--path")
        .arg(&template_path)
        .arg("--name")
        .arg(plugin_name)
        .arg("--silent") // Non-interactive mode
        // Define template variables using --define flags
        .arg("--define")
        .arg(format!("plugin_name={}", plugin_name))
        .arg("--define")
        .arg(format!("plugin_type={}", plugin_type))
        .arg("--define")
        .arg(format!("plugin_title={}", plugin_title))
        .arg("--define")
        .arg(format!("plugin_description=MockForge {} plugin", plugin_type))
        .arg("--define")
        .arg("author_name=Your Name")
        .arg("--define")
        .arg("author_email=your.email@example.com")
        .arg("--define")
        .arg("max_memory_mb=10")
        .arg("--define")
        .arg("max_cpu_time_ms=1000")
        .arg("--define")
        .arg("allow_network=false")
        .arg("--define")
        .arg("allow_filesystem=false");

    // Set the output directory parent (cargo-generate creates the directory)
    // cargo-generate creates the directory with the name specified by --name
    let parent_dir = output_dir.parent().unwrap_or_else(|| Path::new("."));

    cmd.current_dir(parent_dir);

    // Execute cargo-generate
    let output = cmd.output().map_err(|e| {
        anyhow::anyhow!(
            "Failed to execute cargo-generate: {}. Install it with: cargo install cargo-generate",
            e
        )
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        eprintln!("cargo-generate stderr: {}", stderr);
        eprintln!("cargo-generate stdout: {}", stdout);
        anyhow::bail!("cargo-generate failed to create plugin project");
    }

    // If output directory name doesn't match, rename it
    let generated_dir = parent_dir.join(plugin_name);
    if generated_dir != output_dir && generated_dir.exists() {
        if output_dir.exists() {
            fs::remove_dir_all(output_dir)?;
        }
        fs::rename(&generated_dir, output_dir)?;
    }

    println!("  ✓ Plugin scaffolded using cargo-generate");

    // Generate plugin-specific code (still needed for type-specific implementations)
    generate_plugin_code(output_dir, plugin_name, plugin_type)?;

    Ok(())
}

/// Get the plugin template directory
fn get_template_dir() -> anyhow::Result<PathBuf> {
    // Try to find template directory relative to CLI binary
    let current_exe = std::env::current_exe()?;
    let mut template_dir = current_exe
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Cannot determine executable directory"))?
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Cannot determine parent directory"))?
        .join("templates")
        .join("plugin-template");

    // If not found, try relative to workspace root
    if !template_dir.exists() {
        template_dir = PathBuf::from("templates/plugin-template");
    }

    if !template_dir.exists() {
        anyhow::bail!("Plugin template directory not found at: {}", template_dir.display());
    }

    Ok(template_dir)
}

/// Copy template files to output directory
fn copy_template_files(
    template_dir: &Path,
    output_dir: &Path,
    plugin_name: &str,
    plugin_type: &str,
) -> anyhow::Result<()> {
    use std::fs;

    // Copy Cargo.toml
    let cargo_toml_src = template_dir.join("Cargo.toml");
    if cargo_toml_src.exists() {
        let mut cargo_content = fs::read_to_string(&cargo_toml_src)?;
        // Replace template variables
        cargo_content = cargo_content.replace("{{plugin_name}}", &plugin_name.replace("-", "_"));
        cargo_content = cargo_content.replace("{{author_name}}", "Your Name");
        cargo_content = cargo_content.replace("{{author_email}}", "your.email@example.com");
        cargo_content = cargo_content
            .replace("{{plugin_description}}", &format!("MockForge {} plugin", plugin_type));

        fs::write(output_dir.join("Cargo.toml"), cargo_content)?;
        println!("  ✓ Created Cargo.toml");
    }

    // Copy plugin.yaml
    let plugin_yaml_src = template_dir.join("plugin.yaml");
    if plugin_yaml_src.exists() {
        let mut yaml_content = fs::read_to_string(&plugin_yaml_src)?;
        // Replace template variables
        yaml_content = yaml_content.replace("{{plugin_name}}", plugin_name);
        yaml_content = yaml_content.replace("{{plugin_title}}", &format!("{} Plugin", plugin_name));
        yaml_content = yaml_content
            .replace("{{plugin_description}}", &format!("MockForge {} plugin", plugin_type));
        yaml_content = yaml_content.replace("{{plugin_type}}", plugin_type);
        yaml_content = yaml_content.replace("{{author_name}}", "Your Name");
        yaml_content = yaml_content.replace("{{author_email}}", "your.email@example.com");
        yaml_content = yaml_content.replace("{{allow_network}}", "false");
        yaml_content = yaml_content.replace("{{allow_filesystem}}", "false");
        yaml_content = yaml_content.replace("{{max_memory_mb}}", "10");
        yaml_content = yaml_content.replace("{{max_cpu_time_ms}}", "100");

        fs::write(output_dir.join("plugin.yaml"), yaml_content)?;
        println!("  ✓ Created plugin.yaml");
    }

    // Copy README.md
    let readme_src = template_dir.join("README.md");
    if readme_src.exists() {
        let mut readme_content = fs::read_to_string(&readme_src)?;
        // Replace template variables
        readme_content =
            readme_content.replace("{{plugin_title}}", &format!("{} Plugin", plugin_name));
        readme_content = readme_content
            .replace("{{plugin_description}}", &format!("MockForge {} plugin", plugin_type));
        readme_content = readme_content.replace("{{plugin_name}}", plugin_name);
        readme_content = readme_content.replace("{{plugin_type}}", plugin_type);

        fs::write(output_dir.join("README.md"), readme_content)?;
        println!("  ✓ Created README.md");
    }

    // Create src directory
    let src_dir = output_dir.join("src");
    fs::create_dir_all(&src_dir)?;

    // Copy lib.rs template
    let lib_rs_src = template_dir.join("src").join("lib.rs");
    if lib_rs_src.exists() {
        let lib_content = fs::read_to_string(&lib_rs_src)?;
        fs::write(src_dir.join("lib.rs"), lib_content)?;
        println!("  ✓ Created src/lib.rs");
    }

    // Create tests directory
    let tests_dir = output_dir.join("tests");
    fs::create_dir_all(&tests_dir)?;

    Ok(())
}

/// Generate plugin-specific code based on type
fn generate_plugin_code(
    output_dir: &Path,
    plugin_name: &str,
    plugin_type: &str,
) -> anyhow::Result<()> {
    use std::fs;

    let src_dir = output_dir.join("src");
    let lib_rs_path = src_dir.join("lib.rs");

    // Read existing lib.rs
    let mut lib_content = fs::read_to_string(&lib_rs_path)?;

    // Generate type-specific implementation
    let type_impl = match plugin_type {
        "auth" => generate_auth_plugin_code(plugin_name),
        "datasource" => generate_datasource_plugin_code(plugin_name),
        "response" => generate_response_plugin_code(plugin_name),
        "webhook" => generate_webhook_plugin_code(plugin_name),
        "chaos" => generate_chaos_plugin_code(plugin_name),
        _ => generate_template_plugin_code(plugin_name), // template is default
    };

    // Replace the implementation section
    // This is a simplified approach - in production, you'd use a proper template engine
    lib_content = type_impl;

    fs::write(&lib_rs_path, lib_content)?;

    Ok(())
}

/// Generate auth plugin code
fn generate_auth_plugin_code(plugin_name: &str) -> String {
    format!(
        r#"//! {} - Authentication Plugin
//!
//! A MockForge authentication plugin

use mockforge_plugin_core::*;
use serde::{{Deserialize, Serialize}};
use std::collections::HashMap;

/// Plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {{
    // Add your configuration fields here
}}

impl Default for PluginConfig {{
    fn default() -> Self {{
        Self {{
            // Set default values
        }}
    }}
}}

/// Main plugin struct
#[derive(Debug)]
pub struct {}Plugin {{
    config: PluginConfig,
}}

impl {}Plugin {{
    /// Create a new plugin instance
    pub fn new(config: PluginConfig) -> Self {{
        Self {{ config }}
    }}
}}

// Auth Plugin Implementation
#[async_trait::async_trait]
impl AuthPlugin for {}Plugin {{
    async fn authenticate(
        &self,
        context: &PluginContext,
        credentials: &AuthCredentials,
    ) -> PluginResult<AuthResult> {{
        // TODO: Implement your authentication logic
        // Example: Check credentials against your auth system

        match credentials.credential_type.as_str() {{
            "bearer" => {{
                // Validate bearer token
                let token = credentials.credentials.get("token")
                    .ok_or_else(|| "Missing token")?;

                // Token validation: Check format and basic structure
                if token.is_empty() {{
                    return PluginResult::failure("Invalid token: token is empty".to_string(), 401);
                }}

                // Basic token format validation
                // JWT tokens have 3 parts separated by dots
                if token.contains('.') {{
                    let parts: Vec<&str> = token.split('.').collect();
                    if parts.len() != 3 {{
                        return PluginResult::failure(
                            "Invalid token: JWT format invalid (expected 3 parts)".to_string(),
                            401
                        );
                    }}
                    // Basic validation: JWT parts should not be empty
                    for part in &parts[0..2] {{
                        if part.is_empty() {{
                            return PluginResult::failure(
                                "Invalid token: JWT parts cannot be empty".to_string(),
                                401
                            );
                        }}
                    }}
                }}

                // Additional validation: Check token length (reasonable bounds)
                if token.len() < 10 {{
                    return PluginResult::failure(
                        "Invalid token: token too short".to_string(),
                        401
                    );
                }}

                if token.len() > 8192 {{
                    return PluginResult::failure(
                        "Invalid token: token too long".to_string(),
                        401
                    );
                }}

                // TODO: Add signature verification for JWT tokens
                // TODO: Add token expiration check
                // TODO: Add token revocation check (if using a token store)

                // Extract user ID from token if possible (basic implementation)
                let user_id = if token.contains('.') {{
                    // Try to extract from JWT payload (simplified - production would decode properly)
                    // For now, use a hash of the token as user ID
                    format!("user_{{}}", token.chars().take(8).collect::<String>())
                }} else {{
                    "user123".to_string()
                }};

                PluginResult::success(AuthResult::Authenticated {{
                    user_id,
                    claims: HashMap::new(),
                }})
            }}
            "api_key" => {{
                // API Key authentication
                let api_key = credentials.credentials.get("api_key")
                    .ok_or_else(|| "Missing API key")?;

                if api_key.is_empty() {{
                    return PluginResult::failure("Invalid API key: key is empty".to_string(), 401);
                }}

                // Basic API key validation
                if api_key.len() < 8 {{
                    return PluginResult::failure(
                        "Invalid API key: key too short (minimum 8 characters)".to_string(),
                        401
                    );
                }}

                if api_key.len() > 256 {{
                    return PluginResult::failure(
                        "Invalid API key: key too long (maximum 256 characters)".to_string(),
                        401
                    );
                }}

                // TODO: Validate API key against configured keys
                // TODO: Check API key expiration
                // TODO: Check API key rate limits

                PluginResult::success(AuthResult::Authenticated {{
                    user_id: format!("api_user_{{}}", api_key.chars().take(8).collect::<String>()),
                    claims: HashMap::new(),
                }})
            }}
            "basic" => {{
                // Basic authentication (username/password)
                let username = credentials.credentials.get("username")
                    .ok_or_else(|| "Missing username")?;
                let password = credentials.credentials.get("password")
                    .ok_or_else(|| "Missing password")?;

                if username.is_empty() {{
                    return PluginResult::failure("Invalid username: username is empty".to_string(), 401);
                }}

                if password.is_empty() {{
                    return PluginResult::failure("Invalid password: password is empty".to_string(), 401);
                }}

                // Basic validation
                if username.len() < 3 {{
                    return PluginResult::failure(
                        "Invalid username: too short (minimum 3 characters)".to_string(),
                        401
                    );
                }}

                if password.len() < 6 {{
                    return PluginResult::failure(
                        "Invalid password: too short (minimum 6 characters)".to_string(),
                        401
                    );
                }}

                // TODO: Validate against user database
                // TODO: Check password hash
                // TODO: Implement rate limiting for failed attempts

                PluginResult::success(AuthResult::Authenticated {{
                    user_id: username.clone(),
                    claims: {{
                        let mut claims = HashMap::new();
                        claims.insert("username".to_string(), username.clone());
                        claims
                    }},
                }})
            }}
            "oauth2" => {{
                // OAuth2 authentication
                let access_token = credentials.credentials.get("access_token")
                    .ok_or_else(|| "Missing access token")?;

                if access_token.is_empty() {{
                    return PluginResult::failure(
                        "Invalid access token: token is empty".to_string(),
                        401
                    );
                }}

                // Basic token validation
                if access_token.len() < 10 {{
                    return PluginResult::failure(
                        "Invalid access token: token too short".to_string(),
                        401
                    );
                }}

                // TODO: Validate OAuth2 token with provider
                // TODO: Check token expiration
                // TODO: Verify token signature
                // TODO: Extract user info from token

                PluginResult::success(AuthResult::Authenticated {{
                    user_id: format!("oauth_user_{{}}", access_token.chars().take(8).collect::<String>()),
                    claims: HashMap::new(),
                }})
            }}
            _ => PluginResult::failure(
                format!("Unsupported credential type: {{}}", credentials.credential_type),
                400
            ),
        }}
    }}

    fn get_capabilities(&self) -> PluginCapabilities {{
        PluginCapabilities {{
            network: NetworkCapabilities {{
                allow_http_outbound: false, // Set to true if you need to call external auth services
                allowed_hosts: vec![],
            }},
            filesystem: FilesystemCapabilities {{
                allow_read: false,
                allow_write: false,
                allowed_paths: vec![],
            }},
            resources: PluginResources {{
                max_memory_bytes: 10 * 1024 * 1024, // 10MB
                max_cpu_time_ms: 100,
            }},
        }}
    }}
}}

// Export the plugin (required)
mockforge_plugin_core::export_plugin!({}Plugin);

#[cfg(test)]
mod tests {{
    use super::*;

    #[tokio::test]
    async fn test_authentication() {{
        let plugin = {}Plugin::new(PluginConfig::default());
        // Add your tests here
        assert!(true);
    }}
}}
"#,
        plugin_name,
        to_pascal_case(plugin_name),
        to_pascal_case(plugin_name),
        to_pascal_case(plugin_name),
        to_pascal_case(plugin_name),
        to_pascal_case(plugin_name),
    )
}

/// Generate datasource plugin code
fn generate_datasource_plugin_code(plugin_name: &str) -> String {
    format!(
        r#"//! {} - Data Source Plugin
//!
//! A MockForge data source plugin

use mockforge_plugin_core::*;
use serde::{{Deserialize, Serialize}};

/// Plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {{
    // Add your configuration fields here
    // Example: connection_string: String,
}}

impl Default for PluginConfig {{
    fn default() -> Self {{
        Self {{
            // Set default values
        }}
    }}
}}

/// Main plugin struct
#[derive(Debug)]
pub struct {}Plugin {{
    config: PluginConfig,
}}

impl {}Plugin {{
    /// Create a new plugin instance
    pub fn new(config: PluginConfig) -> Self {{
        Self {{ config }}
    }}
}}

// Data Source Plugin Implementation
#[async_trait::async_trait]
impl DataSourcePlugin for {}Plugin {{
    async fn query(
        &self,
        query: &str,
        context: &PluginContext,
    ) -> PluginResult<serde_json::Value> {{
        // TODO: Implement your data source query logic
        // Example: Query a database, CSV file, or external API

        // For now, return empty result
        PluginResult::success(serde_json::json!([]))
    }}

    fn get_capabilities(&self) -> PluginCapabilities {{
        PluginCapabilities {{
            network: NetworkCapabilities {{
                allow_http_outbound: false, // Set to true if querying external APIs
                allowed_hosts: vec![],
            }},
            filesystem: FilesystemCapabilities {{
                allow_read: true, // Set to true if reading files
                allow_write: false,
                allowed_paths: vec![],
            }},
            resources: PluginResources {{
                max_memory_bytes: 50 * 1024 * 1024, // 50MB
                max_cpu_time_ms: 1000,
            }},
        }}
    }}
}}

// Export the plugin (required)
mockforge_plugin_core::export_plugin!({}Plugin);

#[cfg(test)]
mod tests {{
    use super::*;

    #[tokio::test]
    async fn test_query() {{
        let plugin = {}Plugin::new(PluginConfig::default());
        // Add your tests here
        assert!(true);
    }}
}}
"#,
        plugin_name,
        to_pascal_case(plugin_name),
        to_pascal_case(plugin_name),
        to_pascal_case(plugin_name),
        to_pascal_case(plugin_name),
        to_pascal_case(plugin_name),
    )
}

/// Generate response plugin code
fn generate_response_plugin_code(plugin_name: &str) -> String {
    format!(
        r#"//! {} - Response Plugin
//!
//! A MockForge response generation plugin

use mockforge_plugin_core::*;
use serde::{{Deserialize, Serialize}};

/// Plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {{
    // Add your configuration fields here
}}

impl Default for PluginConfig {{
    fn default() -> Self {{
        Self {{
            // Set default values
        }}
    }}
}}

/// Main plugin struct
#[derive(Debug)]
pub struct {}Plugin {{
    config: PluginConfig,
}}

impl {}Plugin {{
    /// Create a new plugin instance
    pub fn new(config: PluginConfig) -> Self {{
        Self {{ config }}
    }}
}}

// Response Plugin Implementation
#[async_trait::async_trait]
impl ResponsePlugin for {}Plugin {{
    async fn generate_response(
        &self,
        request: &PluginRequest,
        context: &PluginContext,
    ) -> PluginResult<PluginResponse> {{
        // TODO: Implement your response generation logic
        // Example: Generate dynamic responses based on request

        let body = serde_json::json!({{
            "message": "Generated by {} plugin",
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }});

        PluginResult::success(PluginResponse {{
            status: 200,
            headers: std::collections::HashMap::new(),
            body: serde_json::to_vec(&body).unwrap(),
        }})
    }}

    fn get_capabilities(&self) -> PluginCapabilities {{
        PluginCapabilities {{
            network: NetworkCapabilities {{
                allow_http_outbound: false,
                allowed_hosts: vec![],
            }},
            filesystem: FilesystemCapabilities {{
                allow_read: false,
                allow_write: false,
                allowed_paths: vec![],
            }},
            resources: PluginResources {{
                max_memory_bytes: 10 * 1024 * 1024, // 10MB
                max_cpu_time_ms: 100,
            }},
        }}
    }}
}}

// Export the plugin (required)
mockforge_plugin_core::export_plugin!({}Plugin);

#[cfg(test)]
mod tests {{
    use super::*;

    #[tokio::test]
    async fn test_response_generation() {{
        let plugin = {}Plugin::new(PluginConfig::default());
        // Add your tests here
        assert!(true);
    }}
}}
"#,
        plugin_name,
        to_pascal_case(plugin_name),
        to_pascal_case(plugin_name),
        to_pascal_case(plugin_name),
        plugin_name,
        to_pascal_case(plugin_name),
        to_pascal_case(plugin_name),
    )
}

/// Generate webhook plugin code
fn generate_webhook_plugin_code(plugin_name: &str) -> String {
    format!(
        r#"//! {} - Webhook Plugin
//!
//! A MockForge webhook handler plugin

use mockforge_plugin_core::*;
use serde::{{Deserialize, Serialize}};

/// Plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {{
    // Add your configuration fields here
    // Example: webhook_url: String,
}}

impl Default for PluginConfig {{
    fn default() -> Self {{
        Self {{
            // Set default values
        }}
    }}
}}

/// Main plugin struct
#[derive(Debug)]
pub struct {}Plugin {{
    config: PluginConfig,
}}

impl {}Plugin {{
    /// Create a new plugin instance
    pub fn new(config: PluginConfig) -> Self {{
        Self {{ config }}
    }}
}}

// Webhook Plugin Implementation
#[async_trait::async_trait]
impl WebhookPlugin for {}Plugin {{
    async fn handle_webhook(
        &self,
        event: &WebhookEvent,
        context: &PluginContext,
    ) -> PluginResult<()> {{
        // TODO: Implement your webhook handling logic
        // Example: Process webhook event and trigger actions

        println!("Received webhook event: {{:?}}", event);

        PluginResult::success(())
    }}

    fn get_capabilities(&self) -> PluginCapabilities {{
        PluginCapabilities {{
            network: NetworkCapabilities {{
                allow_http_outbound: true, // Webhooks typically need network access
                allowed_hosts: vec![],
            }},
            filesystem: FilesystemCapabilities {{
                allow_read: false,
                allow_write: false,
                allowed_paths: vec![],
            }},
            resources: PluginResources {{
                max_memory_bytes: 10 * 1024 * 1024, // 10MB
                max_cpu_time_ms: 1000,
            }},
        }}
    }}
}}

// Export the plugin (required)
mockforge_plugin_core::export_plugin!({}Plugin);

#[cfg(test)]
mod tests {{
    use super::*;

    #[tokio::test]
    async fn test_webhook_handling() {{
        let plugin = {}Plugin::new(PluginConfig::default());
        // Add your tests here
        assert!(true);
    }}
}}
"#,
        plugin_name,
        to_pascal_case(plugin_name),
        to_pascal_case(plugin_name),
        to_pascal_case(plugin_name),
        to_pascal_case(plugin_name),
        to_pascal_case(plugin_name),
    )
}

/// Generate chaos plugin code
fn generate_chaos_plugin_code(plugin_name: &str) -> String {
    format!(
        r#"//! {} - Chaos Plugin
//!
//! A MockForge chaos engineering plugin

use mockforge_plugin_core::*;
use serde::{{Deserialize, Serialize}};

/// Plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {{
    // Add your configuration fields here
    // Example: failure_rate: f64,
}}

impl Default for PluginConfig {{
    fn default() -> Self {{
        Self {{
            // Set default values
        }}
    }}
}}

/// Main plugin struct
#[derive(Debug)]
pub struct {}Plugin {{
    config: PluginConfig,
}}

impl {}Plugin {{
    /// Create a new plugin instance
    pub fn new(config: PluginConfig) -> Self {{
        Self {{ config }}
    }}
}}

// Chaos Plugin Implementation
#[async_trait::async_trait]
impl ChaosPlugin for {}Plugin {{
    async fn inject_chaos(
        &self,
        request: &PluginRequest,
        context: &PluginContext,
    ) -> PluginResult<ChaosResult> {{
        // TODO: Implement your chaos injection logic
        // Example: Randomly inject latency, errors, or modify responses

        // For now, return no chaos
        PluginResult::success(ChaosResult::None)
    }}

    fn get_capabilities(&self) -> PluginCapabilities {{
        PluginCapabilities {{
            network: NetworkCapabilities {{
                allow_http_outbound: false,
                allowed_hosts: vec![],
            }},
            filesystem: FilesystemCapabilities {{
                allow_read: false,
                allow_write: false,
                allowed_paths: vec![],
            }},
            resources: PluginResources {{
                max_memory_bytes: 10 * 1024 * 1024, // 10MB
                max_cpu_time_ms: 50, // Chaos should be fast
            }},
        }}
    }}
}}

// Export the plugin (required)
mockforge_plugin_core::export_plugin!({}Plugin);

#[cfg(test)]
mod tests {{
    use super::*;

    #[tokio::test]
    async fn test_chaos_injection() {{
        let plugin = {}Plugin::new(PluginConfig::default());
        // Add your tests here
        assert!(true);
    }}
}}
"#,
        plugin_name,
        to_pascal_case(plugin_name),
        to_pascal_case(plugin_name),
        to_pascal_case(plugin_name),
        to_pascal_case(plugin_name),
        to_pascal_case(plugin_name),
    )
}

/// Generate template plugin code (default)
fn generate_template_plugin_code(plugin_name: &str) -> String {
    format!(
        r#"//! {} - Template Plugin
//!
//! A MockForge template function plugin

use mockforge_plugin_core::*;
use serde::{{Deserialize, Serialize}};
use std::collections::HashMap;

/// Plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {{
    // Add your configuration fields here
}}

impl Default for PluginConfig {{
    fn default() -> Self {{
        Self {{
            // Set default values
        }}
    }}
}}

/// Main plugin struct
#[derive(Debug)]
pub struct {}Plugin {{
    config: PluginConfig,
}}

impl {}Plugin {{
    /// Create a new plugin instance
    pub fn new(config: PluginConfig) -> Self {{
        Self {{ config }}
    }}
}}

// Template Plugin Implementation
#[async_trait::async_trait]
impl TemplatePlugin for {}Plugin {{
    async fn execute_function(
        &self,
        function_name: &str,
        args: &[serde_json::Value],
        context: &PluginContext,
    ) -> PluginResult<serde_json::Value> {{
        match function_name {{
            "example_function" => {{
                // TODO: Implement your template function
                if args.is_empty() {{
                    return PluginResult::failure("Missing argument".to_string(), 0);
                }}

                let input = args[0].as_str()
                    .ok_or_else(|| "Argument must be a string")?;

                // Example: Convert to uppercase
                let result = input.to_uppercase();
                PluginResult::success(serde_json::json!(result))
            }}
            _ => PluginResult::failure(
                format!("Unknown function: {{}}", function_name),
                0
            ),
        }}
    }}

    fn get_functions(&self) -> Vec<TemplateFunction> {{
        vec![TemplateFunction {{
            name: "example_function".to_string(),
            description: "An example template function".to_string(),
            parameters: vec![FunctionParameter {{
                name: "input".to_string(),
                param_type: "string".to_string(),
                required: true,
                description: "Input string".to_string(),
            }}],
            return_type: "string".to_string(),
        }}]
    }}

    fn get_capabilities(&self) -> PluginCapabilities {{
        PluginCapabilities {{
            network: NetworkCapabilities {{
                allow_http_outbound: false,
                allowed_hosts: vec![],
            }},
            filesystem: FilesystemCapabilities {{
                allow_read: false,
                allow_write: false,
                allowed_paths: vec![],
            }},
            resources: PluginResources {{
                max_memory_bytes: 10 * 1024 * 1024, // 10MB
                max_cpu_time_ms: 100,
            }},
        }}
    }}
}}

// Export the plugin (required)
mockforge_plugin_core::export_plugin!({}Plugin);

#[cfg(test)]
mod tests {{
    use super::*;

    #[tokio::test]
    async fn test_plugin_creation() {{
        let plugin = {}Plugin::new(PluginConfig::default());
        // Add your tests here
        assert!(true);
    }}
}}
"#,
        plugin_name,
        to_pascal_case(plugin_name),
        to_pascal_case(plugin_name),
        to_pascal_case(plugin_name),
        to_pascal_case(plugin_name),
        to_pascal_case(plugin_name),
    )
}

/// Convert kebab-case to PascalCase
fn to_pascal_case(s: &str) -> String {
    s.split('-')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect()
}
