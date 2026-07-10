//! Configuration management for MockForge

mod auth;
mod contracts;
mod operational;
mod protocol;
mod routes;

pub use auth::*;
pub use contracts::*;
pub use operational::*;
pub use protocol::*;
pub use routes::*;

use crate::{Config as CoreConfig, Error, RealityLevel, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;

/// Reality slider configuration for YAML config files
///
/// This is a simplified configuration that stores just the level.
/// The full RealityConfig with all subsystem settings is generated
/// automatically from the level via the RealityEngine.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RealitySliderConfig {
    /// Reality level (1-5)
    pub level: RealityLevel,
    /// Whether to enable reality slider (if false, uses individual subsystem configs)
    pub enabled: bool,
}

impl Default for RealitySliderConfig {
    fn default() -> Self {
        Self {
            level: RealityLevel::ModerateRealism,
            enabled: true,
        }
    }
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct ServerConfig {
    /// HTTP server configuration
    pub http: HttpConfig,
    /// WebSocket server configuration
    pub websocket: WebSocketConfig,
    /// GraphQL server configuration
    pub graphql: GraphQLConfig,
    /// gRPC server configuration
    pub grpc: GrpcConfig,
    /// MQTT server configuration
    pub mqtt: MqttConfig,
    /// SMTP server configuration
    pub smtp: SmtpConfig,
    /// FTP server configuration
    pub ftp: FtpConfig,
    /// Kafka server configuration
    pub kafka: KafkaConfig,
    /// AMQP server configuration
    pub amqp: AmqpConfig,
    /// TCP server configuration
    pub tcp: TcpConfig,
    /// Admin UI configuration
    pub admin: AdminConfig,
    /// Request chaining configuration
    pub chaining: ChainingConfig,
    /// Core MockForge configuration
    pub core: CoreConfig,
    /// Logging configuration
    pub logging: LoggingConfig,
    /// Data generation configuration
    pub data: DataConfig,
    /// MockAI (Behavioral Mock Intelligence) configuration
    #[serde(default)]
    pub mockai: MockAIConfig,
    /// Observability configuration (metrics, tracing)
    pub observability: ObservabilityConfig,
    /// Multi-tenant workspace configuration
    pub multi_tenant: crate::multi_tenant::MultiTenantConfig,
    /// Custom routes configuration
    #[serde(default)]
    pub routes: Vec<RouteConfig>,
    /// Protocol enable/disable configuration
    #[serde(default)]
    pub protocols: ProtocolsConfig,
    /// Named configuration profiles (dev, ci, demo, etc.)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub profiles: HashMap<String, ProfileConfig>,
    /// Deceptive deploy configuration for production-like mock APIs
    #[serde(default)]
    pub deceptive_deploy: DeceptiveDeployConfig,
    /// Behavioral cloning configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub behavioral_cloning: Option<BehavioralCloningConfig>,
    /// Reality slider configuration for unified realism control
    #[serde(default)]
    pub reality: RealitySliderConfig,
    /// Reality Continuum configuration for blending mock and real data sources
    #[serde(default)]
    pub reality_continuum: crate::reality_continuum::ContinuumConfig,
    /// Security monitoring and SIEM configuration
    #[serde(default)]
    pub security: SecurityConfig,
    /// Drift budget and contract monitoring configuration
    #[serde(default)]
    pub drift_budget: crate::contract_drift::DriftBudgetConfig,
    /// Incident management configuration
    #[serde(default)]
    pub incidents: IncidentConfig,
    /// PR generation configuration
    #[serde(default)]
    pub pr_generation: crate::pr_generation::PRGenerationConfig,
    /// Consumer contracts configuration
    #[serde(default)]
    pub consumer_contracts: ConsumerContractsConfig,
    /// Contracts configuration (fitness rules, etc.)
    #[serde(default)]
    pub contracts: ContractsConfig,
    /// Behavioral Economics Engine configuration
    #[serde(default)]
    pub behavioral_economics: BehavioralEconomicsConfig,
    /// Drift Learning configuration
    #[serde(default)]
    pub drift_learning: DriftLearningConfig,
    /// Organization AI controls configuration (YAML defaults, DB overrides)
    #[serde(default)]
    pub org_ai_controls: crate::ai_studio::org_controls::OrgAiControlsConfig,
    /// Performance and resource configuration
    #[serde(default)]
    pub performance: PerformanceConfig,
    /// Plugin resource limits configuration
    #[serde(default)]
    pub plugins: PluginResourceConfig,
    /// Configuration hot-reload settings
    #[serde(default)]
    pub hot_reload: ConfigHotReloadConfig,
    /// Secret backend configuration
    #[serde(default)]
    pub secrets: SecretBackendConfig,
}

/// Profile configuration - a partial ServerConfig that overrides base settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct ProfileConfig {
    /// HTTP server configuration overrides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http: Option<HttpConfig>,
    /// WebSocket server configuration overrides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub websocket: Option<WebSocketConfig>,
    /// GraphQL server configuration overrides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graphql: Option<GraphQLConfig>,
    /// gRPC server configuration overrides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grpc: Option<GrpcConfig>,
    /// MQTT server configuration overrides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mqtt: Option<MqttConfig>,
    /// SMTP server configuration overrides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smtp: Option<SmtpConfig>,
    /// FTP server configuration overrides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ftp: Option<FtpConfig>,
    /// Kafka server configuration overrides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kafka: Option<KafkaConfig>,
    /// AMQP server configuration overrides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amqp: Option<AmqpConfig>,
    /// TCP server configuration overrides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tcp: Option<TcpConfig>,
    /// Admin UI configuration overrides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub admin: Option<AdminConfig>,
    /// Request chaining configuration overrides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chaining: Option<ChainingConfig>,
    /// Core MockForge configuration overrides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub core: Option<CoreConfig>,
    /// Logging configuration overrides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging: Option<LoggingConfig>,
    /// Data generation configuration overrides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<DataConfig>,
    /// MockAI configuration overrides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mockai: Option<MockAIConfig>,
    /// Observability configuration overrides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observability: Option<ObservabilityConfig>,
    /// Multi-tenant workspace configuration overrides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multi_tenant: Option<crate::multi_tenant::MultiTenantConfig>,
    /// Custom routes configuration overrides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routes: Option<Vec<RouteConfig>>,
    /// Protocol enable/disable configuration overrides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocols: Option<ProtocolsConfig>,
    /// Deceptive deploy configuration overrides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deceptive_deploy: Option<DeceptiveDeployConfig>,
    /// Reality slider configuration overrides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reality: Option<RealitySliderConfig>,
    /// Reality Continuum configuration overrides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reality_continuum: Option<crate::reality_continuum::ContinuumConfig>,
    /// Security configuration overrides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub security: Option<SecurityConfig>,
}

