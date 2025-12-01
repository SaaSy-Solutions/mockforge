//! Configuration for mock generation from OpenAPI specifications
//!
//! This module provides configuration management for the `mockforge generate` command,
//! allowing users to customize the mock generation process through configuration files.
//!
//! ## Supported Formats
//!
//! - `mockforge.toml` (recommended, type-safe)
//! - `mockforge.json` (JSON format)
//! - `mockforge.yaml` or `mockforge.yml` (YAML format)
//!
//! ## Priority Order
//!
//! 1. CLI arguments (highest precedence)
//! 2. Configuration file
//! 3. Environment variables
//! 4. Default values (lowest precedence)
//!
//! ## Example Configuration
//!
//! ```toml
//! [input]
//! spec = "openapi.json"
//!
//! [output]
//! path = "./generated"
//! filename = "mock-server.rs"
//!
//! [plugins]
//! oas-types = { package = "oas-types" }
//!
//! [options]
//! client = "reqwest"
//! mode = "tags"
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Configuration for mock generation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct GenerateConfig {
    /// Input specification configuration
    pub input: InputConfig,
    /// Output configuration
    pub output: OutputConfig,
    /// Plugins to use during generation
    #[serde(default)]
    pub plugins: HashMap<String, PluginConfig>,
    /// Generation options
    pub options: Option<GenerateOptions>,
}

impl Default for GenerateConfig {
    fn default() -> Self {
        Self {
            input: InputConfig::default(),
            output: OutputConfig::default(),
            plugins: HashMap::new(),
            options: Some(GenerateOptions::default()),
        }
    }
}

/// Input specification configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[derive(Default)]
pub struct InputConfig {
    /// Path to OpenAPI specification file (JSON or YAML)
    pub spec: Option<PathBuf>,
    /// Additional input files
    #[serde(default)]
    pub additional: Vec<PathBuf>,
}

/// Barrel file type for organizing exports
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum BarrelType {
    /// No barrel files generated
    None,
    /// Generate index.ts (TypeScript/JavaScript)
    #[serde(rename = "index")]
    Index,
    /// Generate index.ts and similar barrel files (full barrel pattern)
    #[serde(rename = "barrel")]
    Barrel,
}

impl Default for BarrelType {
    fn default() -> Self {
        Self::None
    }
}

/// Output configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct OutputConfig {
    /// Output directory path
    pub path: PathBuf,
    /// Output file name (without extension)
    pub filename: Option<String>,
    /// Clean output directory before generation
    #[serde(default)]
    pub clean: bool,
    /// Type of barrel/index files to generate
    #[serde(default)]
    pub barrel_type: BarrelType,
    /// File extension override (e.g., "ts", "tsx", "js", "mjs")
    pub extension: Option<String>,
    /// Banner comment template to prepend to generated files
    /// Supports placeholders: {{timestamp}}, {{source}}, {{generator}}
    pub banner: Option<String>,
    /// File naming template for generated files
    /// Supports placeholders: {{name}}, {{tag}}, {{operation}}, {{path}}
    pub file_naming_template: Option<String>,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            path: PathBuf::from("./generated"),
            filename: None,
            clean: false,
            barrel_type: BarrelType::None,
            extension: None,
            banner: None,
            file_naming_template: None,
        }
    }
}

/// Plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PluginConfig {
    /// Simple plugin (string package name)
    Simple(String),
    /// Advanced plugin configuration
    Advanced {
        /// Package name
        package: String,
        /// Plugin options
        #[serde(default)]
        options: HashMap<String, serde_json::Value>,
    },
}

/// Generation options
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct GenerateOptions {
    /// Client library to generate for (reqwest, ureq, etc.)
    pub client: Option<String>,
    /// Generation mode (operations, tags, paths)
    pub mode: Option<String>,
    /// Include validation in generated code
    pub include_validation: bool,
    /// Include examples in responses
    pub include_examples: bool,
    /// Target runtime (tokio, async-std, sync)
    pub runtime: Option<String>,
}

impl Default for GenerateOptions {
    fn default() -> Self {
        Self {
            client: Some("reqwest".to_string()),
            mode: Some("tags".to_string()),
            include_validation: true,
            include_examples: true,
            runtime: Some("tokio".to_string()),
        }
    }
}

/// Discovery configuration file paths in the current directory
pub fn discover_config_file() -> Result<PathBuf, String> {
    let config_names = vec![
        "mockforge.toml",
        "mockforge.json",
        "mockforge.yaml",
        "mockforge.yml",
        ".mockforge.toml",
        ".mockforge.json",
        ".mockforge.yaml",
        ".mockforge.yml",
    ];

    for name in config_names {
        let path = Path::new(&name);
        if path.exists() {
            return Ok(path.to_path_buf());
        }
    }

    Err("No configuration file found".to_string())
}

