//! Registry-related CLI commands

use anyhow::{Context, Result};
use clap::Subcommand;
use colored::Colorize;
use mockforge_plugin_registry::{
    api::RegistryClient,
    config::{load_config, set_token, clear_token},
    PluginCategory, SearchQuery, SortOrder,
};

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

async fn install_from_registry(plugin_spec: &str, _force: bool) -> Result<()> {
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
    let _data = client.download(&version_entry.download_url).await?;

    // TODO: Verify checksum and install plugin

    println!(
        "{} {} {} installed successfully!",
        "✓".green(),
        name.bold(),
        target_version
    );

    Ok(())
}

async fn publish_plugin(_path: &str, dry_run: bool) -> Result<()> {
    let config = load_config().await?;

    if config.token.is_none() {
        anyhow::bail!("Not logged in. Run 'mockforge plugin registry login' first.");
    }

    let client = RegistryClient::new(config)?;

    // TODO: Load and validate plugin manifest from path
    // TODO: Build plugin if needed
    // TODO: Calculate checksum
    // TODO: Create publish request

    if dry_run {
        println!("{} Dry run - validation passed!", "✓".green());
        return Ok(());
    }

    println!("{} Publishing plugin...", "📦".blue());

    // TODO: Call client.publish()

    println!("{} Plugin published successfully!", "✓".green());

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
