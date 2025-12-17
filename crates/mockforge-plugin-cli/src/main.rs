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

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    // Clap CLI tests
    #[test]
    fn test_cli_verify_app() {
        // Verify that the CLI command structure is valid
        Cli::command().debug_assert();
    }

    #[test]
    fn test_cli_has_subcommands() {
        let cmd = Cli::command();
        let subcommands: Vec<_> = cmd.get_subcommands().map(|s| s.get_name()).collect();

        assert!(subcommands.contains(&"new"));
        assert!(subcommands.contains(&"build"));
        assert!(subcommands.contains(&"test"));
        assert!(subcommands.contains(&"package"));
        assert!(subcommands.contains(&"validate"));
        assert!(subcommands.contains(&"init"));
        assert!(subcommands.contains(&"info"));
        assert!(subcommands.contains(&"clean"));
    }

    #[test]
    fn test_cli_version() {
        let cmd = Cli::command();
        assert!(cmd.get_version().is_some());
    }

    #[test]
    fn test_cli_name() {
        let cmd = Cli::command();
        assert_eq!(cmd.get_name(), "mockforge-plugin");
    }

    // Commands::New tests
    #[test]
    fn test_new_command_required_args() {
        let result = Cli::try_parse_from(&["mockforge-plugin", "new", "my-plugin", "-t", "auth"]);
        assert!(result.is_ok());

        if let Commands::New {
            name, plugin_type, ..
        } = result.unwrap().command
        {
            assert_eq!(name, "my-plugin");
            assert_eq!(plugin_type, "auth");
        } else {
            panic!("Expected New command");
        }
    }

    #[test]
    fn test_new_command_with_output() {
        let result = Cli::try_parse_from(&[
            "mockforge-plugin",
            "new",
            "test-plugin",
            "--plugin-type",
            "template",
            "--output",
            "/tmp/plugins",
        ]);
        assert!(result.is_ok());

        if let Commands::New {
            name,
            plugin_type,
            output,
            ..
        } = result.unwrap().command
        {
            assert_eq!(name, "test-plugin");
            assert_eq!(plugin_type, "template");
            assert_eq!(output.unwrap().to_str().unwrap(), "/tmp/plugins");
        } else {
            panic!("Expected New command");
        }
    }

    #[test]
    fn test_new_command_with_author() {
        let result = Cli::try_parse_from(&[
            "mockforge-plugin",
            "new",
            "plugin",
            "--plugin-type",
            "auth",
            "--author",
            "John Doe",
            "--email",
            "john@example.com",
        ]);
        assert!(result.is_ok());

        if let Commands::New { author, email, .. } = result.unwrap().command {
            assert_eq!(author.unwrap(), "John Doe");
            assert_eq!(email.unwrap(), "john@example.com");
        } else {
            panic!("Expected New command");
        }
    }

    #[test]
    fn test_new_command_no_git_flag() {
        let result = Cli::try_parse_from(&[
            "mockforge-plugin",
            "new",
            "plugin",
            "--plugin-type",
            "auth",
            "--no-git",
        ]);
        assert!(result.is_ok());

        if let Commands::New { no_git, .. } = result.unwrap().command {
            assert!(no_git);
        } else {
            panic!("Expected New command");
        }
    }

    #[test]
    fn test_new_command_missing_plugin_type() {
        let result = Cli::try_parse_from(&["mockforge-plugin", "new", "plugin"]);
        assert!(result.is_err());
    }

    // Commands::Build tests
    #[test]
    fn test_build_command_default() {
        let result = Cli::try_parse_from(&["mockforge-plugin", "build"]);
        assert!(result.is_ok());

        if let Commands::Build { release, path } = result.unwrap().command {
            assert!(!release);
            assert!(path.is_none());
        } else {
            panic!("Expected Build command");
        }
    }

    #[test]
    fn test_build_command_release() {
        let result = Cli::try_parse_from(&["mockforge-plugin", "build", "--release"]);
        assert!(result.is_ok());

        if let Commands::Build { release, .. } = result.unwrap().command {
            assert!(release);
        } else {
            panic!("Expected Build command");
        }
    }

    #[test]
    fn test_build_command_with_path() {
        let result = Cli::try_parse_from(&["mockforge-plugin", "build", "--path", "/custom/path"]);
        assert!(result.is_ok());

        if let Commands::Build { path, .. } = result.unwrap().command {
            assert_eq!(path.unwrap().to_str().unwrap(), "/custom/path");
        } else {
            panic!("Expected Build command");
        }
    }

    #[test]
    fn test_build_command_short_flags() {
        let result = Cli::try_parse_from(&["mockforge-plugin", "build", "-r", "-p", "/path"]);
        assert!(result.is_ok());

        if let Commands::Build { release, path } = result.unwrap().command {
            assert!(release);
            assert_eq!(path.unwrap().to_str().unwrap(), "/path");
        } else {
            panic!("Expected Build command");
        }
    }

    // Commands::Test tests
    #[test]
    fn test_test_command_default() {
        let result = Cli::try_parse_from(&["mockforge-plugin", "test"]);
        assert!(result.is_ok());

        if let Commands::Test { path, test } = result.unwrap().command {
            assert!(path.is_none());
            assert!(test.is_none());
        } else {
            panic!("Expected Test command");
        }
    }

    #[test]
    fn test_test_command_with_pattern() {
        let result = Cli::try_parse_from(&["mockforge-plugin", "test", "--test", "integration"]);
        assert!(result.is_ok());

        if let Commands::Test { test, .. } = result.unwrap().command {
            assert_eq!(test.unwrap(), "integration");
        } else {
            panic!("Expected Test command");
        }
    }

    #[test]
    fn test_test_command_with_path() {
        let result = Cli::try_parse_from(&["mockforge-plugin", "test", "--path", "/project"]);
        assert!(result.is_ok());

        if let Commands::Test { path, .. } = result.unwrap().command {
            assert_eq!(path.unwrap().to_str().unwrap(), "/project");
        } else {
            panic!("Expected Test command");
        }
    }

    // Commands::Package tests
    #[test]
    fn test_package_command_default() {
        let result = Cli::try_parse_from(&["mockforge-plugin", "package"]);
        assert!(result.is_ok());

        if let Commands::Package { path, output } = result.unwrap().command {
            assert!(path.is_none());
            assert!(output.is_none());
        } else {
            panic!("Expected Package command");
        }
    }

    #[test]
    fn test_package_command_with_output() {
        let result = Cli::try_parse_from(&["mockforge-plugin", "package", "-o", "plugin.zip"]);
        assert!(result.is_ok());

        if let Commands::Package { output, .. } = result.unwrap().command {
            assert_eq!(output.unwrap().to_str().unwrap(), "plugin.zip");
        } else {
            panic!("Expected Package command");
        }
    }

    #[test]
    fn test_package_command_with_path_and_output() {
        let result = Cli::try_parse_from(&[
            "mockforge-plugin",
            "package",
            "--path",
            "/src",
            "--output",
            "/dist/plugin.zip",
        ]);
        assert!(result.is_ok());

        if let Commands::Package { path, output } = result.unwrap().command {
            assert_eq!(path.unwrap().to_str().unwrap(), "/src");
            assert_eq!(output.unwrap().to_str().unwrap(), "/dist/plugin.zip");
        } else {
            panic!("Expected Package command");
        }
    }

    // Commands::Validate tests
    #[test]
    fn test_validate_command_default() {
        let result = Cli::try_parse_from(&["mockforge-plugin", "validate"]);
        assert!(result.is_ok());

        if let Commands::Validate { path } = result.unwrap().command {
            assert!(path.is_none());
        } else {
            panic!("Expected Validate command");
        }
    }

    #[test]
    fn test_validate_command_with_path() {
        let result = Cli::try_parse_from(&["mockforge-plugin", "validate", "-p", "/plugin"]);
        assert!(result.is_ok());

        if let Commands::Validate { path } = result.unwrap().command {
            assert_eq!(path.unwrap().to_str().unwrap(), "/plugin");
        } else {
            panic!("Expected Validate command");
        }
    }

    // Commands::Init tests
    #[test]
    fn test_init_command_required_args() {
        let result = Cli::try_parse_from(&["mockforge-plugin", "init", "--plugin-type", "auth"]);
        assert!(result.is_ok());

        if let Commands::Init {
            plugin_type,
            output,
        } = result.unwrap().command
        {
            assert_eq!(plugin_type, "auth");
            assert!(output.is_none());
        } else {
            panic!("Expected Init command");
        }
    }

    #[test]
    fn test_init_command_with_output() {
        let result = Cli::try_parse_from(&[
            "mockforge-plugin",
            "init",
            "-t",
            "datasource",
            "-o",
            "custom.yaml",
        ]);
        assert!(result.is_ok());

        if let Commands::Init {
            plugin_type,
            output,
        } = result.unwrap().command
        {
            assert_eq!(plugin_type, "datasource");
            assert_eq!(output.unwrap().to_str().unwrap(), "custom.yaml");
        } else {
            panic!("Expected Init command");
        }
    }

    #[test]
    fn test_init_command_missing_plugin_type() {
        let result = Cli::try_parse_from(&["mockforge-plugin", "init"]);
        assert!(result.is_err());
    }

    // Commands::Info tests
    #[test]
    fn test_info_command_default() {
        let result = Cli::try_parse_from(&["mockforge-plugin", "info"]);
        assert!(result.is_ok());

        if let Commands::Info { path } = result.unwrap().command {
            assert!(path.is_none());
        } else {
            panic!("Expected Info command");
        }
    }

    #[test]
    fn test_info_command_with_path() {
        let result = Cli::try_parse_from(&["mockforge-plugin", "info", "--path", "/project"]);
        assert!(result.is_ok());

        if let Commands::Info { path } = result.unwrap().command {
            assert_eq!(path.unwrap().to_str().unwrap(), "/project");
        } else {
            panic!("Expected Info command");
        }
    }

    // Commands::Clean tests
    #[test]
    fn test_clean_command_default() {
        let result = Cli::try_parse_from(&["mockforge-plugin", "clean"]);
        assert!(result.is_ok());

        if let Commands::Clean { path } = result.unwrap().command {
            assert!(path.is_none());
        } else {
            panic!("Expected Clean command");
        }
    }

    #[test]
    fn test_clean_command_with_path() {
        let result = Cli::try_parse_from(&["mockforge-plugin", "clean", "-p", "/plugin"]);
        assert!(result.is_ok());

        if let Commands::Clean { path } = result.unwrap().command {
            assert_eq!(path.unwrap().to_str().unwrap(), "/plugin");
        } else {
            panic!("Expected Clean command");
        }
    }

    // Edge case tests
    #[test]
    fn test_invalid_command() {
        let result = Cli::try_parse_from(&["mockforge-plugin", "invalid"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_help_flag() {
        let result = Cli::try_parse_from(&["mockforge-plugin", "--help"]);
        assert!(result.is_err()); // Help causes early exit
    }

    #[test]
    fn test_version_flag() {
        let result = Cli::try_parse_from(&["mockforge-plugin", "--version"]);
        assert!(result.is_err()); // Version causes early exit
    }

    #[test]
    fn test_subcommand_help() {
        let result = Cli::try_parse_from(&["mockforge-plugin", "new", "--help"]);
        assert!(result.is_err()); // Help causes early exit
    }

    // Commands enum matching tests
    #[test]
    fn test_commands_enum_variants() {
        // Test that Commands enum has all expected variants
        let new_cmd = Commands::New {
            name: "test".to_string(),
            plugin_type: "auth".to_string(),
            output: None,
            author: None,
            email: None,
            no_git: false,
        };

        match new_cmd {
            Commands::New { .. } => {}
            _ => panic!("Expected New variant"),
        }

        let build_cmd = Commands::Build {
            release: false,
            path: None,
        };

        match build_cmd {
            Commands::Build { .. } => {}
            _ => panic!("Expected Build variant"),
        }
    }
}