impl ServerConfig {
    /// Create a minimal configuration with all defaults.
    pub fn minimal() -> Self {
        Self::default()
    }

    /// Create a development-friendly configuration with admin UI enabled and
    /// debug-level logging.
    pub fn development() -> Self {
        let mut cfg = Self::default();
        cfg.admin.enabled = true;
        cfg.logging.level = "debug".to_string();
        cfg
    }

    /// Create a CI-oriented configuration with latency and failure injection
    /// disabled for deterministic test runs.
    pub fn ci() -> Self {
        let mut cfg = Self::default();
        cfg.core.latency_enabled = false;
        cfg.core.failures_enabled = false;
        cfg
    }

    /// Builder: set the HTTP port.
    #[must_use]
    pub fn with_http_port(mut self, port: u16) -> Self {
        self.http.port = port;
        self
    }

    /// Builder: enable the admin UI on the given port.
    #[must_use]
    pub fn with_admin(mut self, port: u16) -> Self {
        self.admin.enabled = true;
        self.admin.port = port;
        self
    }

    /// Builder: enable gRPC on the given port.
    #[must_use]
    pub fn with_grpc(mut self, port: u16) -> Self {
        self.grpc.enabled = true;
        self.grpc.port = port;
        self.protocols.grpc.enabled = true;
        self
    }

    /// Builder: enable WebSocket on the given port.
    #[must_use]
    pub fn with_websocket(mut self, port: u16) -> Self {
        self.websocket.enabled = true;
        self.websocket.port = port;
        self.protocols.websocket.enabled = true;
        self
    }

    /// Builder: set the log level.
    #[must_use]
    pub fn with_log_level(mut self, level: &str) -> Self {
        self.logging.level = level.to_string();
        self
    }

    /// Check whether any advanced features (MockAI, behavioral cloning,
    /// reality continuum) are enabled.
    pub fn has_advanced_features(&self) -> bool {
        self.mockai.enabled
            || self.behavioral_cloning.as_ref().is_some_and(|bc| bc.enabled)
            || self.reality_continuum.enabled
    }

    /// Check whether any enterprise features (multi-tenant, federation,
    /// security monitoring) are enabled.
    pub fn has_enterprise_features(&self) -> bool {
        self.multi_tenant.enabled || self.security.monitoring.siem.enabled
    }
}

/// Load configuration from file
pub async fn load_config<P: AsRef<Path>>(path: P) -> Result<ServerConfig> {
    let content = fs::read_to_string(&path)
        .await
        .map_err(|e| Error::io_with_context("reading config file", e.to_string()))?;

    // Parse config with improved error messages
    let config: ServerConfig = if path.as_ref().extension().and_then(|s| s.to_str()) == Some("yaml")
        || path.as_ref().extension().and_then(|s| s.to_str()) == Some("yml")
    {
        serde_yaml::from_str(&content).map_err(|e| {
            // Improve error message with field path context
            let error_msg = e.to_string();
            let mut full_msg = format!("Failed to parse YAML config: {}", error_msg);

            // Add helpful context for common errors
            if error_msg.contains("missing field") {
                full_msg.push_str(
                    "\n\n\u{1f4a1} Most configuration fields are optional with defaults.",
                );
                full_msg.push_str(
                    "\n   Omit fields you don't need - MockForge will use sensible defaults.",
                );
                full_msg.push_str("\n   See config.template.yaml for all available options.");
            } else if error_msg.contains("unknown field") {
                full_msg.push_str("\n\n\u{1f4a1} Check for typos in field names.");
                full_msg.push_str("\n   See config.template.yaml for valid field names.");
            }

            Error::config(full_msg)
        })?
    } else {
        serde_json::from_str(&content).map_err(|e| {
            // Improve error message with field path context
            let error_msg = e.to_string();
            let mut full_msg = format!("Failed to parse JSON config: {}", error_msg);

            // Add helpful context for common errors
            if error_msg.contains("missing field") {
                full_msg.push_str(
                    "\n\n\u{1f4a1} Most configuration fields are optional with defaults.",
                );
                full_msg.push_str(
                    "\n   Omit fields you don't need - MockForge will use sensible defaults.",
                );
                full_msg.push_str("\n   See config.template.yaml for all available options.");
            } else if error_msg.contains("unknown field") {
                full_msg.push_str("\n\n\u{1f4a1} Check for typos in field names.");
                full_msg.push_str("\n   See config.template.yaml for valid field names.");
            }

            Error::config(full_msg)
        })?
    };

    let mut config = config;
    resolve_config_relative_paths(path.as_ref(), &mut config);
    // Whether the user literally wrote `http.validation` — the struct default
    // is `Some(enforce)`, so we can't tell "explicitly set" from "defaulted"
    // off the parsed struct alone (#927).
    let validation_explicit = raw_http_key_present(&content, "validation");
    reconcile_unknown_http_keys(&mut config, validation_explicit);

    Ok(config)
}

