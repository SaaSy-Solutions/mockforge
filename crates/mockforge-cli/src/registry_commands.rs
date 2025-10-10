//! Registry-related CLI commands

use anyhow::{Context, Result};
use clap::Subcommand;
use colored::Colorize;
use mockforge_plugin_core::manifest::ManifestLoader;
use mockforge_plugin_loader::installer::{InstallOptions, PluginInstaller, PluginLoaderConfig};
use mockforge_plugin_registry::{
    api::{PublishRequest, RegistryClient},
    config::{load_config, set_token, clear_token},
    manifest::{validate_manifest, PluginManifest as RegistryPluginManifest},
    PluginCategory, SearchQuery, SortOrder,
};
use ring::digest::{Context, SHA256};
use std::fs;
use std::path::Path;
use tempfile;

/// Calculate SHA-256 checksum of data
fn calculate_checksum(data: &[u8]) -> String {
    let mut context = Context::new(&SHA256);
    context.update(data);
    let digest = context.finish();
    hex::encode(digest.as_ref())
}

/// Build plugin WASM module
async fn build_plugin_wasm(path: &str) -> Result<()> {
    use std::process::Command;

    // Check prerequisites
    check_cargo().context("Cargo not found")?;
    check_wasm_target().context("wasm32-wasi target not available")?;

    // Change to project directory
    std::env::set_current_dir(path)
        .with_context(|| format!("Failed to change to directory {}", path))?;

    // Build the plugin
    let status = Command::new("cargo")
        .args(["build", "--target", "wasm32-wasi", "--release"])
        .status()
        .context("Failed to execute cargo build")?;

    if !status.success() {
        anyhow::bail!("Plugin build failed");
    }

    Ok(())
}

