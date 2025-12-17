//! Project templates for different plugin types

use anyhow::{Context, Result};
use chrono::Datelike;
use handlebars::Handlebars;
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

mod auth_template;
mod datasource_template;
mod response_template;
mod template_template;

pub use auth_template::AUTH_TEMPLATE;
pub use datasource_template::DATASOURCE_TEMPLATE;
pub use response_template::RESPONSE_TEMPLATE;
pub use template_template::TEMPLATE_TEMPLATE;

/// Plugin type templates
#[derive(Debug, Clone, Copy)]
pub enum PluginType {
    Auth,
    Template,
    Response,
    DataSource,
}

impl PluginType {
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "auth" => Ok(Self::Auth),
            "template" => Ok(Self::Template),
            "response" => Ok(Self::Response),
            "datasource" | "data-source" => Ok(Self::DataSource),
            _ => anyhow::bail!(
                "Unknown plugin type: {}. Valid types: auth, template, response, datasource",
                s
            ),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Auth => "auth",
            Self::Template => "template",
            Self::Response => "response",
            Self::DataSource => "datasource",
        }
    }
}

/// Template data for rendering
pub struct TemplateData {
    pub plugin_name: String,
    pub plugin_id: String,
    pub plugin_type: PluginType,
    pub author_name: Option<String>,
    pub author_email: Option<String>,
}

/// Generate a plugin project from a template
pub fn generate_project(data: &TemplateData, output_dir: &Path) -> Result<()> {
    let mut handlebars = Handlebars::new();
    handlebars.set_strict_mode(true);

    // Get template files based on plugin type
    let template_files = get_template_files(data.plugin_type);

    // Prepare template context
    let context = create_template_context(data);

    // Generate each file
    for (relative_path, template_content) in template_files {
        let file_path = output_dir.join(&relative_path);

        // Ensure parent directory exists
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory {}", parent.display()))?;
        }

        // Render template
        let rendered = handlebars
            .render_template(template_content, &context)
            .with_context(|| format!("Failed to render template for {}", relative_path))?;

        // Write file
        fs::write(&file_path, rendered)
            .with_context(|| format!("Failed to write file {}", file_path.display()))?;
    }

    Ok(())
}

fn create_template_context(data: &TemplateData) -> serde_json::Value {
    let plugin_id_underscore = data.plugin_id.replace('-', "_");
    let plugin_type_pascal = to_pascal_case(data.plugin_type.as_str());

    json!({
        "plugin_name": data.plugin_name,
        "plugin_id": data.plugin_id,
        "plugin_id_underscore": plugin_id_underscore,
        "plugin_type": data.plugin_type.as_str(),
        "plugin_type_pascal": plugin_type_pascal,
        "author_name": data.author_name.as_deref().unwrap_or("Your Name"),
        "author_email": data.author_email.as_deref().unwrap_or("you@example.com"),
        "year": chrono::Utc::now().year(),
    })
}

fn get_template_files(plugin_type: PluginType) -> HashMap<String, &'static str> {
    let mut files = HashMap::new();

    // Common files for all plugin types
    files.insert("Cargo.toml".to_string(), CARGO_TOML_TEMPLATE);
    files.insert("plugin.yaml".to_string(), PLUGIN_MANIFEST_TEMPLATE);
    files.insert(".gitignore".to_string(), GITIGNORE_TEMPLATE);
    files.insert("README.md".to_string(), README_TEMPLATE);

    // Plugin-specific implementation
    let impl_template = match plugin_type {
        PluginType::Auth => AUTH_TEMPLATE,
        PluginType::Template => TEMPLATE_TEMPLATE,
        PluginType::Response => RESPONSE_TEMPLATE,
        PluginType::DataSource => DATASOURCE_TEMPLATE,
    };
    files.insert("src/lib.rs".to_string(), impl_template);

    files
}

fn to_pascal_case(s: &str) -> String {
    s.split('-')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        })
        .collect()
}

// Common templates

const CARGO_TOML_TEMPLATE: &str = r#"[package]
name = "{{plugin_id}}"
version = "0.1.0"
edition = "2021"
authors = ["{{author_name}} <{{author_email}}>"]

[lib]
crate-type = ["cdylib"]

[dependencies]
mockforge-plugin-sdk = "0.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

[dev-dependencies]
tokio = { version = "1", features = ["macros", "rt"] }

