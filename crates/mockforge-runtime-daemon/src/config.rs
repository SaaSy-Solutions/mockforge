//! Configuration for the runtime daemon

use serde::{Deserialize, Serialize};

/// Configuration for the runtime daemon
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeDaemonConfig {
    /// Whether the runtime daemon is enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Whether to auto-create mocks on 404 responses
    #[serde(default = "default_true")]
    pub auto_create_on_404: bool,

    /// Whether to use AI generation for mock responses
    #[serde(default = "default_false")]
    pub ai_generation: bool,

    /// Whether to generate types (TypeScript/JSON schema)
    #[serde(default = "default_false")]
    pub generate_types: bool,

    /// Whether to generate client stubs
    #[serde(default = "default_false")]
    pub generate_client_stubs: bool,

    /// Whether to update OpenAPI schema automatically
    #[serde(default = "default_false")]
    pub update_openapi: bool,

    /// Whether to create scenarios automatically
    #[serde(default = "default_false")]
    pub create_scenario: bool,

    /// Workspace directory for saving generated mocks
    #[serde(default)]
    pub workspace_dir: Option<String>,

    /// Patterns to exclude from auto-generation (e.g., ["/health", "/metrics"])
    #[serde(default)]
    pub exclude_patterns: Vec<String>,
}

impl RuntimeDaemonConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        let enabled = std::env::var("MOCKFORGE_RUNTIME_DAEMON_ENABLED")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .unwrap_or(false);

        let auto_create_on_404 = std::env::var("MOCKFORGE_RUNTIME_DAEMON_AUTO_CREATE_ON_404")
            .unwrap_or_else(|_| "true".to_string())
            .parse::<bool>()
            .unwrap_or(true);

        let ai_generation = std::env::var("MOCKFORGE_RUNTIME_DAEMON_AI_GENERATION")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .unwrap_or(false);

        let generate_types = std::env::var("MOCKFORGE_RUNTIME_DAEMON_GENERATE_TYPES")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .unwrap_or(false);

        let generate_client_stubs = std::env::var("MOCKFORGE_RUNTIME_DAEMON_GENERATE_CLIENT_STUBS")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .unwrap_or(false);

        let update_openapi = std::env::var("MOCKFORGE_RUNTIME_DAEMON_UPDATE_OPENAPI")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .unwrap_or(false);

        let create_scenario = std::env::var("MOCKFORGE_RUNTIME_DAEMON_CREATE_SCENARIO")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .unwrap_or(false);

        let workspace_dir = std::env::var("MOCKFORGE_RUNTIME_DAEMON_WORKSPACE_DIR").ok();

        // Parse exclude patterns from comma-separated env var
        let exclude_patterns = std::env::var("MOCKFORGE_RUNTIME_DAEMON_EXCLUDE_PATTERNS")
            .unwrap_or_else(|_| "/health,/metrics,/__mockforge".to_string())
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        Self {
            enabled,
            auto_create_on_404,
            ai_generation,
            generate_types,
            generate_client_stubs,
            update_openapi,
            create_scenario,
            workspace_dir,
            exclude_patterns,
        }
    }
}

impl Default for RuntimeDaemonConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            auto_create_on_404: true,
            ai_generation: false,
            generate_types: false,
            generate_client_stubs: false,
            update_openapi: false,
            create_scenario: false,
            workspace_dir: None,
            exclude_patterns: vec![
                "/health".to_string(),
                "/metrics".to_string(),
                "/__mockforge".to_string(),
            ],
        }
    }
}