/// Best-effort check for whether `http.<key>` was literally present in the
/// config source. Parses the raw text generically (YAML or JSON) so it works
/// regardless of how the typed struct defaults the field.
fn raw_http_key_present(content: &str, key: &str) -> bool {
    if let Ok(doc) = serde_yaml::from_str::<serde_yaml::Value>(content) {
        return doc.get("http").and_then(|h| h.get(key)).is_some();
    }
    if let Ok(doc) = serde_json::from_str::<serde_json::Value>(content) {
        return doc.get("http").and_then(|h| h.get(key)).is_some();
    }
    false
}

/// Surface (and where possible honour) `http:` keys MockForge doesn't know.
///
/// Issue #927 — `http.request_validation: "off"` was silently ignored; the real
/// key is `http.validation.mode`. Serde dropped it without a word, so the user
/// had no signal their config was inert. We now capture leftovers in
/// `HttpConfig::unknown_keys` and:
///
///   * honour `request_validation` as an alias for `validation.mode` (only when
///     `validation` wasn't given explicitly, which always wins), and
///   * warn about every other unrecognised key so typos are visible.
fn reconcile_unknown_http_keys(config: &mut ServerConfig, validation_explicit: bool) {
    if config.http.unknown_keys.is_empty() {
        return;
    }

    // Legacy alias: `request_validation: off|warn|enforce`.
    if let Some(value) = config.http.unknown_keys.remove("request_validation") {
        let mode = match &value {
            serde_json::Value::String(s) => Some(s.clone()),
            serde_json::Value::Bool(false) => Some("off".to_string()),
            serde_json::Value::Bool(true) => Some("enforce".to_string()),
            _ => None,
        };
        match mode {
            Some(mode) if !validation_explicit => {
                tracing::warn!(
                    "`http.request_validation` is deprecated; use `http.validation.mode: {}`. \
                     Honouring it for now.",
                    mode
                );
                config.http.validation = Some(HttpValidationConfig { mode });
            }
            Some(_) => {
                tracing::warn!(
                    "`http.request_validation` is deprecated and was ignored because \
                     `http.validation.mode` is also set (that one wins)."
                );
            }
            None => {
                tracing::warn!(
                    "`http.request_validation` must be a string (off|warn|enforce); ignoring."
                );
            }
        }
    }

    for key in config.http.unknown_keys.keys() {
        tracing::warn!(
            "Unknown `http` config key `{}` was ignored. Run `mockforge schema` to see valid keys.",
            key
        );
    }
}

/// Resolve file paths declared in the config that were given as relative paths.
///
/// Issue #928 — `openapi_spec: "abb-rws-openapi.yaml"` was resolved against the
/// process working directory (`/app` in the Docker image), not against the
/// directory holding the config file (`/config`), so a config + spec mounted
/// side by side could not reference each other relatively.
///
/// Backwards compatible on purpose: a path that already resolves from the CWD
/// keeps winning, so existing setups are untouched. Only when the CWD-relative
/// path does NOT exist do we fall back to the config file's directory. If
/// neither exists the value is left alone so the loader's error names the path
/// the user actually wrote.
fn resolve_config_relative_paths(config_path: &Path, config: &mut ServerConfig) {
    let Some(config_dir) = config_path.parent() else {
        return;
    };
    if config_dir.as_os_str().is_empty() {
        return;
    }
    resolve_relative_to(config_dir, &mut config.http.openapi_spec);
}

/// Rewrite `value` to `config_dir/value` when it is a relative path that does
/// not resolve from the CWD but does resolve from the config file's directory.
fn resolve_relative_to(config_dir: &Path, value: &mut Option<String>) {
    let Some(raw) = value.as_deref() else {
        return;
    };
    let candidate = Path::new(raw);
    if candidate.is_absolute() || candidate.exists() {
        return;
    }
    let relocated = config_dir.join(candidate);
    if relocated.exists() {
        tracing::debug!(
            "Resolved relative config path {:?} against config directory {:?}",
            raw,
            config_dir
        );
        *value = Some(relocated.to_string_lossy().into_owned());
    }
}

/// Save configuration to file
pub async fn save_config<P: AsRef<Path>>(path: P, config: &ServerConfig) -> Result<()> {
    let content = if path.as_ref().extension().and_then(|s| s.to_str()) == Some("yaml")
        || path.as_ref().extension().and_then(|s| s.to_str()) == Some("yml")
    {
        serde_yaml::to_string(config)
            .map_err(|e| Error::config(format!("Failed to serialize config to YAML: {}", e)))?
    } else {
        serde_json::to_string_pretty(config)
            .map_err(|e| Error::config(format!("Failed to serialize config to JSON: {}", e)))?
    };

    fs::write(path, content)
        .await
        .map_err(|e| Error::io_with_context("writing config file", e.to_string()))?;

    Ok(())
}

/// Load configuration with fallback to default
pub async fn load_config_with_fallback<P: AsRef<Path>>(path: P) -> ServerConfig {
    match load_config(&path).await {
        Ok(config) => {
            tracing::info!("Loaded configuration from {:?}", path.as_ref());
            config
        }
        Err(e) => {
            tracing::warn!(
                "Failed to load config from {:?}: {}. Using defaults.",
                path.as_ref(),
                e
            );
            ServerConfig::default()
        }
    }
}