/// Convert core PluginManifest to registry PluginManifest
fn convert_to_registry_manifest(
    core_manifest: &mockforge_plugin_core::PluginManifest,
) -> Result<RegistryPluginManifest> {
    use mockforge_plugin_registry::manifest::{AuthorInfo, PluginCategory};

    let plugin_info = &core_manifest.plugin;

    // Convert author
    let author = if let Some(core_author) = &plugin_info.author {
        AuthorInfo {
            name: core_author.name.clone(),
            email: core_author.email.clone(),
            url: core_author.url.clone(),
        }
    } else {
        AuthorInfo {
            name: "Unknown".to_string(),
            email: None,
            url: None,
        }
    };

    // Convert category (map from types or use default)
    let category = if plugin_info.types.contains(&"auth".to_string()) {
        PluginCategory::Auth
    } else if plugin_info.types.contains(&"template".to_string()) {
        PluginCategory::Template
    } else if plugin_info.types.contains(&"response".to_string()) {
        PluginCategory::Response
    } else if plugin_info.types.contains(&"datasource".to_string()) {
        PluginCategory::DataSource
    } else if plugin_info.types.contains(&"middleware".to_string()) {
        PluginCategory::Middleware
    } else if plugin_info.types.contains(&"testing".to_string()) {
        PluginCategory::Testing
    } else if plugin_info.types.contains(&"observability".to_string()) {
        PluginCategory::Observability
    } else {
        PluginCategory::Other
    };

    // Convert dependencies
    let mut dependencies = std::collections::HashMap::new();
    for dep in &core_manifest.dependencies {
        dependencies.insert(dep.id.to_string(), dep.version.clone());
    }

    // Extract min_mockforge_version from metadata
    let min_mockforge_version = core_manifest.metadata
        .get("runtime")
        .and_then(|runtime| runtime.as_object())
        .and_then(|obj| obj.get("min_mockforge_version"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Ok(RegistryPluginManifest {
        name: plugin_info.id.to_string(),
        version: plugin_info.version.to_string(),
        description: plugin_info.description.clone().unwrap_or_else(|| "No description".to_string()),
        author,
        license: plugin_info.license.clone().unwrap_or_else(|| "Unknown".to_string()),
        repository: plugin_info.repository.clone(),
        homepage: plugin_info.homepage.clone(),
        tags: plugin_info.keywords.clone(),
        category,
        min_mockforge_version,
        dependencies,
    })
}

/// Check if cargo is available
fn check_cargo() -> Result<()> {
    std::process::Command::new("cargo")
        .arg("--version")
        .output()
        .context("Cargo not found")?;
    Ok(())
}

/// Check if wasm32-wasi target is installed
fn check_wasm_target() -> Result<()> {
    let output = std::process::Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
        .context("Failed to check installed targets")?;

    let installed = String::from_utf8(output.stdout)
        .context("Failed to parse target list")?;

    if !installed.lines().any(|line| line.trim() == "wasm32-wasi") {
        anyhow::bail!("wasm32-wasi target not installed. Run: rustup target add wasm32-wasi");
    }

    Ok(())
}

#[derive(Debug, Subcommand)]
pub enum RegistryCommand {
    /// Search for plugins in the registry
    Search {
        /// Search query
        query: Option<String>,

        /// Filter by category
        #[arg(long)]
        category: Option<String>,

        /// Filter by tags (comma-separated)
        #[arg(long)]
        tags: Option<String>,

        /// Sort order (relevance, downloads, rating, recent, name)
        #[arg(long, default_value = "relevance")]
        sort: String,

        /// Page number (0-indexed)
        #[arg(long, default_value = "0")]
        page: usize,

        /// Results per page
        #[arg(long, default_value = "20")]
        per_page: usize,
    },

    /// Get detailed information about a plugin
    Info {
        /// Plugin name
        name: String,

        /// Show specific version
        #[arg(long)]
        version: Option<String>,
    },

    /// Install a plugin from the registry
    Install {
        /// Plugin name (optionally with version: name@version)
        plugin: String,

        /// Force reinstall even if already installed
        #[arg(long)]
        force: bool,
    },

    /// Publish a plugin to the registry
    Publish {
        /// Path to plugin package
        #[arg(default_value = ".")]
        path: String,

        /// Dry run (validate without publishing)
        #[arg(long)]
        dry_run: bool,
    },

    /// Yank a published version (remove from index)
    Yank {
        /// Plugin name
        name: String,

        /// Version to yank
        version: String,
    },

    /// Set registry API token
    Login {
        /// API token
        #[arg(long)]
        token: Option<String>,
    },

    /// Clear registry API token
    Logout,

    /// Show registry configuration
    Config,
}

pub async fn handle_registry_command(cmd: RegistryCommand) -> Result<()> {
    match cmd {
        RegistryCommand::Search {
            query,
            category,
            tags,
            sort,
            page,
            per_page,
        } => search_plugins(query, category, tags, sort, page, per_page).await,

        RegistryCommand::Info { name, version } => show_plugin_info(&name, version.as_deref()).await,

        RegistryCommand::Install { plugin, force } => install_from_registry(&plugin, force).await,

        RegistryCommand::Publish { path, dry_run } => publish_plugin(&path, dry_run).await,

        RegistryCommand::Yank { name, version } => yank_version(&name, &version).await,

        RegistryCommand::Login { token } => login(token).await,

        RegistryCommand::Logout => logout().await,

        RegistryCommand::Config => show_config().await,
    }
}

async fn search_plugins(
    query: Option<String>,
    category: Option<String>,
    tags: Option<String>,
    sort: String,
    page: usize,
    per_page: usize,
) -> Result<()> {
    let config = load_config().await?;
    let client = RegistryClient::new(config)?;

    let sort_order = match sort.as_str() {
        "downloads" => SortOrder::Downloads,
        "rating" => SortOrder::Rating,
        "recent" => SortOrder::Recent,
        "name" => SortOrder::Name,
        _ => SortOrder::Relevance,
    };

    let category_filter = category.and_then(|c| match c.to_lowercase().as_str() {
        "auth" => Some(PluginCategory::Auth),
        "template" => Some(PluginCategory::Template),
        "response" => Some(PluginCategory::Response),
        "datasource" => Some(PluginCategory::DataSource),
        "middleware" => Some(PluginCategory::Middleware),
        "testing" => Some(PluginCategory::Testing),
        "observability" => Some(PluginCategory::Observability),
        _ => None,
    });

    let tags_vec = tags
        .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();

    let search_query = SearchQuery {
        query,
        category: category_filter,
        tags: tags_vec,
        sort: sort_order,
        page,
        per_page,
    };

    let results = client.search(search_query).await?;

    if results.plugins.is_empty() {
        println!("{}", "No plugins found".yellow());
        return Ok(());
    }

    println!("\n{} {} plugins found\n", "📦".blue(), results.total);

    for plugin in &results.plugins {
        println!("{} {} v{}", "•".blue(), plugin.name.bold(), plugin.version);
        println!("  {}", plugin.description);
        println!(
            "  {} {} downloads • ⭐ {:.1}/5.0 • {}",
            "↓".green(),
            plugin.downloads,
            plugin.rating,
            plugin.license.dimmed()
        );
        if !plugin.tags.is_empty() {
            println!("  Tags: {}", plugin.tags.join(", ").dimmed());
        }
        println!();
    }

    let total_pages = (results.total + results.per_page - 1) / results.per_page;
    println!(
        "Page {}/{} • Showing {} results",
        results.page + 1,
        total_pages,
        results.plugins.len()
    );

    Ok(())
}

async fn show_plugin_info(name: &str, version: Option<&str>) -> Result<()> {
    let config = load_config().await?;
    let client = RegistryClient::new(config)?;

    let plugin = client.get_plugin(name).await?;

    println!("\n{} {}", "📦".blue(), plugin.name.bold());
    println!("Version: {}", plugin.version.green());
    println!("Category: {:?}", plugin.category);
    println!("License: {}", plugin.license);
    println!("\n{}", plugin.description);

    if !plugin.tags.is_empty() {
        println!("\nTags: {}", plugin.tags.join(", "));
    }

    println!("\n{}", "Statistics:".bold());
    println!("  Downloads: {}", plugin.downloads);
    println!("  Rating: ⭐ {:.1}/5.0 ({} reviews)", plugin.rating, plugin.reviews_count);
    println!("  Created: {}", plugin.created_at);
    println!("  Updated: {}", plugin.updated_at);

    if let Some(repo) = &plugin.repository {
        println!("\nRepository: {}", repo);
    }

    if let Some(homepage) = &plugin.homepage {
        println!("Homepage: {}", homepage);
    }

    println!("\n{}", "Available versions:".bold());
    for ver in &plugin.versions {
        if !ver.yanked {
            println!("  • {} ({})", ver.version, ver.published_at);
        }
    }

    if let Some(ver) = version {
        if let Some(version_entry) = plugin.versions.iter().find(|v| v.version == ver) {
            println!("\n{}", format!("Version {} details:", ver).bold());
            println!("  Download URL: {}", version_entry.download_url);
            println!("  Checksum: {}", version_entry.checksum);
            println!("  Size: {} bytes", version_entry.size);
        }
    }

    Ok(())
}

async fn install_from_registry(plugin_spec: &str, force: bool) -> Result<()> {
    let config = load_config().await?;
    let client = RegistryClient::new(config)?;

    // Parse plugin spec (name@version or just name)
    let (name, version) = if let Some(pos) = plugin_spec.find('@') {
        let (n, v) = plugin_spec.split_at(pos);
        (n, Some(&v[1..]))
    } else {
        (plugin_spec, None)
    };

    println!("{} Installing {} from registry...", "📦".blue(), name.bold());

    // Get plugin info
    let plugin = client.get_plugin(name).await?;

    // Determine version to install
    let target_version = version.unwrap_or(&plugin.version);

    let version_entry = plugin
        .versions
        .iter()
        .find(|v| v.version == target_version)
        .context(format!("Version {} not found", target_version))?;

    if version_entry.yanked {
        anyhow::bail!("Version {} has been yanked", target_version);
    }

    println!("{} Downloading version {}...", "↓".green(), target_version);

    // Download plugin
    let data = client.download(&version_entry.download_url).await?;

    // Verify checksum
    let calculated_checksum = calculate_checksum(&data);
    if calculated_checksum != version_entry.checksum {
        anyhow::bail!(
            "Checksum verification failed! Expected: {}, Got: {}",
            version_entry.checksum,
            calculated_checksum
        );
    }

    println!("{} Checksum verified", "✓".green());

    // Save to temporary file
    let temp_file = tempfile::NamedTempFile::new()
        .context("Failed to create temporary file")?;
    fs::write(&temp_file, &data)
        .context("Failed to write plugin data to temporary file")?;

    // Install plugin
    let loader_config = PluginLoaderConfig::default();
    let installer = PluginInstaller::new(loader_config)
        .context("Failed to create plugin installer")?;

    let install_options = InstallOptions {
        force,
        skip_validation: false,
        verify_signature: true,
        expected_checksum: None,
    };

    let plugin_id = installer
        .install(temp_file.path().to_string_lossy().as_ref(), install_options)
        .await
        .context("Failed to install plugin")?;

    println!(
        "{} {} {} installed successfully!",
        "✓".green(),
        plugin_id,
        target_version
    );

    Ok(())
}

async fn publish_plugin(path: &str, dry_run: bool) -> Result<()> {
    let config = load_config().await?;

    if config.token.is_none() {
        anyhow::bail!("Not logged in. Run 'mockforge plugin registry login' first.");
    }

    let client = RegistryClient::new(config)?;

    // Load and validate plugin manifest from path
    println!("{} Loading plugin manifest from {}...", "📄".blue(), path);
    let manifest_path = Path::new(path).join("plugin.yaml");
    if !manifest_path.exists() {
        anyhow::bail!("Plugin manifest not found at: {}", manifest_path.display());
    }

    let core_manifest = ManifestLoader::load_and_validate_from_file(&manifest_path)
        .context("Failed to load and validate plugin manifest")?;

    println!("{} Manifest loaded and validated", "✓".green());

    // Build plugin if needed
    let target_dir = Path::new(path).join("target").join("wasm32-wasi").join("release");
    let wasm_file = target_dir.join(format!("{}.wasm", core_manifest.id()));

    if !wasm_file.exists() {
        println!("{} Building plugin WASM module...", "🔨".blue());
        build_plugin_wasm(path).await?;
        println!("{} Plugin built successfully", "✓".green());
    } else {
        println!("{} Using existing WASM file: {}", "📦".blue(), wasm_file.display());
    }

    // Calculate checksum
    let wasm_data = fs::read(&wasm_file)
        .context(format!("Failed to read WASM file: {}", wasm_file.display()))?;
    let checksum = calculate_checksum(&wasm_data);
    let size = wasm_data.len() as u64;

    println!("{} Checksum calculated: {}", "🔐".blue(), checksum);

    // Convert to registry manifest
    let registry_manifest = convert_to_registry_manifest(&core_manifest)?;

    // Validate registry manifest
    validate_manifest(&registry_manifest)
        .context("Registry manifest validation failed")?;

    // Create publish request
    let publish_request = PublishRequest {
        name: registry_manifest.name.clone(),
        version: registry_manifest.version.clone(),
        description: registry_manifest.description.clone(),
        author: registry_manifest.author.clone(),
        license: registry_manifest.license.clone(),
        repository: registry_manifest.repository.clone(),
        homepage: registry_manifest.homepage.clone(),
        tags: registry_manifest.tags.clone(),
        category: registry_manifest.category.clone(),
        checksum,
        size,
        min_mockforge_version: registry_manifest.min_mockforge_version.clone(),
    };

    if dry_run {
        println!("{} Dry run - validation passed!", "✓".green());
        println!("  Name: {}", publish_request.name);
        println!("  Version: {}", publish_request.version);
        println!("  Checksum: {}", publish_request.checksum);
        println!("  Size: {} bytes", publish_request.size);
        return Ok(());
    }

    println!("{} Publishing plugin...", "📦".blue());

    // Call client.publish()
    let response = client.publish(publish_request).await
        .context("Failed to publish plugin")?;

    println!("{} Plugin published successfully!", "✓".green());
    println!("  Upload URL: {}", response.upload_url);
    if !response.message.is_empty() {
        println!("  Message: {}", response.message);
    }

    Ok(())
}

async fn yank_version(name: &str, version: &str) -> Result<()> {
    let config = load_config().await?;

    if config.token.is_none() {
        anyhow::bail!("Not logged in. Run 'mockforge plugin registry login' first.");
    }

    let client = RegistryClient::new(config)?;

    println!("{} Yanking {} {}...", "⚠".yellow(), name, version);

    client.yank(name, version).await?;

    println!("{} Version yanked successfully", "✓".green());

    Ok(())
}

async fn login(token: Option<String>) -> Result<()> {
    let token = if let Some(t) = token {
        t
    } else {
        // Prompt for token
        println!("Enter your API token:");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        input.trim().to_string()
    };

    set_token(token).await?;

    println!("{} Successfully logged in", "✓".green());

    Ok(())
}

async fn logout() -> Result<()> {
    clear_token().await?;
    println!("{} Successfully logged out", "✓".green());
    Ok(())
}

async fn show_config() -> Result<()> {
    let config = load_config().await?;

    println!("\n{}", "Registry Configuration:".bold());
    println!("  URL: {}", config.url);
    println!("  Timeout: {}s", config.timeout);
    println!("  Token: {}", if config.token.is_some() { "Set" } else { "Not set" });

    if !config.alternative_registries.is_empty() {
        println!("\n{}", "Alternative Registries:".bold());
        for reg in &config.alternative_registries {
            println!("  • {}", reg);
        }
    }

    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;
    use mockforge_plugin_core::{PluginCapabilities, PluginId, PluginVersion};
    use mockforge_plugin_core::manifest::models::{PluginInfo, PluginManifest};
    use std::collections::HashMap;

    #[test]
    fn test_convert_manifest_without_min_version() {
        let plugin_info = PluginInfo {
            id: PluginId::new("test-plugin").unwrap(),
            name: "Test Plugin".to_string(),
            version: PluginVersion::new("1.0.0").unwrap(),
            description: Some("A test plugin".to_string()),
            author: None,
            types: vec!["auth".to_string()],
            homepage: None,
            repository: None,
            license: Some("MIT".to_string()),
            keywords: vec!["test".to_string()],
        };

        let core_manifest = PluginManifest {
            manifest_version: "1.0".to_string(),
            plugin: plugin_info,
            capabilities: PluginCapabilities::default(),
            dependencies: Vec::new(),
            config_schema: None,
            metadata: HashMap::new(),
        };

        let result = convert_to_registry_manifest(&core_manifest);
        assert!(result.is_ok());

        let registry_manifest = result.unwrap();
        assert_eq!(registry_manifest.name, "test-plugin");
        assert_eq!(registry_manifest.version, "1.0.0");
        assert!(registry_manifest.min_mockforge_version.is_none());
    }

    #[test]
    fn test_convert_manifest_with_min_version() {
        let plugin_info = PluginInfo {
            id: PluginId::new("test-plugin").unwrap(),
            name: "Test Plugin".to_string(),
            version: PluginVersion::new("1.0.0").unwrap(),
            description: Some("A test plugin".to_string()),
            author: None,
            types: vec!["auth".to_string()],
            homepage: None,
            repository: None,
            license: Some("MIT".to_string()),
            keywords: vec!["test".to_string()],
        };

        let mut metadata = HashMap::new();
        let mut runtime = serde_json::Map::new();
        runtime.insert(
            "min_mockforge_version".to_string(),
            serde_json::Value::String("0.1.0".to_string()),
        );
        metadata.insert("runtime".to_string(), serde_json::Value::Object(runtime));

        let core_manifest = PluginManifest {
            manifest_version: "1.0".to_string(),
            plugin: plugin_info,
            capabilities: PluginCapabilities::default(),
            dependencies: Vec::new(),
            config_schema: None,
            metadata,
        };

        let result = convert_to_registry_manifest(&core_manifest);
        assert!(result.is_ok());

        let registry_manifest = result.unwrap();
        assert_eq!(registry_manifest.name, "test-plugin");
        assert_eq!(registry_manifest.version, "1.0.0");
        assert_eq!(
            registry_manifest.min_mockforge_version,
            Some("0.1.0".to_string())
        );
    }

    #[test]
    fn test_convert_manifest_with_invalid_runtime_type() {
        let plugin_info = PluginInfo {
            id: PluginId::new("test-plugin").unwrap(),
            name: "Test Plugin".to_string(),
            version: PluginVersion::new("1.0.0").unwrap(),
            description: Some("A test plugin".to_string()),
            author: None,
            types: vec!["auth".to_string()],
            homepage: None,
            repository: None,
            license: Some("MIT".to_string()),
            keywords: vec!["test".to_string()],
        };

        let mut metadata = HashMap::new();
        // Insert runtime as a string instead of an object
        metadata.insert("runtime".to_string(), serde_json::Value::String("invalid".to_string()));

        let core_manifest = PluginManifest {
            manifest_version: "1.0".to_string(),
            plugin: plugin_info,
            capabilities: PluginCapabilities::default(),
            dependencies: Vec::new(),
            config_schema: None,
            metadata,
        };

        let result = convert_to_registry_manifest(&core_manifest);
        assert!(result.is_ok());

        let registry_manifest = result.unwrap();
        // Should gracefully handle invalid type and return None
        assert!(registry_manifest.min_mockforge_version.is_none());
    }
}
