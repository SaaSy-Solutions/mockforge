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