/// Create default configuration file
pub async fn create_default_config<P: AsRef<Path>>(path: P) -> Result<()> {
    let config = ServerConfig::default();
    save_config(path, &config).await?;
    Ok(())
}

/// Environment variable overrides for configuration
pub fn apply_env_overrides(mut config: ServerConfig) -> ServerConfig {
    // HTTP server overrides
    if let Ok(port) = std::env::var("MOCKFORGE_HTTP_PORT") {
        if let Ok(port_num) = port.parse() {
            config.http.port = port_num;
        }
    }

    if let Ok(host) = std::env::var("MOCKFORGE_HTTP_HOST") {
        config.http.host = host;
    }

    // WebSocket server overrides
    if let Ok(port) = std::env::var("MOCKFORGE_WS_PORT") {
        if let Ok(port_num) = port.parse() {
            config.websocket.port = port_num;
        }
    }

    if let Ok(host) = std::env::var("MOCKFORGE_WS_HOST") {
        config.websocket.host = host;
    }

    if let Ok(replay) = std::env::var("MOCKFORGE_WS_REPLAY_FILE") {
        config.websocket.replay_file = Some(replay);
    }

    // gRPC server overrides
    if let Ok(port) = std::env::var("MOCKFORGE_GRPC_PORT") {
        if let Ok(port_num) = port.parse() {
            config.grpc.port = port_num;
        }
    }

    if let Ok(enabled) = std::env::var("MOCKFORGE_GRPC_ENABLED") {
        config.grpc.enabled = enabled == "1" || enabled.eq_ignore_ascii_case("true");
    }

    // MQTT broker overrides
    if let Ok(port) = std::env::var("MOCKFORGE_MQTT_PORT") {
        if let Ok(port_num) = port.parse() {
            config.mqtt.port = port_num;
        }
    }

    if let Ok(host) = std::env::var("MOCKFORGE_MQTT_HOST") {
        config.mqtt.host = host;
    }

    if let Ok(enabled) = std::env::var("MOCKFORGE_MQTT_ENABLED") {
        config.mqtt.enabled = enabled == "1" || enabled.eq_ignore_ascii_case("true");
    }

    // Kafka broker overrides
    if let Ok(port) = std::env::var("MOCKFORGE_KAFKA_PORT") {
        if let Ok(port_num) = port.parse() {
            config.kafka.port = port_num;
        }
    }

    if let Ok(host) = std::env::var("MOCKFORGE_KAFKA_HOST") {
        config.kafka.host = host;
    }

    if let Ok(enabled) = std::env::var("MOCKFORGE_KAFKA_ENABLED") {
        config.kafka.enabled = enabled == "1" || enabled.eq_ignore_ascii_case("true");
    }

    if let Ok(dir) = std::env::var("MOCKFORGE_KAFKA_FIXTURES_DIR") {
        config.kafka.fixtures_dir = Some(std::path::PathBuf::from(dir));
    }

    // Kafka advertised-listener overrides. Required when the broker is
    // reachable on a different host/port from clients than where it binds —
    // i.e. every hosted-mock deployment. The orchestrator templates the
    // public `<app>.fly.dev` hostname into MOCKFORGE_KAFKA_ADVERTISED_HOST.
    if let Ok(host) = std::env::var("MOCKFORGE_KAFKA_ADVERTISED_HOST") {
        if !host.trim().is_empty() {
            config.kafka.advertised_host = Some(host);
        }
    }

    if let Ok(port) = std::env::var("MOCKFORGE_KAFKA_ADVERTISED_PORT") {
        if let Ok(port_num) = port.parse() {
            config.kafka.advertised_port = Some(port_num);
        }
    }

    // OpenTelemetry / OTLP overrides. When `MOCKFORGE_OTLP_ENDPOINT` is set,
    // turn on the observability tracing config and route exports to the
    // given URL. Hosted-mock deployments get this set automatically by the
    // orchestrator (#233) so spans flow back to MockForge Cloud.
    if let Ok(endpoint) = std::env::var("MOCKFORGE_OTLP_ENDPOINT") {
        if !endpoint.trim().is_empty() {
            let otel = config
                .observability
                .opentelemetry
                .get_or_insert_with(OpenTelemetryConfig::default);
            otel.enabled = true;
            otel.otlp_endpoint = Some(endpoint);
        }
    }

    if let Ok(rate) = std::env::var("MOCKFORGE_OTLP_SAMPLING_RATE") {
        if let Ok(parsed) = rate.parse::<f64>() {
            if let Some(otel) = config.observability.opentelemetry.as_mut() {
                otel.sampling_rate = parsed.clamp(0.0, 1.0);
            }
        }
    }

    if let Ok(service) = std::env::var("MOCKFORGE_OTLP_SERVICE_NAME") {
        if !service.trim().is_empty() {
            if let Some(otel) = config.observability.opentelemetry.as_mut() {
                otel.service_name = service;
            }
        }
    }

    // AMQP broker overrides
    if let Ok(port) = std::env::var("MOCKFORGE_AMQP_PORT") {
        if let Ok(port_num) = port.parse() {
            config.amqp.port = port_num;
        }
    }

    if let Ok(host) = std::env::var("MOCKFORGE_AMQP_HOST") {
        config.amqp.host = host;
    }

    if let Ok(enabled) = std::env::var("MOCKFORGE_AMQP_ENABLED") {
        config.amqp.enabled = enabled == "1" || enabled.eq_ignore_ascii_case("true");
    }

    // SMTP server overrides
    if let Ok(port) = std::env::var("MOCKFORGE_SMTP_PORT") {
        if let Ok(port_num) = port.parse() {
            config.smtp.port = port_num;
        }
    }

    if let Ok(host) = std::env::var("MOCKFORGE_SMTP_HOST") {
        config.smtp.host = host;
    }

    if let Ok(enabled) = std::env::var("MOCKFORGE_SMTP_ENABLED") {
        config.smtp.enabled = enabled == "1" || enabled.eq_ignore_ascii_case("true");
    }

    if let Ok(hostname) = std::env::var("MOCKFORGE_SMTP_HOSTNAME") {
        config.smtp.hostname = hostname;
    }

    // TCP server overrides
    if let Ok(port) = std::env::var("MOCKFORGE_TCP_PORT") {
        if let Ok(port_num) = port.parse() {
            config.tcp.port = port_num;
        }
    }

    if let Ok(host) = std::env::var("MOCKFORGE_TCP_HOST") {
        config.tcp.host = host;
    }

    if let Ok(enabled) = std::env::var("MOCKFORGE_TCP_ENABLED") {
        config.tcp.enabled = enabled == "1" || enabled.eq_ignore_ascii_case("true");
    }

    // Admin UI overrides
    if let Ok(port) = std::env::var("MOCKFORGE_ADMIN_PORT") {
        if let Ok(port_num) = port.parse() {
            config.admin.port = port_num;
        }
    }

    if std::env::var("MOCKFORGE_ADMIN_ENABLED").unwrap_or_default() == "true" {
        config.admin.enabled = true;
    }

    // Admin UI host override - critical for Docker deployments
    if let Ok(host) = std::env::var("MOCKFORGE_ADMIN_HOST") {
        config.admin.host = host;
    }

    if let Ok(mount_path) = std::env::var("MOCKFORGE_ADMIN_MOUNT_PATH") {
        if !mount_path.trim().is_empty() {
            config.admin.mount_path = Some(mount_path);
        }
    }

    if let Ok(api_enabled) = std::env::var("MOCKFORGE_ADMIN_API_ENABLED") {
        let on = api_enabled == "1" || api_enabled.eq_ignore_ascii_case("true");
        config.admin.api_enabled = on;
    }

    if let Ok(prometheus_url) = std::env::var("PROMETHEUS_URL") {
        config.admin.prometheus_url = prometheus_url;
    }

    // Core configuration overrides
    if let Ok(latency_enabled) = std::env::var("MOCKFORGE_LATENCY_ENABLED") {
        let enabled = latency_enabled == "1" || latency_enabled.eq_ignore_ascii_case("true");
        config.core.latency_enabled = enabled;
    }

    if let Ok(failures_enabled) = std::env::var("MOCKFORGE_FAILURES_ENABLED") {
        let enabled = failures_enabled == "1" || failures_enabled.eq_ignore_ascii_case("true");
        config.core.failures_enabled = enabled;
    }

    if let Ok(overrides_enabled) = std::env::var("MOCKFORGE_OVERRIDES_ENABLED") {
        let enabled = overrides_enabled == "1" || overrides_enabled.eq_ignore_ascii_case("true");
        config.core.overrides_enabled = enabled;
    }

    if let Ok(traffic_shaping_enabled) = std::env::var("MOCKFORGE_TRAFFIC_SHAPING_ENABLED") {
        let enabled =
            traffic_shaping_enabled == "1" || traffic_shaping_enabled.eq_ignore_ascii_case("true");
        config.core.traffic_shaping_enabled = enabled;
    }

    // Traffic shaping overrides
    if let Ok(bandwidth_enabled) = std::env::var("MOCKFORGE_BANDWIDTH_ENABLED") {
        let enabled = bandwidth_enabled == "1" || bandwidth_enabled.eq_ignore_ascii_case("true");
        config.core.traffic_shaping.bandwidth.enabled = enabled;
    }

    if let Ok(max_bytes_per_sec) = std::env::var("MOCKFORGE_BANDWIDTH_MAX_BYTES_PER_SEC") {
        if let Ok(bytes) = max_bytes_per_sec.parse() {
            config.core.traffic_shaping.bandwidth.max_bytes_per_sec = bytes;
            config.core.traffic_shaping.bandwidth.enabled = true;
        }
    }

    if let Ok(burst_capacity) = std::env::var("MOCKFORGE_BANDWIDTH_BURST_CAPACITY_BYTES") {
        if let Ok(bytes) = burst_capacity.parse() {
            config.core.traffic_shaping.bandwidth.burst_capacity_bytes = bytes;
        }
    }

    if let Ok(burst_loss_enabled) = std::env::var("MOCKFORGE_BURST_LOSS_ENABLED") {
        let enabled = burst_loss_enabled == "1" || burst_loss_enabled.eq_ignore_ascii_case("true");
        config.core.traffic_shaping.burst_loss.enabled = enabled;
    }

    if let Ok(burst_probability) = std::env::var("MOCKFORGE_BURST_LOSS_PROBABILITY") {
        if let Ok(prob) = burst_probability.parse::<f64>() {
            config.core.traffic_shaping.burst_loss.burst_probability = prob.clamp(0.0, 1.0);
            config.core.traffic_shaping.burst_loss.enabled = true;
        }
    }

    if let Ok(burst_duration) = std::env::var("MOCKFORGE_BURST_LOSS_DURATION_MS") {
        if let Ok(ms) = burst_duration.parse() {
            config.core.traffic_shaping.burst_loss.burst_duration_ms = ms;
        }
    }

    if let Ok(loss_rate) = std::env::var("MOCKFORGE_BURST_LOSS_RATE") {
        if let Ok(rate) = loss_rate.parse::<f64>() {
            config.core.traffic_shaping.burst_loss.loss_rate_during_burst = rate.clamp(0.0, 1.0);
        }
    }

    if let Ok(recovery_time) = std::env::var("MOCKFORGE_BURST_LOSS_RECOVERY_MS") {
        if let Ok(ms) = recovery_time.parse() {
            config.core.traffic_shaping.burst_loss.recovery_time_ms = ms;
        }
    }

    // Logging overrides
    if let Ok(level) = std::env::var("MOCKFORGE_LOG_LEVEL") {
        config.logging.level = level;
    }

    config
}

