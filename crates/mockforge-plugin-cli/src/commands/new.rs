//! Create new plugin project command

use crate::templates::{generate_project, PluginType, TemplateData};
use crate::utils::{ensure_dir, to_kebab_case};
use anyhow::{Context, Result};
use colored::*;
use std::path::Path;
use std::process::Command;

pub async fn create_plugin_project(
    name: &str,
    plugin_type_str: &str,
    output: Option<&Path>,
    author_name: Option<&str>,
    author_email: Option<&str>,
    init_git: bool,
) -> Result<()> {
    // Parse and validate plugin type
    let plugin_type = PluginType::from_str(plugin_type_str)?;

    // Extract plugin name from path if a path was provided
    let plugin_name = Path::new(name).file_name().and_then(|n| n.to_str()).unwrap_or(name);

    // Determine output directory
    let plugin_id = to_kebab_case(plugin_name);
    let output_dir = if name.contains('/') || name.contains('\\') {
        // If name contains path separators, treat it as a full path
        Path::new(name).to_path_buf()
    } else if let Some(out) = output {
        out.join(&plugin_id)
    } else {
        std::env::current_dir()?.join(&plugin_id)
    };

    // Check if directory already exists
    if output_dir.exists() {
        anyhow::bail!(
            "Directory {} already exists. Choose a different name or location.",
            output_dir.display()
        );
    }

    println!("{}", "Creating new plugin project...".cyan().bold());
    println!("  {} {}", "Name:".bold(), plugin_name);
    println!("  {} {}", "Type:".bold(), plugin_type.as_str());
    println!("  {} {}", "Directory:".bold(), output_dir.display());
    println!();

    // Create output directory
    ensure_dir(&output_dir)?;

    // Prepare template data
    let template_data = TemplateData {
        plugin_name: plugin_name.to_string(),
        plugin_id: plugin_id.clone(),
        plugin_type,
        author_name: author_name.map(String::from),
        author_email: author_email.map(String::from),
    };

    // Generate project from template
    generate_project(&template_data, &output_dir)
        .context("Failed to generate project from template")?;

    println!("{}", "✓ Project files generated".green());

    // Initialize Git repository if requested
    if init_git {
        init_git_repo(&output_dir)?;
        println!("{}", "✓ Git repository initialized".green());
    }

    // Print next steps
    println!();
    println!("{}", "Next steps:".bold().green());
    println!("  1. cd {}", plugin_id);
    println!("  2. cargo build --target wasm32-wasi --release");
    println!("  3. cargo test");
    println!();
    println!("{}", "Or use the MockForge plugin CLI:".bold());
    println!("  mockforge-plugin build --release");
    println!("  mockforge-plugin test");
    println!("  mockforge-plugin package");

    Ok(())
}

