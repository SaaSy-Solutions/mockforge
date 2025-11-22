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