[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
strip = true
"#;

const PLUGIN_MANIFEST_TEMPLATE: &str = r#"id: {{plugin_id}}
version: 0.1.0
name: {{plugin_name}}
description: A {{plugin_type}} plugin for MockForge
author:
  name: {{author_name}}
  email: {{author_email}}

plugin_type: {{plugin_type}}

capabilities:
  network: false
  filesystem: false

resource_limits:
  max_memory_bytes: 10485760  # 10MB
  max_cpu_time_ms: 5000       # 5 seconds
"#;

const GITIGNORE_TEMPLATE: &str = r#"/target
/Cargo.lock
**/*.rs.bk
*.wasm
.DS_Store
"#;

const README_TEMPLATE: &str = r#"# {{plugin_name}}

A {{plugin_type}} plugin for MockForge.

## Building

```bash
cargo build --target wasm32-wasi --release
```

Or using the MockForge plugin CLI:

```bash
mockforge-plugin build --release
```

## Testing

```bash
cargo test
```

## Installation

```bash
mockforge plugin install ./target/wasm32-wasi/release/{{plugin_id_underscore}}.wasm
```

## License

MIT OR Apache-2.0
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // PluginType tests
    #[test]
    fn test_plugin_type_from_str_auth() {
        let result = PluginType::from_str("auth");
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), PluginType::Auth));
    }

    #[test]
    fn test_plugin_type_from_str_template() {
        let result = PluginType::from_str("template");
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), PluginType::Template));
    }

    #[test]
    fn test_plugin_type_from_str_response() {
        let result = PluginType::from_str("response");
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), PluginType::Response));
    }

    #[test]
    fn test_plugin_type_from_str_datasource() {
        let result = PluginType::from_str("datasource");
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), PluginType::DataSource));
    }

    #[test]
    fn test_plugin_type_from_str_data_source() {
        let result = PluginType::from_str("data-source");
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), PluginType::DataSource));
    }

    #[test]
    fn test_plugin_type_from_str_case_insensitive() {
        assert!(PluginType::from_str("AUTH").is_ok());
        assert!(PluginType::from_str("Auth").is_ok());
        assert!(PluginType::from_str("TEMPLATE").is_ok());
    }

    #[test]
    fn test_plugin_type_from_str_invalid() {
        let result = PluginType::from_str("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_plugin_type_as_str_auth() {
        assert_eq!(PluginType::Auth.as_str(), "auth");
    }

    #[test]
    fn test_plugin_type_as_str_template() {
        assert_eq!(PluginType::Template.as_str(), "template");
    }

    #[test]
    fn test_plugin_type_as_str_response() {
        assert_eq!(PluginType::Response.as_str(), "response");
    }

    #[test]
    fn test_plugin_type_as_str_datasource() {
        assert_eq!(PluginType::DataSource.as_str(), "datasource");
    }

    #[test]
    fn test_plugin_type_clone() {
        let pt = PluginType::Auth;
        let cloned = pt.clone();
        assert!(matches!(cloned, PluginType::Auth));
    }

    #[test]
    fn test_plugin_type_copy() {
        let pt = PluginType::Template;
        let copied = pt;
        assert!(matches!(copied, PluginType::Template));
        // Original still usable (Copy trait)
        assert!(matches!(pt, PluginType::Template));
    }

    #[test]
    fn test_plugin_type_debug() {
        let pt = PluginType::Response;
        let debug = format!("{:?}", pt);
        assert!(debug.contains("Response"));
    }

    // to_pascal_case tests
    #[test]
    fn test_to_pascal_case_single_word() {
        assert_eq!(to_pascal_case("auth"), "Auth");
    }

    #[test]
    fn test_to_pascal_case_hyphenated() {
        assert_eq!(to_pascal_case("data-source"), "DataSource");
    }

    #[test]
    fn test_to_pascal_case_multiple_hyphens() {
        assert_eq!(to_pascal_case("my-custom-type"), "MyCustomType");
    }

    #[test]
    fn test_to_pascal_case_already_capitalized() {
        assert_eq!(to_pascal_case("Auth"), "Auth");
    }

    #[test]
    fn test_to_pascal_case_empty() {
        assert_eq!(to_pascal_case(""), "");
    }

    // create_template_context tests
    #[test]
    fn test_create_template_context_basic() {
        let data = TemplateData {
            plugin_name: "Test Plugin".to_string(),
            plugin_id: "test-plugin".to_string(),
            plugin_type: PluginType::Auth,
            author_name: Some("John Doe".to_string()),
            author_email: Some("john@example.com".to_string()),
        };

        let context = create_template_context(&data);
        assert_eq!(context["plugin_name"], "Test Plugin");
        assert_eq!(context["plugin_id"], "test-plugin");
        assert_eq!(context["plugin_id_underscore"], "test_plugin");
        assert_eq!(context["plugin_type"], "auth");
        assert_eq!(context["plugin_type_pascal"], "Auth");
        assert_eq!(context["author_name"], "John Doe");
        assert_eq!(context["author_email"], "john@example.com");
    }

    #[test]
    fn test_create_template_context_defaults() {
        let data = TemplateData {
            plugin_name: "My Plugin".to_string(),
            plugin_id: "my-plugin".to_string(),
            plugin_type: PluginType::Template,
            author_name: None,
            author_email: None,
        };

        let context = create_template_context(&data);
        assert_eq!(context["author_name"], "Your Name");
        assert_eq!(context["author_email"], "you@example.com");
    }

    #[test]
    fn test_create_template_context_datasource_type() {
        let data = TemplateData {
            plugin_name: "DB Plugin".to_string(),
            plugin_id: "db-plugin".to_string(),
            plugin_type: PluginType::DataSource,
            author_name: None,
            author_email: None,
        };

        let context = create_template_context(&data);
        assert_eq!(context["plugin_type"], "datasource");
        assert_eq!(context["plugin_type_pascal"], "Datasource");
    }

    // get_template_files tests
    #[test]
    fn test_get_template_files_auth() {
        let files = get_template_files(PluginType::Auth);
        assert!(files.contains_key("Cargo.toml"));
        assert!(files.contains_key("plugin.yaml"));
        assert!(files.contains_key(".gitignore"));
        assert!(files.contains_key("README.md"));
        assert!(files.contains_key("src/lib.rs"));
        assert_eq!(files.len(), 5);
    }

    #[test]
    fn test_get_template_files_template() {
        let files = get_template_files(PluginType::Template);
        assert!(files.contains_key("src/lib.rs"));
        assert_eq!(files.len(), 5);
    }

    #[test]
    fn test_get_template_files_response() {
        let files = get_template_files(PluginType::Response);
        assert!(files.contains_key("src/lib.rs"));
    }

    #[test]
    fn test_get_template_files_datasource() {
        let files = get_template_files(PluginType::DataSource);
        assert!(files.contains_key("src/lib.rs"));
    }

    // generate_project tests
    #[test]
    fn test_generate_project_creates_files() {
        let temp_dir = TempDir::new().unwrap();
        let data = TemplateData {
            plugin_name: "Test Plugin".to_string(),
            plugin_id: "test-plugin".to_string(),
            plugin_type: PluginType::Auth,
            author_name: Some("Test Author".to_string()),
            author_email: Some("test@example.com".to_string()),
        };

        let result = generate_project(&data, temp_dir.path());
        assert!(result.is_ok());

        // Check files were created
        assert!(temp_dir.path().join("Cargo.toml").exists());
        assert!(temp_dir.path().join("plugin.yaml").exists());
        assert!(temp_dir.path().join(".gitignore").exists());
        assert!(temp_dir.path().join("README.md").exists());
        assert!(temp_dir.path().join("src/lib.rs").exists());
    }

    #[test]
    fn test_generate_project_renders_templates() {
        let temp_dir = TempDir::new().unwrap();
        let data = TemplateData {
            plugin_name: "Custom Plugin".to_string(),
            plugin_id: "custom-plugin".to_string(),
            plugin_type: PluginType::Template,
            author_name: Some("Jane Doe".to_string()),
            author_email: Some("jane@test.com".to_string()),
        };

        generate_project(&data, temp_dir.path()).unwrap();

        // Check Cargo.toml content
        let cargo_content = fs::read_to_string(temp_dir.path().join("Cargo.toml")).unwrap();
        assert!(cargo_content.contains("name = \"custom-plugin\""));
        assert!(cargo_content.contains("Jane Doe"));

        // Check README content
        let readme_content = fs::read_to_string(temp_dir.path().join("README.md")).unwrap();
        assert!(readme_content.contains("Custom Plugin"));
    }

    // Template constants tests
    #[test]
    fn test_cargo_toml_template_valid() {
        assert!(CARGO_TOML_TEMPLATE.contains("[package]"));
        assert!(CARGO_TOML_TEMPLATE.contains("{{plugin_id}}"));
        assert!(CARGO_TOML_TEMPLATE.contains("mockforge-plugin-sdk"));
    }

    #[test]
    fn test_plugin_manifest_template_valid() {
        assert!(PLUGIN_MANIFEST_TEMPLATE.contains("id: {{plugin_id}}"));
        assert!(PLUGIN_MANIFEST_TEMPLATE.contains("plugin_type:"));
        assert!(PLUGIN_MANIFEST_TEMPLATE.contains("capabilities:"));
    }

    #[test]
    fn test_gitignore_template_valid() {
        assert!(GITIGNORE_TEMPLATE.contains("/target"));
        assert!(GITIGNORE_TEMPLATE.contains("*.wasm"));
    }

    #[test]
    fn test_readme_template_valid() {
        assert!(README_TEMPLATE.contains("{{plugin_name}}"));
        assert!(README_TEMPLATE.contains("## Building"));
        assert!(README_TEMPLATE.contains("wasm32-wasi"));
    }

    // TemplateData tests
    #[test]
    fn test_template_data_with_author_info() {
        let data = TemplateData {
            plugin_name: "My Plugin".to_string(),
            plugin_id: "my-plugin".to_string(),
            plugin_type: PluginType::Response,
            author_name: Some("Developer".to_string()),
            author_email: Some("dev@company.com".to_string()),
        };

        assert_eq!(data.plugin_name, "My Plugin");
        assert!(data.author_name.is_some());
    }

    #[test]
    fn test_template_data_without_author_info() {
        let data = TemplateData {
            plugin_name: "Anonymous Plugin".to_string(),
            plugin_id: "anon-plugin".to_string(),
            plugin_type: PluginType::DataSource,
            author_name: None,
            author_email: None,
        };

        assert!(data.author_name.is_none());
        assert!(data.author_email.is_none());
    }
}