/// Validate configuration
pub fn validate_config(config: &ServerConfig) -> Result<()> {
    // Validate port ranges
    if config.http.port == 0 {
        return Err(Error::config("HTTP port cannot be 0"));
    }
    if config.websocket.port == 0 {
        return Err(Error::config("WebSocket port cannot be 0"));
    }
    if config.grpc.port == 0 {
        return Err(Error::config("gRPC port cannot be 0"));
    }
    if config.admin.port == 0 {
        return Err(Error::config("Admin port cannot be 0"));
    }

    // Check for port conflicts
    let ports = [
        ("HTTP", config.http.port),
        ("WebSocket", config.websocket.port),
        ("gRPC", config.grpc.port),
        ("Admin", config.admin.port),
    ];

    for i in 0..ports.len() {
        for j in (i + 1)..ports.len() {
            if ports[i].1 == ports[j].1 {
                return Err(Error::config(format!(
                    "Port conflict: {} and {} both use port {}",
                    ports[i].0, ports[j].0, ports[i].1
                )));
            }
        }
    }

    // Validate log level
    let valid_levels = ["trace", "debug", "info", "warn", "error"];
    if !valid_levels.contains(&config.logging.level.as_str()) {
        return Err(Error::config(format!(
            "Invalid log level: {}. Valid levels: {}",
            config.logging.level,
            valid_levels.join(", ")
        )));
    }

    Ok(())
}

