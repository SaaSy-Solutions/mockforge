//! MockForge Plugin CLI Tool
//!
//! A command-line tool for creating, building, and managing MockForge plugins.

use clap::{Parser, Subcommand};
use colored::*;
use std::path::PathBuf;

mod commands;
mod templates;
mod utils;

#[derive(Parser)]
#[command(name = "mockforge-plugin")]
#[command(about = "MockForge Plugin Development Tool")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new plugin project
    New {
        /// Plugin name
        name: String,

        /// Plugin type (auth, template, response, datasource)
        #[arg(short, long)]
        plugin_type: String,

        /// Output directory (defaults to current directory)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Plugin author name
        #[arg(long)]
        author: Option<String>,

        /// Plugin author email
        #[arg(long)]
        email: Option<String>,

        /// Skip Git initialization
        #[arg(long)]
        no_git: bool,
    },

    /// Build the plugin WASM module
    Build {
        /// Build in release mode
        #[arg(short, long)]
        release: bool,

        /// Project directory (defaults to current)
        #[arg(short, long)]
        path: Option<PathBuf>,
    },

    /// Run plugin tests
    Test {
        /// Project directory (defaults to current)
        #[arg(short, long)]
        path: Option<PathBuf>,

        /// Test name pattern
        #[arg(long)]
        test: Option<String>,
    },

    /// Package plugin for distribution
    Package {
        /// Project directory (defaults to current)
        #[arg(short, long)]
        path: Option<PathBuf>,

        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Validate plugin manifest and WASM module
    Validate {
        /// Project directory (defaults to current)
        #[arg(short, long)]
        path: Option<PathBuf>,
    },

    /// Initialize plugin manifest template
    Init {
        /// Plugin type (auth, template, response, datasource)
        #[arg(short, long)]
        plugin_type: String,

        /// Output file (defaults to plugin.yaml)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Show plugin information
    Info {
        /// Project directory (defaults to current)
        #[arg(short, long)]
        path: Option<PathBuf>,
    },

    /// Clean build artifacts
    Clean {
        /// Project directory (defaults to current)
        #[arg(short, long)]
        path: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::New {
            name,
            plugin_type,
            output,
            author,
            email,
            no_git,
        } => {
            commands::new::create_plugin_project(
                &name,
                &plugin_type,
                output.as_deref(),
                author.as_deref(),
                email.as_deref(),
                !no_git,
            )
            .await?;
            println!("{}", "✅ Plugin project created successfully!".green().bold());
        }

        Commands::Build { release, path } => {
            commands::build::build_plugin(path.as_deref(), release).await?;
            println!("{}", "✅ Plugin built successfully!".green().bold());
        }

        Commands::Test { path, test } => {
            commands::test::run_tests(path.as_deref(), test.as_deref()).await?;
            println!("{}", "✅ Tests passed!".green().bold());
        }

        Commands::Package { path, output } => {
            let package_path =
                commands::package::package_plugin(path.as_deref(), output.as_deref()).await?;
            println!("{} {}", "✅ Plugin packaged:".green().bold(), package_path.display());
        }

        Commands::Validate { path } => {
            commands::validate::validate_plugin(path.as_deref()).await?;
            println!("{}", "✅ Plugin is valid!".green().bold());
        }

        Commands::Init {
            plugin_type,
            output,
        } => {
            commands::init::init_manifest(&plugin_type, output.as_deref()).await?;
            println!("{}", "✅ Manifest created successfully!".green().bold());
        }

        Commands::Info { path } => {
            commands::info::show_plugin_info(path.as_deref()).await?;
        }

        Commands::Clean { path } => {
            commands::clean::clean_artifacts(path.as_deref()).await?;
            println!("{}", "✅ Build artifacts cleaned!".green().bold());
        }
    }

    Ok(())
}