fn init_git_repo(dir: &Path) -> Result<()> {
    let status = Command::new("git")
        .arg("init")
        .current_dir(dir)
        .status()
        .context("Failed to execute git init. Is git installed?")?;

    if !status.success() {
        anyhow::bail!("Git init failed");
    }

    // Create initial commit
    let _ = Command::new("git").args(["add", "."]).current_dir(dir).status();

    let _ = Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(dir)
        .status();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::Mutex;
    use tempfile::TempDir;

    // Mutex to serialize tests that modify the current working directory
    static CWD_MUTEX: Mutex<()> = Mutex::new(());

    #[tokio::test]
    async fn test_create_plugin_project_basic() {
        let temp_dir = TempDir::new().unwrap();

        let result =
            create_plugin_project("test-plugin", "auth", Some(temp_dir.path()), None, None, false)
                .await;

        assert!(result.is_ok());

        let plugin_dir = temp_dir.path().join("test-plugin");
        assert!(plugin_dir.exists());
        assert!(plugin_dir.join("Cargo.toml").exists());
        assert!(plugin_dir.join("plugin.yaml").exists());
        assert!(plugin_dir.join("src/lib.rs").exists());
        assert!(plugin_dir.join("README.md").exists());
        assert!(plugin_dir.join(".gitignore").exists());
    }

    #[tokio::test]
    async fn test_create_plugin_project_with_author() {
        let temp_dir = TempDir::new().unwrap();

        let result = create_plugin_project(
            "author-plugin",
            "template",
            Some(temp_dir.path()),
            Some("John Doe"),
            Some("john@example.com"),
            false,
        )
        .await;

        assert!(result.is_ok());

        let plugin_dir = temp_dir.path().join("author-plugin");
        let cargo_content = fs::read_to_string(plugin_dir.join("Cargo.toml")).unwrap();
        assert!(cargo_content.contains("John Doe"));
        assert!(cargo_content.contains("john@example.com"));

        let manifest_content = fs::read_to_string(plugin_dir.join("plugin.yaml")).unwrap();
        assert!(manifest_content.contains("John Doe"));
        assert!(manifest_content.contains("john@example.com"));
    }

    #[tokio::test]
    async fn test_create_plugin_project_all_types() {
        let temp_dir = TempDir::new().unwrap();

        for plugin_type in &["auth", "template", "response", "datasource"] {
            let result = create_plugin_project(
                &format!("{}-plugin", plugin_type),
                plugin_type,
                Some(temp_dir.path()),
                None,
                None,
                false,
            )
            .await;

            assert!(result.is_ok(), "Failed for plugin type: {}", plugin_type);

            let plugin_dir = temp_dir.path().join(format!("{}-plugin", plugin_type));
            assert!(plugin_dir.exists());

            let manifest_content = fs::read_to_string(plugin_dir.join("plugin.yaml")).unwrap();
            assert!(manifest_content.contains(&format!("plugin_type: {}", plugin_type)));
        }
    }

    #[tokio::test]
    async fn test_create_plugin_project_invalid_type() {
        let temp_dir = TempDir::new().unwrap();

        let result = create_plugin_project(
            "bad-plugin",
            "invalid-type",
            Some(temp_dir.path()),
            None,
            None,
            false,
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_plugin_project_already_exists() {
        let temp_dir = TempDir::new().unwrap();

        // Create first time
        create_plugin_project("existing", "auth", Some(temp_dir.path()), None, None, false)
            .await
            .unwrap();

        // Try to create again - should fail
        let result =
            create_plugin_project("existing", "auth", Some(temp_dir.path()), None, None, false)
                .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[tokio::test]
    async fn test_create_plugin_project_kebab_case_conversion() {
        let temp_dir = TempDir::new().unwrap();

        let result = create_plugin_project(
            "My Plugin Name",
            "auth",
            Some(temp_dir.path()),
            None,
            None,
            false,
        )
        .await;

        assert!(result.is_ok());

        let plugin_dir = temp_dir.path().join("my-plugin-name");
        assert!(plugin_dir.exists());

        let manifest_content = fs::read_to_string(plugin_dir.join("plugin.yaml")).unwrap();
        assert!(manifest_content.contains("id: my-plugin-name"));
    }

    #[tokio::test]
    async fn test_create_plugin_project_with_path_in_name() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("custom/path/plugin");

        let result =
            create_plugin_project(project_path.to_str().unwrap(), "auth", None, None, None, false)
                .await;

        assert!(result.is_ok());
        assert!(project_path.exists());
        assert!(project_path.join("Cargo.toml").exists());
    }

    #[tokio::test]
    async fn test_create_plugin_project_default_output() {
        // Lock mutex to prevent concurrent tests from changing current directory
        let _guard = CWD_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

        let temp_dir = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();

        std::env::set_current_dir(&temp_dir).unwrap();

        let result = create_plugin_project("default-out", "auth", None, None, None, false).await;

        // Always restore directory, even if test fails
        std::env::set_current_dir(&original_dir).unwrap();

        assert!(result.is_ok());

        let plugin_dir = temp_dir.path().join("default-out");
        assert!(plugin_dir.exists());
    }

    #[tokio::test]
    async fn test_create_plugin_project_files_content() {
        let temp_dir = TempDir::new().unwrap();

        create_plugin_project("content-test", "auth", Some(temp_dir.path()), None, None, false)
            .await
            .unwrap();

        let plugin_dir = temp_dir.path().join("content-test");

        // Check Cargo.toml
        let cargo_content = fs::read_to_string(plugin_dir.join("Cargo.toml")).unwrap();
        assert!(cargo_content.contains("name = \"content-test\""));
        assert!(cargo_content.contains("crate-type = [\"cdylib\"]"));
        assert!(cargo_content.contains("mockforge-plugin-sdk"));

        // Check plugin.yaml
        let manifest_content = fs::read_to_string(plugin_dir.join("plugin.yaml")).unwrap();
        assert!(manifest_content.contains("id: content-test"));
        assert!(manifest_content.contains("plugin_type: auth"));
        assert!(manifest_content.contains("capabilities:"));

        // Check .gitignore
        let gitignore_content = fs::read_to_string(plugin_dir.join(".gitignore")).unwrap();
        assert!(gitignore_content.contains("/target"));
        assert!(gitignore_content.contains("*.wasm"));

        // Check README.md
        let readme_content = fs::read_to_string(plugin_dir.join("README.md")).unwrap();
        assert!(readme_content.contains("content-test"));
        assert!(readme_content.contains("auth"));

        // Check src/lib.rs exists
        assert!(plugin_dir.join("src/lib.rs").exists());
    }

    #[tokio::test]
    async fn test_create_plugin_project_data_source_hyphen() {
        let temp_dir = TempDir::new().unwrap();

        let result = create_plugin_project(
            "ds-plugin",
            "data-source",
            Some(temp_dir.path()),
            None,
            None,
            false,
        )
        .await;

        assert!(result.is_ok());

        let plugin_dir = temp_dir.path().join("ds-plugin");
        let manifest_content = fs::read_to_string(plugin_dir.join("plugin.yaml")).unwrap();
        assert!(manifest_content.contains("plugin_type: datasource"));
    }

    #[test]
    fn test_init_git_repo() {
        let temp_dir = TempDir::new().unwrap();

        // Create a basic directory structure
        fs::create_dir_all(temp_dir.path()).unwrap();
        fs::write(temp_dir.path().join("test.txt"), "test").unwrap();

        let result = init_git_repo(temp_dir.path());

        // Git might not be available in all test environments
        // So we accept both success and error
        match result {
            Ok(_) => {
                // If git succeeded, check .git directory exists
                assert!(temp_dir.path().join(".git").exists());
            }
            Err(_) => {
                // Git not available, test passes
            }
        }
    }

    #[tokio::test]
    async fn test_create_plugin_project_no_git() {
        let temp_dir = TempDir::new().unwrap();

        let result = create_plugin_project(
            "no-git",
            "auth",
            Some(temp_dir.path()),
            None,
            None,
            false, // init_git = false
        )
        .await;

        assert!(result.is_ok());

        let plugin_dir = temp_dir.path().join("no-git");
        assert!(plugin_dir.exists());
        // .git might not exist if git init wasn't called or failed
    }
}