/// Apply a profile to a base configuration
pub fn apply_profile(mut base: ServerConfig, profile: ProfileConfig) -> ServerConfig {
    // Macro to merge optional fields
    macro_rules! merge_field {
        ($field:ident) => {
            if let Some(override_val) = profile.$field {
                base.$field = override_val;
            }
        };
    }

    merge_field!(http);
    merge_field!(websocket);
    merge_field!(graphql);
    merge_field!(grpc);
    merge_field!(mqtt);
    merge_field!(smtp);
    merge_field!(ftp);
    merge_field!(kafka);
    merge_field!(amqp);
    merge_field!(tcp);
    merge_field!(admin);
    merge_field!(chaining);
    merge_field!(core);
    merge_field!(logging);
    merge_field!(data);
    merge_field!(mockai);
    merge_field!(observability);
    merge_field!(multi_tenant);
    merge_field!(routes);
    merge_field!(protocols);

    base
}

/// Load configuration with profile support
pub async fn load_config_with_profile<P: AsRef<Path>>(
    path: P,
    profile_name: Option<&str>,
) -> Result<ServerConfig> {
    // Use load_config_auto to support all formats
    let mut config = load_config_auto(&path).await?;

    // Apply profile if specified
    if let Some(profile) = profile_name {
        if let Some(profile_config) = config.profiles.remove(profile) {
            tracing::info!("Applying profile: {}", profile);
            config = apply_profile(config, profile_config);
        } else {
            return Err(Error::config(format!(
                "Profile '{}' not found in configuration. Available profiles: {}",
                profile,
                config.profiles.keys().map(|k| k.as_str()).collect::<Vec<_>>().join(", ")
            )));
        }
    }

    // Clear profiles from final config to save memory
    config.profiles.clear();

    Ok(config)
}

/// Load configuration from TypeScript/JavaScript file
#[cfg(feature = "scripting")]
pub async fn load_config_from_js<P: AsRef<Path>>(path: P) -> Result<ServerConfig> {
    use rquickjs::{Context, Runtime};

    let content = fs::read_to_string(&path)
        .await
        .map_err(|e| Error::io_with_context("reading JS/TS config file", e.to_string()))?;

    // Create a JavaScript runtime
    let runtime =
        Runtime::new().map_err(|e| Error::config(format!("Failed to create JS runtime: {}", e)))?;
    let context = Context::full(&runtime)
        .map_err(|e| Error::config(format!("Failed to create JS context: {}", e)))?;

    context.with(|ctx| {
        // For TypeScript files, we need to strip type annotations
        // This is a simple approach - for production, consider using a proper TS compiler
        let js_content = if path
            .as_ref()
            .extension()
            .and_then(|s| s.to_str())
            .map(|ext| ext == "ts")
            .unwrap_or(false)
        {
            strip_typescript_types(&content)?
        } else {
            content
        };

        // Evaluate the config file — uses rquickjs sandboxed JS runtime (not arbitrary code execution)
        let result: rquickjs::Value = ctx
            .eval(js_content.as_bytes())
            .map_err(|e| Error::config(format!("Failed to evaluate JS config: {}", e)))?;

        // Convert to JSON string
        let json_str: String = ctx
            .json_stringify(result)
            .map_err(|e| Error::config(format!("Failed to stringify JS config: {}", e)))?
            .ok_or_else(|| Error::config("JS config returned undefined"))?
            .get()
            .map_err(|e| Error::config(format!("Failed to get JSON string: {}", e)))?;

        // Parse JSON into ServerConfig
        serde_json::from_str(&json_str)
            .map_err(|e| Error::config(format!("Failed to parse JS config as ServerConfig: {}", e)))
    })
}