fn default_enabled() -> bool {
    false
}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_daemon_config_default() {
        let config = RuntimeDaemonConfig::default();
        assert!(!config.enabled);
        assert!(config.auto_create_on_404);
        assert!(!config.ai_generation);
        assert!(!config.generate_types);
        assert!(!config.generate_client_stubs);
        assert!(!config.update_openapi);
        assert!(!config.create_scenario);
        assert!(config.workspace_dir.is_none());
    }

    #[test]
    fn test_runtime_daemon_config_default_exclude_patterns() {
        let config = RuntimeDaemonConfig::default();
        assert_eq!(config.exclude_patterns.len(), 3);
        assert!(config.exclude_patterns.contains(&"/health".to_string()));
        assert!(config.exclude_patterns.contains(&"/metrics".to_string()));
        assert!(config.exclude_patterns.contains(&"/__mockforge".to_string()));
    }

    #[test]
    fn test_runtime_daemon_config_clone() {
        let config = RuntimeDaemonConfig {
            enabled: true,
            auto_create_on_404: false,
            ai_generation: true,
            generate_types: true,
            generate_client_stubs: true,
            update_openapi: true,
            create_scenario: true,
            workspace_dir: Some("/tmp/mocks".to_string()),
            exclude_patterns: vec!["/custom".to_string()],
        };

        let cloned = config.clone();
        assert_eq!(config.enabled, cloned.enabled);
        assert_eq!(config.ai_generation, cloned.ai_generation);
        assert_eq!(config.workspace_dir, cloned.workspace_dir);
        assert_eq!(config.exclude_patterns, cloned.exclude_patterns);
    }

    #[test]
    fn test_runtime_daemon_config_debug() {
        let config = RuntimeDaemonConfig::default();
        let debug = format!("{:?}", config);
        assert!(debug.contains("RuntimeDaemonConfig"));
        assert!(debug.contains("enabled"));
    }

    #[test]
    fn test_runtime_daemon_config_serialize() {
        let config = RuntimeDaemonConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"enabled\":false"));
        assert!(json.contains("\"auto_create_on_404\":true"));
    }

    #[test]
    fn test_runtime_daemon_config_deserialize() {
        let json = r#"{
            "enabled": true,
            "auto_create_on_404": false,
            "ai_generation": true,
            "workspace_dir": "/custom/path"
        }"#;

        let config: RuntimeDaemonConfig = serde_json::from_str(json).unwrap();
        assert!(config.enabled);
        assert!(!config.auto_create_on_404);
        assert!(config.ai_generation);
        assert_eq!(config.workspace_dir, Some("/custom/path".to_string()));
    }

    #[test]
    fn test_runtime_daemon_config_deserialize_defaults() {
        let json = r#"{}"#;
        let config: RuntimeDaemonConfig = serde_json::from_str(json).unwrap();
        // Should use default values
        assert!(!config.enabled);
        assert!(config.auto_create_on_404);
        assert!(!config.ai_generation);
    }

    #[test]
    fn test_runtime_daemon_config_with_custom_exclude() {
        let config = RuntimeDaemonConfig {
            exclude_patterns: vec![
                "/internal".to_string(),
                "/admin".to_string(),
                "secret".to_string(),
            ],
            ..Default::default()
        };

        assert_eq!(config.exclude_patterns.len(), 3);
        assert!(config.exclude_patterns.contains(&"/internal".to_string()));
    }

    #[test]
    fn test_runtime_daemon_config_all_features_enabled() {
        let config = RuntimeDaemonConfig {
            enabled: true,
            auto_create_on_404: true,
            ai_generation: true,
            generate_types: true,
            generate_client_stubs: true,
            update_openapi: true,
            create_scenario: true,
            workspace_dir: Some("./workspace".to_string()),
            exclude_patterns: vec![],
        };

        assert!(config.enabled);
        assert!(config.ai_generation);
        assert!(config.generate_types);
        assert!(config.generate_client_stubs);
        assert!(config.update_openapi);
        assert!(config.create_scenario);
    }

    #[test]
    fn test_default_enabled_function() {
        assert!(!default_enabled());
    }

    #[test]
    fn test_default_true_function() {
        assert!(default_true());
    }

    #[test]
    fn test_default_false_function() {
        assert!(!default_false());
    }
}
