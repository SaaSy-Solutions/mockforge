//! Template Library Commands
//!
//! Commands for managing the template library, including:
//! - Registering templates
//! - Searching templates
//! - Installing from marketplace
//! - Managing template versions

use clap::Subcommand;
use mockforge_core::{
    template_library::{TemplateLibraryManager, TemplateMarketplace, TemplateMetadata},
    Error, Result,
};
use std::path::PathBuf;
use tracing::info;

#[derive(Subcommand, Debug, Clone)]
pub enum TemplateCommands {
    /// Register a new template in the library
    Register {
        /// Template ID (unique identifier)
        #[arg(long)]
        id: String,
        /// Template name
        #[arg(long)]
        name: String,
        /// Template description
        #[arg(long)]
        description: Option<String>,
        /// Template version (semver format)
        #[arg(long, default_value = "1.0.0")]
        version: String,
        /// Template author
        #[arg(long)]
        author: Option<String>,
        /// Template tags (comma-separated)
        #[arg(long, value_delimiter = ',')]
        tags: Vec<String>,
        /// Template category
        #[arg(long)]
        category: Option<String>,
        /// Template content (the actual template string)
        #[arg(long)]
        content: String,
        /// Example usage
        #[arg(long)]
        example: Option<String>,
        /// Dependencies (comma-separated template IDs)
        #[arg(long, value_delimiter = ',')]
        dependencies: Vec<String>,
        /// Storage directory for templates
        #[arg(long)]
        storage_dir: Option<PathBuf>,
    },
    /// List templates in the library
    List {
        /// Filter by category
        #[arg(long)]
        category: Option<String>,
        /// Search query
        #[arg(long)]
        search: Option<String>,
        /// Storage directory for templates
        #[arg(long)]
        storage_dir: Option<PathBuf>,
    },
    /// Get a template by ID
    Get {
        /// Template ID
        id: String,
        /// Specific version (defaults to latest)
        #[arg(long)]
        version: Option<String>,
        /// Storage directory for templates
        #[arg(long)]
        storage_dir: Option<PathBuf>,
    },
    /// Remove a template or version
    Remove {
        /// Template ID
        id: String,
        /// Specific version to remove (removes entire template if not specified)
        #[arg(long)]
        version: Option<String>,
        /// Storage directory for templates
        #[arg(long)]
        storage_dir: Option<PathBuf>,
    },
    /// Search templates
    Search {
        /// Search query
        query: String,
        /// Storage directory for templates
        #[arg(long)]
        storage_dir: Option<PathBuf>,
    },
    /// Install a template from marketplace
    Install {
        /// Template ID
        id: String,
        /// Specific version to install (defaults to latest)
        #[arg(long)]
        version: Option<String>,
        /// Marketplace registry URL
        #[arg(long)]
        registry_url: String,
        /// Authentication token
        #[arg(long)]
        auth_token: Option<String>,
        /// Storage directory for templates
        #[arg(long)]
        storage_dir: Option<PathBuf>,
    },
    /// Marketplace operations
    Marketplace {
        /// Marketplace registry URL
        #[arg(long)]
        registry_url: String,
        /// Authentication token
        #[arg(long)]
        auth_token: Option<String>,
        #[command(subcommand)]
        command: MarketplaceCommands,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum MarketplaceCommands {
    /// Search templates in marketplace
    Search {
        /// Search query
        query: String,
    },
    /// List featured templates
    Featured,
    /// List templates by category
    Category {
        /// Category name
        category: String,
    },
    /// Get a template from marketplace
    Get {
        /// Template ID
        id: String,
        /// Specific version (defaults to latest)
        #[arg(long)]
        version: Option<String>,
    },
}

/// Handle template library commands
pub async fn handle_template_command(command: TemplateCommands) -> Result<()> {
    // Extract storage_dir from any command variant
    let storage_dir = match &command {
        TemplateCommands::Register { storage_dir, .. } => storage_dir.clone(),
        TemplateCommands::List { storage_dir, .. } => storage_dir.clone(),
        TemplateCommands::Get { storage_dir, .. } => storage_dir.clone(),
        TemplateCommands::Remove { storage_dir, .. } => storage_dir.clone(),
        TemplateCommands::Search { storage_dir, .. } => storage_dir.clone(),
        TemplateCommands::Install { storage_dir, .. } => storage_dir.clone(),
        TemplateCommands::Marketplace { .. } => None,
    };

    let storage = storage_dir.unwrap_or_else(|| PathBuf::from("./.mockforge-templates"));
    let mut manager = TemplateLibraryManager::new(&storage)?;

    match command {
        TemplateCommands::Register {
            id,
            name,
            description,
            version,
            author,
            tags,
            category,
            content,
            example,
            dependencies,
            ..
        } => {
            handle_register_template(
                &mut manager,
                id,
                name,
                description,
                version,
                author,
                tags,
                category,
                content,
                example,
                dependencies,
            )
            .await
        }
        TemplateCommands::List {
            category, search, ..
        } => handle_list_templates(&manager, category, search).await,
        TemplateCommands::Get { id, version, .. } => {
            handle_get_template(&manager, id, version).await
        }
        TemplateCommands::Remove { id, version, .. } => {
            handle_remove_template(&mut manager, id, version).await
        }
        TemplateCommands::Search { query, .. } => handle_search_templates(&manager, query).await,
        TemplateCommands::Install {
            id,
            version,
            registry_url,
            auth_token,
            ..
        } => handle_install_template(&mut manager, id, version, registry_url, auth_token).await,
        TemplateCommands::Marketplace {
            registry_url,
            auth_token,
            command: marketplace_cmd,
            ..
        } => handle_marketplace_command(registry_url, auth_token, marketplace_cmd).await,
    }
}

async fn handle_register_template(
    manager: &mut TemplateLibraryManager,
    id: String,
    name: String,
    description: Option<String>,
    version: String,
    author: Option<String>,
    tags: Vec<String>,
    category: Option<String>,
    content: String,
    example: Option<String>,
    dependencies: Vec<String>,
) -> Result<()> {
    let metadata = TemplateMetadata {
        id,
        name,
        description,
        version,
        author,
        tags,
        category,
        content,
        example,
        dependencies,
        created_at: None,
        updated_at: None,
    };

    manager.library_mut().register_template(metadata)?;
    info!("Template registered successfully");
    Ok(())
}

async fn handle_list_templates(
    manager: &TemplateLibraryManager,
    category: Option<String>,
    search: Option<String>,
) -> Result<()> {
    let templates = if let Some(ref category) = category {
        manager.library().templates_by_category(category)
    } else if let Some(ref query) = search {
        manager.library().search_templates(query)
    } else {
        manager.library().list_templates()
    };

    if templates.is_empty() {
        println!("No templates found.");
        return Ok(());
    }

    println!("Found {} template(s):\n", templates.len());
    for template in templates {
        println!("ID: {}", template.id);
        println!("Name: {}", template.name);
        if let Some(ref desc) = template.description {
            println!("Description: {}", desc);
        }
        println!("Version: {}", template.latest_version);
        if let Some(ref author) = template.author {
            println!("Author: {}", author);
        }
        if !template.tags.is_empty() {
            println!("Tags: {}", template.tags.join(", "));
        }
        if let Some(ref category) = template.category {
            println!("Category: {}", category);
        }
        println!("Versions: {}", template.versions.len());
        println!();
    }

    Ok(())
}

async fn handle_get_template(
    manager: &TemplateLibraryManager,
    id: String,
    version: Option<String>,
) -> Result<()> {
    let version_clone = version.clone();
    let content = if let Some(ref version) = version {
        manager.library().get_template_version(&id, version)
    } else {
        manager.library().get_latest_template(&id)
    };

    match content {
        Some(content) => {
            println!("Template: {}", id);
            if let Some(ref version) = version_clone {
                println!("Version: {}", version);
            }
            println!("\nContent:\n{}", content);
            Ok(())
        }
        None => Err(Error::generic(format!("Template '{}' not found", id))),
    }
}

async fn handle_remove_template(
    manager: &mut TemplateLibraryManager,
    id: String,
    version: Option<String>,
) -> Result<()> {
    if let Some(version) = version {
        manager.library_mut().remove_template_version(&id, &version)?;
        info!("Removed version {} of template {}", version, id);
    } else {
        manager.library_mut().remove_template(&id)?;
        info!("Removed template {}", id);
    }
    Ok(())
}

async fn handle_search_templates(manager: &TemplateLibraryManager, query: String) -> Result<()> {
    let templates = manager.library().search_templates(&query);

    if templates.is_empty() {
        println!("No templates found matching '{}'", query);
        return Ok(());
    }

    println!("Found {} template(s) matching '{}':\n", templates.len(), query);
    for template in templates {
        println!("- {} ({})", template.name, template.id);
        if let Some(ref desc) = template.description {
            println!("  {}", desc);
        }
    }

    Ok(())
}

async fn handle_install_template(
    manager: &mut TemplateLibraryManager,
    id: String,
    version: Option<String>,
    registry_url: String,
    auth_token: Option<String>,
) -> Result<()> {
    // Take ownership, configure marketplace, then put it back
    let mut temp_manager =
        std::mem::replace(manager, TemplateLibraryManager::new(manager.library().storage_dir())?);
    temp_manager = temp_manager.with_marketplace(registry_url, auth_token);
    temp_manager.install_from_marketplace(&id, version.as_deref()).await?;
    *manager = temp_manager;
    info!("Template '{}' installed successfully", id);
    Ok(())
}

async fn handle_marketplace_command(
    registry_url: String,
    auth_token: Option<String>,
    command: MarketplaceCommands,
) -> Result<()> {
    let marketplace = TemplateMarketplace::new(registry_url, auth_token);

    match command {
        MarketplaceCommands::Search { query } => {
            let templates = marketplace.search(&query).await?;
            println!("Found {} template(s) in marketplace:\n", templates.len());
            for template in templates {
                println!("- {} ({}) - {}", template.name, template.id, template.latest_version);
                if let Some(ref desc) = template.description {
                    println!("  {}", desc);
                }
            }
        }
        MarketplaceCommands::Featured => {
            let templates = marketplace.list_featured().await?;
            println!("Featured templates:\n");
            for template in templates {
                println!("- {} ({}) - {}", template.name, template.id, template.latest_version);
                if let Some(ref desc) = template.description {
                    println!("  {}", desc);
                }
            }
        }
        MarketplaceCommands::Category { category } => {
            let templates = marketplace.list_by_category(&category).await?;
            println!("Templates in category '{}':\n", category);
            for template in templates {
                println!("- {} ({}) - {}", template.name, template.id, template.latest_version);
            }
        }
        MarketplaceCommands::Get { id, version } => {
            let template = marketplace.get_template(&id, version.as_deref()).await?;
            println!("Template: {}", template.name);
            println!("ID: {}", template.id);
            println!("Version: {}", template.latest_version);
            if let Some(ref desc) = template.description {
                println!("Description: {}", desc);
            }
            if let Some(ref author) = template.author {
                println!("Author: {}", author);
            }
            println!(
                "\nContent:\n{}",
                template.versions.first().map(|v| &v.content).unwrap_or(&String::new())
            );
        }
    }

    Ok(())
}