/// Simple TypeScript type stripper (removes type annotations)
/// Note: This is a basic implementation. For production use, consider using swc or esbuild
///
/// # Errors
/// Returns an error if regex compilation fails. This should never happen with static patterns,
/// but we handle it gracefully to prevent panics.
#[cfg(feature = "scripting")]
fn strip_typescript_types(content: &str) -> Result<String> {
    use regex::Regex;

    let mut result = content.to_string();

    // Compile regex patterns with error handling
    // Note: These patterns are statically known and should never fail,
    // but we handle errors to prevent panics in edge cases

    // Remove interface declarations (handles multi-line)
    let interface_re = Regex::new(r"(?ms)interface\s+\w+\s*\{[^}]*\}\s*")
        .map_err(|e| Error::config(format!("Failed to compile interface regex: {}", e)))?;
    result = interface_re.replace_all(&result, "").to_string();

    // Remove type aliases
    let type_alias_re = Regex::new(r"(?m)^type\s+\w+\s*=\s*[^;]+;\s*")
        .map_err(|e| Error::config(format!("Failed to compile type alias regex: {}", e)))?;
    result = type_alias_re.replace_all(&result, "").to_string();

    // Remove type annotations (: Type)
    let type_annotation_re = Regex::new(r":\s*[A-Z]\w*(<[^>]+>)?(\[\])?")
        .map_err(|e| Error::config(format!("Failed to compile type annotation regex: {}", e)))?;
    result = type_annotation_re.replace_all(&result, "").to_string();

    // Remove type imports and exports
    let type_import_re = Regex::new(r"(?m)^(import|export)\s+type\s+.*$")
        .map_err(|e| Error::config(format!("Failed to compile type import regex: {}", e)))?;
    result = type_import_re.replace_all(&result, "").to_string();

    // Remove as Type
    let as_type_re = Regex::new(r"\s+as\s+\w+")
        .map_err(|e| Error::config(format!("Failed to compile 'as type' regex: {}", e)))?;
    result = as_type_re.replace_all(&result, "").to_string();

    Ok(result)
}

/// Enhanced load_config that supports multiple formats including JS/TS
pub async fn load_config_auto<P: AsRef<Path>>(path: P) -> Result<ServerConfig> {
    let ext = path.as_ref().extension().and_then(|s| s.to_str()).unwrap_or("");

    match ext {
        #[cfg(feature = "scripting")]
        "ts" | "js" => load_config_from_js(&path).await,
        #[cfg(not(feature = "scripting"))]
        "ts" | "js" => Err(Error::config(
            "JS/TS config files require the 'scripting' feature (rquickjs). \
             Enable it with: cargo build --features scripting"
                .to_string(),
        )),
        "yaml" | "yml" | "json" => load_config(&path).await,
        _ => Err(Error::config(format!(
            "Unsupported config file format: {}. Supported: .yaml, .yml, .json{}",
            ext,
            if cfg!(feature = "scripting") {
                ", .ts, .js"
            } else {
                ""
            }
        ))),
    }
}