/// Load configuration from a file
pub async fn load_generate_config<P: AsRef<Path>>(path: P) -> crate::Result<GenerateConfig> {
    let path = path.as_ref();

    if !path.exists() {
        return Ok(GenerateConfig::default());
    }

    let content = tokio::fs::read_to_string(path)
        .await
        .map_err(|e| crate::Error::generic(format!("Failed to read config file: {}", e)))?;

    let config = if path.extension().and_then(|s| s.to_str()) == Some("toml") {
        toml::from_str(&content)
            .map_err(|e| crate::Error::generic(format!("Failed to parse TOML config: {}", e)))?
    } else if path.extension().and_then(|s| s.to_str()).map(|s| s == "json").unwrap_or(false) {
        serde_json::from_str(&content)
            .map_err(|e| crate::Error::generic(format!("Failed to parse JSON config: {}", e)))?
    } else {
        // Try YAML
        serde_yaml::from_str(&content)
            .map_err(|e| crate::Error::generic(format!("Failed to parse YAML config: {}", e)))?
    };

    Ok(config)
}

/// Load configuration with fallback to defaults
pub async fn load_generate_config_with_fallback<P: AsRef<Path>>(path: P) -> GenerateConfig {
    match load_generate_config(path).await {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Warning: Failed to load config file: {}. Using defaults.", e);
            GenerateConfig::default()
        }
    }
}

/// Save configuration to a file
pub async fn save_generate_config<P: AsRef<Path>>(
    path: P,
    config: &GenerateConfig,
) -> crate::Result<()> {
    let path = path.as_ref();

    let content = if path.extension().and_then(|s| s.to_str()) == Some("toml") {
        toml::to_string_pretty(config)
            .map_err(|e| crate::Error::generic(format!("Failed to serialize to TOML: {}", e)))?
    } else if path.extension().and_then(|s| s.to_str()).map(|s| s == "json").unwrap_or(false) {
        serde_json::to_string_pretty(config)
            .map_err(|e| crate::Error::generic(format!("Failed to serialize to JSON: {}", e)))?
    } else {
        serde_yaml::to_string(config)
            .map_err(|e| crate::Error::generic(format!("Failed to serialize to YAML: {}", e)))?
    };

    tokio::fs::write(path, content)
        .await
        .map_err(|e| crate::Error::generic(format!("Failed to write config file: {}", e)))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = GenerateConfig::default();
        assert!(config.input.spec.is_none());
        assert_eq!(config.output.path, PathBuf::from("./generated"));
        assert!(config.plugins.is_empty());
        assert!(config.options.is_some());
    }

    #[test]
    fn test_config_serialization_toml() {
        let config = GenerateConfig::default();
        let toml = toml::to_string(&config).unwrap();
        assert!(toml.contains("input"));
        assert!(toml.contains("output"));
    }

    #[test]
    fn test_config_serialization_json() {
        let config = GenerateConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("input"));
        assert!(json.contains("output"));
    }

    #[test]
    fn test_config_deserialization_toml() {
        let toml_str = r#"
[input]
spec = "openapi.json"

[output]
path = "./generated"
clean = true
"#;
        let config: GenerateConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.input.spec.unwrap(), PathBuf::from("openapi.json"));
        assert_eq!(config.output.path, PathBuf::from("./generated"));
        assert!(config.output.clean);
    }

    #[test]
    fn test_output_config_with_barrel_type() {
        let toml_str = r#"
[input]

[output]
path = "./generated"
barrel-type = "index"
extension = "ts"
banner = "Generated by {{generator}}"
"#;
        let config: GenerateConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.output.barrel_type, BarrelType::Index);
        assert_eq!(config.output.extension, Some("ts".to_string()));
        assert!(config.output.banner.is_some());
    }

    #[test]
    fn test_config_deserialization_json() {
        let json_str = r#"{
            "input": {
                "spec": "openapi.json"
            },
            "output": {
                "path": "./generated",
                "clean": true
            },
            "options": {
                "client": "reqwest",
                "mode": "tags",
                "include-validation": true,
                "include-examples": true
            }
        }"#;
        let config: GenerateConfig = serde_json::from_str(json_str).unwrap();
        assert_eq!(config.input.spec.unwrap(), PathBuf::from("openapi.json"));
        assert_eq!(config.output.path, PathBuf::from("./generated"));
        assert!(config.output.clean);
        assert!(config.options.is_some());
    }

    #[test]
    fn test_plugin_config_simple() {
        let json_str = r#"{
            "plugin-name": "package-name"
        }"#;
        let plugins: HashMap<String, PluginConfig> = serde_json::from_str(json_str).unwrap();
        match plugins.get("plugin-name").unwrap() {
            PluginConfig::Simple(pkg) => assert_eq!(pkg, "package-name"),
            _ => panic!("Expected simple plugin"),
        }
    }

    #[test]
    fn test_plugin_config_advanced() {
        let json_str = r#"{
            "plugin-name": {
                "package": "package-name",
                "options": {
                    "key": "value"
                }
            }
        }"#;
        let plugins: HashMap<String, PluginConfig> = serde_json::from_str(json_str).unwrap();
        match plugins.get("plugin-name").unwrap() {
            PluginConfig::Advanced { package, options } => {
                assert_eq!(package, "package-name");
                assert!(options.contains_key("key"));
            }
            _ => panic!("Expected advanced plugin"),
        }
    }
}