/// Discover configuration file with support for all formats
pub async fn discover_config_file_all_formats() -> Result<std::path::PathBuf> {
    let current_dir = std::env::current_dir()
        .map_err(|e| Error::config(format!("Failed to get current directory: {}", e)))?;

    let config_names = vec![
        "mockforge.config.ts",
        "mockforge.config.js",
        "mockforge.yaml",
        "mockforge.yml",
        ".mockforge.yaml",
        ".mockforge.yml",
    ];

    // Check current directory
    for name in &config_names {
        let path = current_dir.join(name);
        if fs::metadata(&path).await.is_ok() {
            return Ok(path);
        }
    }

    // Check parent directories (up to 5 levels)
    let mut dir = current_dir.clone();
    for _ in 0..5 {
        if let Some(parent) = dir.parent() {
            for name in &config_names {
                let path = parent.join(name);
                if fs::metadata(&path).await.is_ok() {
                    return Ok(path);
                }
            }
            dir = parent.to_path_buf();
        } else {
            break;
        }
    }

    Err(Error::config(
        "No configuration file found. Expected one of: mockforge.config.ts, mockforge.config.js, mockforge.yaml, mockforge.yml",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ServerConfig::default();
        assert_eq!(config.http.port, 3000);
        assert_eq!(config.websocket.port, 3001);
        assert_eq!(config.grpc.port, 50051);
        assert_eq!(config.admin.port, 9080);
    }

    #[test]
    fn test_config_validation() {
        let mut config = ServerConfig::default();
        assert!(validate_config(&config).is_ok());

        // Test port conflict
        config.websocket.port = config.http.port;
        assert!(validate_config(&config).is_err());

        // Test invalid log level
        config.websocket.port = 3001; // Fix port conflict
        config.logging.level = "invalid".to_string();
        assert!(validate_config(&config).is_err());
    }

    #[test]
    fn test_apply_profile() {
        let base = ServerConfig::default();
        assert_eq!(base.http.port, 3000);

        let profile = ProfileConfig {
            http: Some(HttpConfig {
                port: 8080,
                ..Default::default()
            }),
            logging: Some(LoggingConfig {
                level: "debug".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        };

        let merged = apply_profile(base, profile);
        assert_eq!(merged.http.port, 8080);
        assert_eq!(merged.logging.level, "debug");
        assert_eq!(merged.websocket.port, 3001); // Unchanged
    }

    #[test]
    fn test_minimal_config() {
        let config = ServerConfig::minimal();
        assert_eq!(config.http.port, 3000);
        assert!(!config.admin.enabled);
    }

    #[test]
    fn test_development_config() {
        let config = ServerConfig::development();
        assert!(config.admin.enabled);
        assert_eq!(config.logging.level, "debug");
    }

    #[test]
    fn test_ci_config() {
        let config = ServerConfig::ci();
        assert!(!config.core.latency_enabled);
        assert!(!config.core.failures_enabled);
    }

    #[test]
    fn test_builder_with_http_port() {
        let config = ServerConfig::minimal().with_http_port(8080);
        assert_eq!(config.http.port, 8080);
    }

    #[test]
    fn test_builder_with_admin() {
        let config = ServerConfig::minimal().with_admin(9090);
        assert!(config.admin.enabled);
        assert_eq!(config.admin.port, 9090);
    }

    #[test]
    fn test_builder_with_grpc() {
        let config = ServerConfig::minimal().with_grpc(50052);
        assert!(config.grpc.enabled);
        assert_eq!(config.grpc.port, 50052);
        assert!(config.protocols.grpc.enabled);
    }

    #[test]
    fn test_builder_with_websocket() {
        let config = ServerConfig::minimal().with_websocket(3002);
        assert!(config.websocket.enabled);
        assert_eq!(config.websocket.port, 3002);
    }

    #[test]
    fn test_builder_with_log_level() {
        let config = ServerConfig::minimal().with_log_level("trace");
        assert_eq!(config.logging.level, "trace");
    }

    #[test]
    fn test_has_advanced_features_default() {
        let config = ServerConfig::minimal();
        assert!(!config.has_advanced_features());
    }

    #[test]
    fn test_has_enterprise_features_default() {
        let config = ServerConfig::minimal();
        assert!(!config.has_enterprise_features());
    }

    #[test]
    #[cfg(feature = "scripting")]
    fn test_strip_typescript_types() {
        let ts_code = r#"
interface Config {
    port: number;
    host: string;
}

const config: Config = {
    port: 3000,
    host: "localhost"
} as Config;
"#;

        let stripped = strip_typescript_types(ts_code).expect("Should strip TypeScript types");
        assert!(!stripped.contains("interface"));
        assert!(!stripped.contains(": Config"));
        assert!(!stripped.contains("as Config"));
    }

    /// Issue #928 — a relative `openapi_spec` must resolve against the config
    /// file's directory when it does not resolve from the process CWD. This is
    /// the Docker case: config + spec both mounted at `/config`, CWD `/app`.
    #[tokio::test]
    async fn openapi_spec_relative_path_resolves_against_config_dir() {
        let dir = tempfile::tempdir().unwrap();
        let spec_path = dir.path().join("api.yaml");
        std::fs::write(&spec_path, "openapi: 3.0.0\n").unwrap();

        let config_path = dir.path().join("mockforge.yaml");
        std::fs::write(&config_path, "http:\n  openapi_spec: api.yaml\n").unwrap();

        let config = load_config(&config_path).await.expect("config loads");
        let resolved = config.http.openapi_spec.expect("spec path present");
        assert_eq!(
            std::path::Path::new(&resolved).canonicalize().unwrap(),
            spec_path.canonicalize().unwrap(),
            "relative spec must resolve next to the config file, got {resolved}"
        );
    }

    /// An absolute `openapi_spec` is never rewritten.
    #[tokio::test]
    async fn openapi_spec_absolute_path_is_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let spec_path = dir.path().join("api.yaml");
        std::fs::write(&spec_path, "openapi: 3.0.0\n").unwrap();

        let config_path = dir.path().join("mockforge.yaml");
        std::fs::write(&config_path, format!("http:\n  openapi_spec: {}\n", spec_path.display()))
            .unwrap();

        let config = load_config(&config_path).await.expect("config loads");
        assert_eq!(config.http.openapi_spec.as_deref(), Some(spec_path.to_str().unwrap()));
    }

    /// Issue #927 — `http.request_validation` used to be dropped silently.
    /// It is now honoured as an alias for `http.validation.mode`.
    #[tokio::test]
    async fn request_validation_alias_is_honoured() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("mockforge.yaml");
        std::fs::write(&config_path, "http:\n  request_validation: \"off\"\n").unwrap();

        let config = load_config(&config_path).await.expect("config loads");
        assert_eq!(
            config.http.validation.as_ref().map(|v| v.mode.as_str()),
            Some("off"),
            "request_validation must map onto validation.mode"
        );
        // Consumed, so it is not also reported as an unknown key.
        assert!(!config.http.unknown_keys.contains_key("request_validation"));
    }

    /// An explicit `validation.mode` always beats the legacy alias.
    #[tokio::test]
    async fn explicit_validation_mode_beats_alias() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("mockforge.yaml");
        std::fs::write(
            &config_path,
            "http:\n  request_validation: \"off\"\n  validation:\n    mode: \"enforce\"\n",
        )
        .unwrap();

        let config = load_config(&config_path).await.expect("config loads");
        assert_eq!(config.http.validation.as_ref().map(|v| v.mode.as_str()), Some("enforce"));
    }

    /// Issue #927 — a genuinely unknown key is captured so we can warn, rather
    /// than being silently discarded by serde.
    #[tokio::test]
    async fn unknown_http_key_is_captured_for_warning() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("mockforge.yaml");
        std::fs::write(&config_path, "http:\n  porrt: 8080\n").unwrap();

        let config = load_config(&config_path).await.expect("config loads");
        assert!(
            config.http.unknown_keys.contains_key("porrt"),
            "typo'd key must be captured so load_config can warn"
        );
        // And the real port keeps its default.
        assert_eq!(config.http.port, 3000);
    }

    /// A path that resolves from the CWD keeps winning (backwards compatible).
    #[tokio::test]
    async fn openapi_spec_missing_path_is_left_alone_for_a_clear_error() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("mockforge.yaml");
        std::fs::write(&config_path, "http:\n  openapi_spec: nowhere.yaml\n").unwrap();

        let config = load_config(&config_path).await.expect("config loads");
        // Neither CWD nor config-dir resolve it; leave the user's literal value
        // so the loader's error message names what they actually wrote.
        assert_eq!(config.http.openapi_spec.as_deref(), Some("nowhere.yaml"));
    }
}
