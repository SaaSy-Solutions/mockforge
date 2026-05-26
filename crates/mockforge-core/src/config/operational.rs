//! Operational configuration types (performance, secrets, logging, observability, chaos, etc.)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::auth::OAuth2Config;

/// Deceptive deploy configuration for production-like mock APIs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Default)]
pub struct DeceptiveDeployConfig {
    /// Enable deceptive deploy mode
    pub enabled: bool,
    /// Production-like CORS configuration
    pub cors: Option<ProductionCorsConfig>,
    /// Production-like rate limiting
    pub rate_limit: Option<ProductionRateLimitConfig>,
    /// Production-like headers to add to all responses
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// OAuth configuration for production-like auth flows
    pub oauth: Option<ProductionOAuthConfig>,
    /// Custom domain for deployment
    pub custom_domain: Option<String>,
    /// Auto-start tunnel when deploying
    pub auto_tunnel: bool,
    /// Deceptive canary mode configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canary: Option<crate::deceptive_canary::DeceptiveCanaryConfig>,
}

impl DeceptiveDeployConfig {
    /// Generate production-like configuration preset
    pub fn production_preset() -> Self {
        let mut headers = HashMap::new();
        headers.insert("X-API-Version".to_string(), "1.0".to_string());
        headers.insert("X-Request-ID".to_string(), "{{uuid}}".to_string());
        headers.insert("X-Powered-By".to_string(), "MockForge".to_string());

        Self {
            enabled: true,
            cors: Some(ProductionCorsConfig {
                allowed_origins: vec!["*".to_string()],
                allowed_methods: vec![
                    "GET".to_string(),
                    "POST".to_string(),
                    "PUT".to_string(),
                    "DELETE".to_string(),
                    "PATCH".to_string(),
                    "OPTIONS".to_string(),
                ],
                allowed_headers: vec!["*".to_string()],
                allow_credentials: true,
            }),
            rate_limit: Some(ProductionRateLimitConfig {
                requests_per_minute: 1000,
                burst: 2000,
                per_ip: true,
            }),
            headers,
            oauth: None, // Configured separately
            custom_domain: None,
            auto_tunnel: true,
            canary: None,
        }
    }
}

/// Production-like CORS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ProductionCorsConfig {
    /// Allowed origins (use "*" for all origins)
    #[serde(default)]
    pub allowed_origins: Vec<String>,
    /// Allowed HTTP methods
    #[serde(default)]
    pub allowed_methods: Vec<String>,
    /// Allowed headers (use "*" for all headers)
    #[serde(default)]
    pub allowed_headers: Vec<String>,
    /// Allow credentials (cookies, authorization headers)
    pub allow_credentials: bool,
}

/// Production-like rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ProductionRateLimitConfig {
    /// Requests per minute allowed
    pub requests_per_minute: u32,
    /// Burst capacity (maximum requests in a short burst)
    pub burst: u32,
    /// Enable per-IP rate limiting
    pub per_ip: bool,
}

/// Production-like OAuth configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ProductionOAuthConfig {
    /// OAuth2 client ID
    pub client_id: String,
    /// OAuth2 client secret
    pub client_secret: String,
    /// Token introspection URL
    pub introspection_url: String,
    /// Authorization server URL
    pub auth_url: Option<String>,
    /// Token URL
    pub token_url: Option<String>,
    /// Expected token type hint
    pub token_type_hint: Option<String>,
}

impl From<ProductionOAuthConfig> for OAuth2Config {
    /// Convert ProductionOAuthConfig to OAuth2Config for use in auth middleware
    fn from(prod: ProductionOAuthConfig) -> Self {
        OAuth2Config {
            client_id: prod.client_id,
            client_secret: prod.client_secret,
            introspection_url: prod.introspection_url,
            auth_url: prod.auth_url,
            token_url: prod.token_url,
            token_type_hint: prod.token_type_hint,
        }
    }
}

/// Performance and resource configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Default)]
pub struct PerformanceConfig {
    /// Response compression configuration
    pub compression: CompressionConfig,
    /// Connection pooling configuration
    pub connection_pool: ConnectionPoolConfig,
    /// Request limits configuration
    pub request_limits: RequestLimitsConfig,
    /// Worker thread configuration
    pub workers: WorkerConfig,
    /// Circuit breaker configuration
    pub circuit_breaker: CircuitBreakerConfig,
}

/// Response compression configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CompressionConfig {
    /// Enable response compression
    pub enabled: bool,
    /// Compression algorithm: gzip, deflate, br (brotli), zstd
    pub algorithm: String,
    /// Minimum response size to compress (bytes)
    pub min_size: usize,
    /// Compression level (1-9 for gzip/deflate, 0-11 for brotli, 1-22 for zstd)
    pub level: u32,
    /// Content types to compress (e.g., ["application/json", "text/html"])
    pub content_types: Vec<String>,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            algorithm: "gzip".to_string(),
            min_size: 1024, // 1KB
            level: 6,
            content_types: vec![
                "application/json".to_string(),
                "application/xml".to_string(),
                "text/plain".to_string(),
                "text/html".to_string(),
                "text/css".to_string(),
                "application/javascript".to_string(),
            ],
        }
    }
}

/// Connection pooling configuration for downstream services
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ConnectionPoolConfig {
    /// Maximum idle connections per host
    pub max_idle_per_host: usize,
    /// Maximum total connections
    pub max_connections: usize,
    /// Idle connection timeout in seconds
    pub idle_timeout_secs: u64,
    /// Connection acquire timeout in milliseconds
    pub acquire_timeout_ms: u64,
    /// Enable connection pooling
    pub enabled: bool,
}

impl Default for ConnectionPoolConfig {
    fn default() -> Self {
        Self {
            max_idle_per_host: 10,
            max_connections: 100,
            idle_timeout_secs: 90,
            acquire_timeout_ms: 5000,
            enabled: true,
        }
    }
}

/// Request limits configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RequestLimitsConfig {
    /// Maximum request body size in bytes (default: 10MB)
    pub max_body_size: usize,
    /// Maximum header size in bytes
    pub max_header_size: usize,
    /// Maximum number of headers
    pub max_headers: usize,
    /// Maximum URI length
    pub max_uri_length: usize,
    /// Per-route body size limits (path pattern -> max bytes)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub per_route_limits: HashMap<String, usize>,
}

impl Default for RequestLimitsConfig {
    fn default() -> Self {
        Self {
            max_body_size: 10 * 1024 * 1024, // 10MB
            max_header_size: 16 * 1024,      // 16KB
            max_headers: 100,
            max_uri_length: 8192,
            per_route_limits: HashMap::new(),
        }
    }
}

/// Worker thread configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct WorkerConfig {
    /// Number of worker threads (0 = auto-detect based on CPU cores)
    pub threads: usize,
    /// Blocking thread pool size for CPU-intensive work
    pub blocking_threads: usize,
    /// Thread stack size in bytes
    pub stack_size: usize,
    /// Thread name prefix
    pub name_prefix: String,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            threads: 0, // auto-detect
            blocking_threads: 512,
            stack_size: 2 * 1024 * 1024, // 2MB
            name_prefix: "mockforge-worker".to_string(),
        }
    }
}

/// Circuit breaker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CircuitBreakerConfig {
    /// Enable circuit breaker
    pub enabled: bool,
    /// Failure threshold before opening circuit
    pub failure_threshold: u32,
    /// Success threshold before closing circuit
    pub success_threshold: u32,
    /// Half-open timeout in seconds (time before trying again after opening)
    pub half_open_timeout_secs: u64,
    /// Sliding window size for tracking failures
    pub window_size: u32,
    /// Per-endpoint circuit breaker configuration
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub per_endpoint: HashMap<String, EndpointCircuitBreakerConfig>,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            failure_threshold: 5,
            success_threshold: 2,
            half_open_timeout_secs: 30,
            window_size: 10,
            per_endpoint: HashMap::new(),
        }
    }
}

/// Per-endpoint circuit breaker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct EndpointCircuitBreakerConfig {
    /// Failure threshold for this endpoint
    pub failure_threshold: u32,
    /// Success threshold for this endpoint
    pub success_threshold: u32,
    /// Half-open timeout in seconds
    pub half_open_timeout_secs: u64,
}

/// Configuration hot-reload settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ConfigHotReloadConfig {
    /// Enable configuration hot-reload
    pub enabled: bool,
    /// Check interval in seconds
    pub check_interval_secs: u64,
    /// Debounce delay in milliseconds (prevent rapid reloads)
    pub debounce_delay_ms: u64,
    /// Paths to watch for changes (config files, fixture directories)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub watch_paths: Vec<String>,
    /// Reload on OpenAPI spec changes
    pub reload_on_spec_change: bool,
    /// Reload on fixture file changes
    pub reload_on_fixture_change: bool,
    /// Reload on plugin changes
    pub reload_on_plugin_change: bool,
    /// Graceful reload (wait for in-flight requests)
    pub graceful_reload: bool,
    /// Graceful reload timeout in seconds
    pub graceful_timeout_secs: u64,
    /// Validate config before applying reload
    pub validate_before_reload: bool,
    /// Rollback to previous config on reload failure
    pub rollback_on_failure: bool,
}

impl Default for ConfigHotReloadConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            check_interval_secs: 5,
            debounce_delay_ms: 1000,
            watch_paths: Vec::new(),
            reload_on_spec_change: true,
            reload_on_fixture_change: true,
            reload_on_plugin_change: true,
            graceful_reload: true,
            graceful_timeout_secs: 30,
            validate_before_reload: true,
            rollback_on_failure: true,
        }
    }
}

/// Secret backend provider type
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum SecretBackendType {
    /// No secret backend (use environment variables directly)
    #[default]
    None,
    /// HashCorp Vault
    Vault,
    /// AWS Secrets Manager
    AwsSecretsManager,
    /// Azure Key Vault
    AzureKeyVault,
    /// Google Cloud Secret Manager
    GcpSecretManager,
    /// Kubernetes Secrets
    Kubernetes,
    /// Local encrypted file
    EncryptedFile,
}

/// Secret backend configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SecretBackendConfig {
    /// Secret backend provider
    pub provider: SecretBackendType,
    /// Vault-specific configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vault: Option<VaultConfig>,
    /// AWS Secrets Manager configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aws: Option<AwsSecretsConfig>,
    /// Azure Key Vault configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub azure: Option<AzureKeyVaultConfig>,
    /// GCP Secret Manager configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gcp: Option<GcpSecretManagerConfig>,
    /// Kubernetes secrets configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kubernetes: Option<KubernetesSecretsConfig>,
    /// Encrypted file configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encrypted_file: Option<EncryptedFileConfig>,
    /// Secret key mappings (config key -> secret path)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub mappings: HashMap<String, String>,
    /// Cache secrets in memory (seconds, 0 = no caching)
    pub cache_ttl_secs: u64,
    /// Retry configuration for secret retrieval
    pub retry_attempts: u32,
    /// Retry delay in milliseconds
    pub retry_delay_ms: u64,
}

impl Default for SecretBackendConfig {
    fn default() -> Self {
        Self {
            provider: SecretBackendType::None,
            vault: None,
            aws: None,
            azure: None,
            gcp: None,
            kubernetes: None,
            encrypted_file: None,
            mappings: HashMap::new(),
            cache_ttl_secs: 300, // 5 minutes
            retry_attempts: 3,
            retry_delay_ms: 1000,
        }
    }
}

/// HashCorp Vault configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct VaultConfig {
    /// Vault server address
    pub address: String,
    /// Vault namespace (for enterprise)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    /// Authentication method
    pub auth_method: VaultAuthMethod,
    /// Vault token (for token auth)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    /// Role ID (for AppRole auth)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role_id: Option<String>,
    /// Secret ID (for AppRole auth)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret_id: Option<String>,
    /// Kubernetes role (for Kubernetes auth)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kubernetes_role: Option<String>,
    /// Secret engine mount path
    pub mount_path: String,
    /// Secret path prefix
    pub path_prefix: String,
    /// TLS CA certificate path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ca_cert_path: Option<String>,
    /// Skip TLS verification (not recommended for production)
    pub skip_verify: bool,
    /// Request timeout in seconds
    pub timeout_secs: u64,
}

impl Default for VaultConfig {
    fn default() -> Self {
        Self {
            address: "http://127.0.0.1:8200".to_string(),
            namespace: None,
            auth_method: VaultAuthMethod::Token,
            token: None,
            role_id: None,
            secret_id: None,
            kubernetes_role: None,
            mount_path: "secret".to_string(),
            path_prefix: "mockforge".to_string(),
            ca_cert_path: None,
            skip_verify: false,
            timeout_secs: 30,
        }
    }
}

/// Vault authentication methods
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum VaultAuthMethod {
    /// Token authentication
    #[default]
    Token,
    /// AppRole authentication
    AppRole,
    /// Kubernetes authentication
    Kubernetes,
    /// AWS IAM authentication
    AwsIam,
    /// GitHub authentication
    GitHub,
    /// LDAP authentication
    Ldap,
    /// Userpass authentication
    Userpass,
}

/// AWS Secrets Manager configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AwsSecretsConfig {
    /// AWS region
    pub region: String,
    /// Secret name prefix
    pub prefix: String,
    /// Use IAM role (if false, uses access keys)
    pub use_iam_role: bool,
    /// AWS access key ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_key_id: Option<String>,
    /// AWS secret access key
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret_access_key: Option<String>,
    /// Endpoint URL (for LocalStack testing)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint_url: Option<String>,
}

impl Default for AwsSecretsConfig {
    fn default() -> Self {
        Self {
            region: "us-east-1".to_string(),
            prefix: "mockforge".to_string(),
            use_iam_role: true,
            access_key_id: None,
            secret_access_key: None,
            endpoint_url: None,
        }
    }
}

/// Azure Key Vault configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AzureKeyVaultConfig {
    /// Key Vault URL
    pub vault_url: String,
    /// Tenant ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
    /// Client ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    /// Client secret
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_secret: Option<String>,
    /// Use managed identity
    pub use_managed_identity: bool,
    /// Secret name prefix
    pub prefix: String,
}

impl Default for AzureKeyVaultConfig {
    fn default() -> Self {
        Self {
            vault_url: String::new(),
            tenant_id: None,
            client_id: None,
            client_secret: None,
            use_managed_identity: true,
            prefix: "mockforge".to_string(),
        }
    }
}

/// GCP Secret Manager configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct GcpSecretManagerConfig {
    /// GCP project ID
    pub project_id: String,
    /// Secret name prefix
    pub prefix: String,
    /// Service account key file path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credentials_file: Option<String>,
    /// Use default credentials (ADC)
    pub use_default_credentials: bool,
}

impl Default for GcpSecretManagerConfig {
    fn default() -> Self {
        Self {
            project_id: String::new(),
            prefix: "mockforge".to_string(),
            credentials_file: None,
            use_default_credentials: true,
        }
    }
}

/// Kubernetes Secrets configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct KubernetesSecretsConfig {
    /// Namespace to read secrets from
    pub namespace: String,
    /// Secret name prefix
    pub prefix: String,
    /// Label selector
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label_selector: Option<String>,
    /// Use in-cluster config
    pub in_cluster: bool,
    /// Kubeconfig path (if not in-cluster)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kubeconfig_path: Option<String>,
}

impl Default for KubernetesSecretsConfig {
    fn default() -> Self {
        Self {
            namespace: "default".to_string(),
            prefix: "mockforge".to_string(),
            label_selector: None,
            in_cluster: true,
            kubeconfig_path: None,
        }
    }
}

/// Encrypted file configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct EncryptedFileConfig {
    /// Path to encrypted secrets file
    pub file_path: String,
    /// Encryption algorithm
    pub algorithm: String,
    /// Key derivation function
    pub kdf: String,
    /// Master key (from env var)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub master_key_env: Option<String>,
    /// Key file path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_file: Option<String>,
}

impl Default for EncryptedFileConfig {
    fn default() -> Self {
        Self {
            file_path: "secrets.enc".to_string(),
            algorithm: "aes-256-gcm".to_string(),
            kdf: "argon2id".to_string(),
            master_key_env: Some("MOCKFORGE_MASTER_KEY".to_string()),
            key_file: None,
        }
    }
}

/// Plugin runtime resource configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PluginResourceConfig {
    /// Enable plugin system
    pub enabled: bool,
    /// Maximum memory per plugin in bytes (default: 10MB)
    pub max_memory_per_plugin: usize,
    /// Maximum CPU usage per plugin (0.0-1.0, default: 0.5 = 50%)
    pub max_cpu_per_plugin: f64,
    /// Maximum execution time per plugin in milliseconds (default: 5000ms)
    pub max_execution_time_ms: u64,
    /// Allow plugins network access
    pub allow_network_access: bool,
    /// Filesystem paths plugins can access (empty = no fs access)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_fs_paths: Vec<String>,
    /// Maximum concurrent plugin executions
    pub max_concurrent_executions: usize,
    /// Plugin cache directory
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_dir: Option<String>,
    /// Enable debug logging for plugins
    pub debug_logging: bool,
    /// Maximum WASM module size in bytes (default: 5MB)
    pub max_module_size: usize,
    /// Maximum table elements per plugin
    pub max_table_elements: usize,
    /// Maximum WASM stack size in bytes (default: 2MB)
    pub max_stack_size: usize,
}

impl Default for PluginResourceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_memory_per_plugin: 10 * 1024 * 1024, // 10MB
            max_cpu_per_plugin: 0.5,                 // 50% of one core
            max_execution_time_ms: 5000,             // 5 seconds
            allow_network_access: false,
            allowed_fs_paths: Vec::new(),
            max_concurrent_executions: 10,
            cache_dir: None,
            debug_logging: false,
            max_module_size: 5 * 1024 * 1024, // 5MB
            max_table_elements: 1000,
            max_stack_size: 2 * 1024 * 1024, // 2MB
        }
    }
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct LoggingConfig {
    /// Log level
    pub level: String,
    /// Enable JSON logging
    pub json_format: bool,
    /// Log file path (optional)
    pub file_path: Option<String>,
    /// Maximum log file size in MB
    pub max_file_size_mb: u64,
    /// Maximum number of log files to keep
    pub max_files: u32,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            json_format: false,
            file_path: None,
            max_file_size_mb: 10,
            max_files: 5,
        }
    }
}

/// Request chaining configuration for multi-step request workflows
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ChainingConfig {
    /// Enable request chaining
    pub enabled: bool,
    /// Maximum chain length to prevent infinite loops
    pub max_chain_length: usize,
    /// Global timeout for chain execution in seconds
    pub global_timeout_secs: u64,
    /// Enable parallel execution when dependencies allow
    pub enable_parallel_execution: bool,
}

impl Default for ChainingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_chain_length: 20,
            global_timeout_secs: 300,
            enable_parallel_execution: false,
        }
    }
}

/// Data generation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct DataConfig {
    /// Default number of rows to generate
    pub default_rows: usize,
    /// Default output format
    pub default_format: String,
    /// Faker locale
    pub locale: String,
    /// Custom faker templates
    pub templates: HashMap<String, String>,
    /// RAG configuration
    pub rag: RagConfig,
    /// Active persona profile domain (e.g., "finance", "ecommerce", "healthcare")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persona_domain: Option<String>,
    /// Enable persona-based consistency
    #[serde(default = "default_false")]
    pub persona_consistency_enabled: bool,
    /// Persona registry configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persona_registry: Option<PersonaRegistryConfig>,
}

impl Default for DataConfig {
    fn default() -> Self {
        Self {
            default_rows: 100,
            default_format: "json".to_string(),
            locale: "en".to_string(),
            templates: HashMap::new(),
            rag: RagConfig::default(),
            persona_domain: None,
            persona_consistency_enabled: false,
            persona_registry: None,
        }
    }
}

/// RAG configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RagConfig {
    /// Enable RAG by default
    pub enabled: bool,
    /// LLM provider (openai, anthropic, ollama, openai_compatible)
    #[serde(default)]
    pub provider: String,
    /// API endpoint for LLM
    pub api_endpoint: Option<String>,
    /// API key for LLM
    pub api_key: Option<String>,
    /// Model name
    pub model: Option<String>,
    /// Maximum tokens for generation
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
    /// Temperature for generation (0.0 to 2.0)
    #[serde(default = "default_temperature")]
    pub temperature: f64,
    /// Context window size
    pub context_window: usize,
    /// Enable caching
    #[serde(default = "default_true")]
    pub caching: bool,
    /// Cache TTL in seconds
    #[serde(default = "default_cache_ttl")]
    pub cache_ttl_secs: u64,
    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    /// Maximum retries for failed requests
    #[serde(default = "default_max_retries")]
    pub max_retries: usize,
}

fn default_max_tokens() -> usize {
    1024
}

fn default_temperature() -> f64 {
    0.7
}

fn default_true() -> bool {
    true
}

fn default_cache_ttl() -> u64 {
    3600
}

fn default_timeout() -> u64 {
    30
}

fn default_max_retries() -> usize {
    3
}

pub(crate) fn default_false() -> bool {
    false
}

impl Default for RagConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: "openai".to_string(),
            api_endpoint: None,
            api_key: None,
            model: Some("gpt-3.5-turbo".to_string()),
            max_tokens: default_max_tokens(),
            temperature: default_temperature(),
            context_window: 4000,
            caching: default_true(),
            cache_ttl_secs: default_cache_ttl(),
            timeout_secs: default_timeout(),
            max_retries: default_max_retries(),
        }
    }
}

/// Persona registry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Default)]
pub struct PersonaRegistryConfig {
    /// Enable persistence (save personas to disk)
    #[serde(default = "default_false")]
    pub persistent: bool,
    /// Storage path for persistent personas
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_path: Option<String>,
    /// Default traits for new personas
    #[serde(default)]
    pub default_traits: HashMap<String, String>,
}

/// MockAI (Behavioral Mock Intelligence) configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct MockAIConfig {
    /// Enable MockAI features
    pub enabled: bool,
    /// Intelligent behavior configuration
    pub intelligent_behavior: crate::intelligent_behavior::IntelligentBehaviorConfig,
    /// Auto-learn from examples
    pub auto_learn: bool,
    /// Enable mutation detection
    pub mutation_detection: bool,
    /// Enable AI-driven validation errors
    pub ai_validation_errors: bool,
    /// Enable context-aware pagination
    pub intelligent_pagination: bool,
    /// Endpoints to enable MockAI for (empty = all endpoints)
    #[serde(default)]
    pub enabled_endpoints: Vec<String>,
}

impl Default for MockAIConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            intelligent_behavior: crate::intelligent_behavior::IntelligentBehaviorConfig::default(),
            auto_learn: true,
            mutation_detection: true,
            ai_validation_errors: true,
            intelligent_pagination: true,
            enabled_endpoints: Vec::new(),
        }
    }
}

/// Observability configuration for metrics and distributed tracing
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct ObservabilityConfig {
    /// Prometheus metrics configuration
    pub prometheus: PrometheusConfig,
    /// OpenTelemetry distributed tracing configuration
    pub opentelemetry: Option<OpenTelemetryConfig>,
    /// API Flight Recorder configuration
    pub recorder: Option<RecorderConfig>,
    /// Chaos engineering configuration
    pub chaos: Option<ChaosEngConfig>,
}

/// Security monitoring and SIEM configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[derive(Default)]
pub struct SecurityConfig {
    /// Security monitoring configuration
    pub monitoring: SecurityMonitoringConfig,
}

/// Security monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Default)]
pub struct SecurityMonitoringConfig {
    /// SIEM integration configuration
    pub siem: crate::security::siem::SiemConfig,
    /// Access review configuration
    pub access_review: crate::security::access_review::AccessReviewConfig,
    /// Privileged access management configuration
    pub privileged_access: crate::security::privileged_access::PrivilegedAccessConfig,
    /// Change management configuration
    pub change_management: crate::security::change_management::ChangeManagementConfig,
    /// Compliance dashboard configuration
    pub compliance_dashboard: crate::security::compliance_dashboard::ComplianceDashboardConfig,
    /// Risk assessment configuration
    pub risk_assessment: crate::security::risk_assessment::RiskAssessmentConfig,
}

/// Prometheus metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PrometheusConfig {
    /// Enable Prometheus metrics endpoint
    pub enabled: bool,
    /// Port for metrics endpoint
    pub port: u16,
    /// Host for metrics endpoint
    pub host: String,
    /// Path for metrics endpoint
    pub path: String,
}

impl Default for PrometheusConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            port: 9090,
            host: "0.0.0.0".to_string(),
            path: "/metrics".to_string(),
        }
    }
}

/// OpenTelemetry distributed tracing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct OpenTelemetryConfig {
    /// Enable OpenTelemetry tracing
    pub enabled: bool,
    /// Service name for traces
    pub service_name: String,
    /// Deployment environment (development, staging, production)
    pub environment: String,
    /// Jaeger endpoint for trace export
    pub jaeger_endpoint: String,
    /// OTLP endpoint (alternative to Jaeger)
    pub otlp_endpoint: Option<String>,
    /// Protocol: grpc or http
    pub protocol: String,
    /// Sampling rate (0.0 to 1.0)
    pub sampling_rate: f64,
}

impl Default for OpenTelemetryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            service_name: "mockforge".to_string(),
            environment: "development".to_string(),
            jaeger_endpoint: "http://localhost:14268/api/traces".to_string(),
            otlp_endpoint: Some("http://localhost:4317".to_string()),
            protocol: "grpc".to_string(),
            sampling_rate: 1.0,
        }
    }
}

impl OpenTelemetryConfig {
    /// Construct a config from standard OpenTelemetry environment variables.
    ///
    /// Returns `Some(config)` only if at least one of the standard endpoint env
    /// vars is set (`OTEL_EXPORTER_OTLP_ENDPOINT` or
    /// `OTEL_EXPORTER_JAEGER_ENDPOINT`). This is the "zero-config OTel" path:
    /// a user who follows standard OpenTelemetry onboarding (sets the env var,
    /// runs their app) gets spans without having to pass `--tracing` to the
    /// CLI or edit a config file.
    ///
    /// Recognized variables (subset of the OTel spec — only the ones we can
    /// actually act on):
    /// - `OTEL_EXPORTER_OTLP_ENDPOINT` — OTLP collector URL (preferred)
    /// - `OTEL_EXPORTER_JAEGER_ENDPOINT` — Jaeger collector URL (fallback)
    /// - `OTEL_SERVICE_NAME` — overrides the default service name
    /// - `OTEL_TRACES_SAMPLER_ARG` — sampling probability 0.0–1.0
    /// - `OTEL_DEPLOYMENT_ENVIRONMENT` — non-spec convenience for environment
    ///
    /// When neither endpoint var is set, returns `None` so the caller can
    /// stay silent (no log spam, no startup banner).
    pub fn from_env() -> Option<Self> {
        let otlp = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok().filter(|s| !s.is_empty());
        let jaeger = std::env::var("OTEL_EXPORTER_JAEGER_ENDPOINT").ok().filter(|s| !s.is_empty());

        // No endpoint set → user hasn't opted in. Stay quiet.
        if otlp.is_none() && jaeger.is_none() {
            return None;
        }

        let mut cfg = Self {
            enabled: true,
            ..Self::default()
        };

        // OTLP wins when both are set — it's the modern OTel default.
        if let Some(endpoint) = otlp {
            cfg.otlp_endpoint = Some(endpoint);
            cfg.protocol = "grpc".to_string();
        } else if let Some(endpoint) = jaeger {
            cfg.jaeger_endpoint = endpoint;
            cfg.otlp_endpoint = None;
        }

        if let Ok(service_name) = std::env::var("OTEL_SERVICE_NAME") {
            if !service_name.is_empty() {
                cfg.service_name = service_name;
            }
        }
        if let Ok(env) = std::env::var("OTEL_DEPLOYMENT_ENVIRONMENT") {
            if !env.is_empty() {
                cfg.environment = env;
            }
        }
        if let Ok(rate) = std::env::var("OTEL_TRACES_SAMPLER_ARG") {
            if let Ok(parsed) = rate.parse::<f64>() {
                if (0.0..=1.0).contains(&parsed) {
                    cfg.sampling_rate = parsed;
                }
            }
        }

        Some(cfg)
    }
}

#[cfg(test)]
mod opentelemetry_config_tests {
    use super::*;
    use std::sync::Mutex;

    // OTel env vars are process-global. Serialize tests that mutate them so
    // they don't race when run via `cargo test` (which uses a thread pool).
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn clear_otel_env() {
        for var in [
            "OTEL_EXPORTER_OTLP_ENDPOINT",
            "OTEL_EXPORTER_JAEGER_ENDPOINT",
            "OTEL_SERVICE_NAME",
            "OTEL_DEPLOYMENT_ENVIRONMENT",
            "OTEL_TRACES_SAMPLER_ARG",
        ] {
            std::env::remove_var(var);
        }
    }

    #[test]
    fn from_env_returns_none_when_no_endpoint_set() {
        let _g = ENV_LOCK.lock().unwrap();
        clear_otel_env();
        assert!(OpenTelemetryConfig::from_env().is_none());
    }

    #[test]
    fn from_env_picks_up_otlp_endpoint() {
        let _g = ENV_LOCK.lock().unwrap();
        clear_otel_env();
        std::env::set_var("OTEL_EXPORTER_OTLP_ENDPOINT", "http://collector:4317");
        let cfg = OpenTelemetryConfig::from_env().expect("should auto-enable");
        assert!(cfg.enabled);
        assert_eq!(cfg.otlp_endpoint.as_deref(), Some("http://collector:4317"));
        assert_eq!(cfg.protocol, "grpc");
        clear_otel_env();
    }

    #[test]
    fn from_env_picks_up_jaeger_endpoint_when_no_otlp() {
        let _g = ENV_LOCK.lock().unwrap();
        clear_otel_env();
        std::env::set_var("OTEL_EXPORTER_JAEGER_ENDPOINT", "http://jaeger:14268/api/traces");
        let cfg = OpenTelemetryConfig::from_env().expect("should auto-enable");
        assert!(cfg.enabled);
        assert_eq!(cfg.jaeger_endpoint, "http://jaeger:14268/api/traces");
        assert!(cfg.otlp_endpoint.is_none());
        clear_otel_env();
    }

    #[test]
    fn from_env_applies_service_name_and_sampling() {
        let _g = ENV_LOCK.lock().unwrap();
        clear_otel_env();
        std::env::set_var("OTEL_EXPORTER_OTLP_ENDPOINT", "http://collector:4317");
        std::env::set_var("OTEL_SERVICE_NAME", "mockforge-prod");
        std::env::set_var("OTEL_DEPLOYMENT_ENVIRONMENT", "production");
        std::env::set_var("OTEL_TRACES_SAMPLER_ARG", "0.25");
        let cfg = OpenTelemetryConfig::from_env().expect("should auto-enable");
        assert_eq!(cfg.service_name, "mockforge-prod");
        assert_eq!(cfg.environment, "production");
        assert!((cfg.sampling_rate - 0.25).abs() < f64::EPSILON);
        clear_otel_env();
    }

    #[test]
    fn from_env_rejects_invalid_sampling_rate() {
        let _g = ENV_LOCK.lock().unwrap();
        clear_otel_env();
        std::env::set_var("OTEL_EXPORTER_OTLP_ENDPOINT", "http://collector:4317");
        std::env::set_var("OTEL_TRACES_SAMPLER_ARG", "9.0");
        let cfg = OpenTelemetryConfig::from_env().expect("should auto-enable");
        // Invalid value rejected; falls back to default sampling rate.
        assert!((cfg.sampling_rate - 1.0).abs() < f64::EPSILON);
        clear_otel_env();
    }
}

/// API Flight Recorder configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RecorderConfig {
    /// Enable recording
    pub enabled: bool,
    /// Database file path
    pub database_path: String,
    /// Enable management API
    pub api_enabled: bool,
    /// Management API port (if different from main port)
    pub api_port: Option<u16>,
    /// Maximum number of requests to store (0 for unlimited)
    pub max_requests: i64,
    /// Auto-delete requests older than N days (0 to disable)
    pub retention_days: i64,
    /// Record HTTP requests
    pub record_http: bool,
    /// Record gRPC requests
    pub record_grpc: bool,
    /// Record WebSocket messages
    pub record_websocket: bool,
    /// Record GraphQL requests
    pub record_graphql: bool,
    /// Record proxied requests (requests that are forwarded to real backends)
    /// When enabled, proxied requests/responses will be recorded with metadata indicating proxy source
    #[serde(default = "default_true")]
    pub record_proxy: bool,
}

impl Default for RecorderConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            database_path: "./mockforge-recordings.db".to_string(),
            api_enabled: true,
            api_port: None,
            max_requests: 10000,
            retention_days: 7,
            record_http: true,
            record_grpc: true,
            record_websocket: true,
            record_graphql: true,
            record_proxy: true,
        }
    }
}

/// Chaos engineering configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ChaosEngConfig {
    /// Enable chaos engineering
    pub enabled: bool,
    /// Latency injection configuration
    pub latency: Option<LatencyInjectionConfig>,
    /// Fault injection configuration
    pub fault_injection: Option<FaultConfig>,
    /// Rate limiting configuration
    pub rate_limit: Option<RateLimitingConfig>,
    /// Traffic shaping configuration
    pub traffic_shaping: Option<NetworkShapingConfig>,
    /// Predefined scenario to use
    pub scenario: Option<String>,
}

/// Latency injection configuration for chaos engineering
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct LatencyInjectionConfig {
    /// Enable latency injection
    pub enabled: bool,
    /// Fixed delay to inject (in milliseconds)
    pub fixed_delay_ms: Option<u64>,
    /// Random delay range (min_ms, max_ms) in milliseconds
    pub random_delay_range_ms: Option<(u64, u64)>,
    /// Jitter percentage to add variance to delays (0.0 to 1.0)
    pub jitter_percent: f64,
    /// Probability of injecting latency (0.0 to 1.0)
    pub probability: f64,
}

/// What "connection error" means at the wire level when injected.
///
/// Mirrors `mockforge_chaos::config::ConnectionErrorKind` so the YAML
/// loader can stay dep-free of the chaos crate. The bridge in `serve.rs`
/// converts to the chaos type at runtime.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ConnectionErrorKindConfig {
    /// HTTP 503 on a healthy connection (default, back-compat).
    #[default]
    Http503,
    /// TCP RST at accept time (`SO_LINGER=0` + drop).
    TcpReset,
    /// TCP FIN at accept time (clean drop, client sees EOF).
    TcpClose,
}

/// Payload corruption type. Mirrors `mockforge_chaos::config::CorruptionType`.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum CorruptionTypeConfig {
    /// No corruption.
    #[default]
    None,
    /// Replace random bytes with random values.
    RandomBytes,
    /// Truncate payload at random position.
    Truncate,
    /// Flip random bits in the payload.
    BitFlip,
}

/// Error injection pattern. Mirrors `mockforge_chaos::config::ErrorPattern`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", tag = "type")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ErrorPatternConfig {
    /// Burst pattern: inject N errors within a time interval.
    Burst {
        /// Number of errors to inject in the burst.
        count: usize,
        /// Time interval in milliseconds for the burst.
        interval_ms: u64,
    },
    /// Random pattern: inject errors with a probability.
    Random {
        /// Probability of injecting an error (0.0-1.0).
        probability: f64,
    },
    /// Sequential pattern: inject errors in a specific sequence.
    Sequential {
        /// Sequence of status codes to inject in order.
        sequence: Vec<u16>,
    },
}

/// Header presence / exact-value filter for the request matcher.
/// Mirrors `mockforge_chaos::request_matcher::HeaderMatch`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct HeaderMatchConfig {
    /// Header name (case-insensitive).
    pub name: String,
    /// Optional exact value. `None` = match on presence only.
    #[serde(default)]
    pub value: Option<String>,
}

/// Per-request matcher gating chaos injection.
/// Mirrors `mockforge_chaos::request_matcher::RequestMatcher`.
///
/// AND across populated fields; within a list, OR. Empty matcher = match all.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RequestMatcherConfig {
    /// CIDR ranges to match against the client IP. A bare IP without prefix
    /// is treated as `/32` (v4) or `/128` (v6). Empty = match any IP.
    #[serde(default)]
    pub source_ips: Vec<String>,
    /// Required headers. All entries must be satisfied (AND across the list).
    #[serde(default)]
    pub headers: Vec<HeaderMatchConfig>,
    /// Minimum request body size in bytes (inclusive).
    #[serde(default)]
    pub min_body_size_bytes: Option<usize>,
    /// Maximum request body size in bytes (inclusive).
    #[serde(default)]
    pub max_body_size_bytes: Option<usize>,
    /// `Some(true)` matches only chunked requests, `Some(false)` only
    /// non-chunked, `None` matches either.
    #[serde(default)]
    pub chunked_only: Option<bool>,
}

/// Fault injection configuration for chaos engineering.
///
/// Field set mirrors `mockforge_chaos::config::FaultInjectionConfig` so that
/// `--config chaos.yaml` can configure every fault knob the chaos middleware
/// understands. The bridge in `serve.rs` converts this to the chaos crate's
/// type at runtime.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct FaultConfig {
    /// Enable fault injection.
    pub enabled: bool,
    /// HTTP status codes to randomly return (e.g., `[500, 502, 503]`).
    pub http_errors: Vec<u16>,
    /// Probability of returning HTTP errors (0.0 to 1.0).
    pub http_error_probability: f64,
    /// Enable connection errors.
    pub connection_errors: bool,
    /// Probability of connection errors (0.0 to 1.0).
    pub connection_error_probability: f64,
    /// What "connection error" means at the wire level — HTTP 503 (default,
    /// back-compat), TCP RST, or TCP FIN. The TCP variants require the chaos
    /// listener wrapper, which the bridge installs automatically when
    /// connection_errors is enabled and this is `tcp_reset` / `tcp_close`.
    #[serde(default)]
    pub connection_error_kind: ConnectionErrorKindConfig,
    /// Enable timeout errors.
    pub timeout_errors: bool,
    /// Timeout duration in milliseconds.
    pub timeout_ms: u64,
    /// Probability of timeout errors (0.0 to 1.0).
    pub timeout_probability: f64,
    /// Enable partial-response truncation. Non-chunked responses keep the
    /// original `Content-Length` so clients see an unexpected EOF; chunked
    /// responses end without the terminating `0\r\n\r\n`.
    #[serde(default)]
    pub partial_responses: bool,
    /// Probability of truncating a response (0.0 to 1.0).
    #[serde(default)]
    pub partial_response_probability: f64,
    /// Enable payload corruption (post-response, before send).
    #[serde(default)]
    pub payload_corruption: bool,
    /// Probability of corrupting a response payload (0.0 to 1.0).
    #[serde(default)]
    pub payload_corruption_probability: f64,
    /// Type of corruption to apply when `payload_corruption` fires.
    #[serde(default)]
    pub corruption_type: CorruptionTypeConfig,
    /// Optional structured pattern for error injection (Burst / Random /
    /// Sequential). When `None`, the per-fault probabilities above govern
    /// firing independently.
    #[serde(default)]
    pub error_pattern: Option<ErrorPatternConfig>,
    /// Enable MockAI-driven dynamic error generation (advanced).
    #[serde(default)]
    pub mockai_enabled: bool,
    /// Optional per-request matcher. When set, faults only fire for
    /// requests that match (AND across populated fields). `None` = always.
    #[serde(default)]
    pub request_matcher: Option<RequestMatcherConfig>,
}

/// Rate limiting configuration for traffic control
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RateLimitingConfig {
    /// Enable rate limiting
    pub enabled: bool,
    /// Maximum requests per second allowed
    pub requests_per_second: u32,
    /// Maximum burst size before rate limiting kicks in
    pub burst_size: u32,
    /// Apply rate limiting per IP address
    pub per_ip: bool,
    /// Apply rate limiting per endpoint/path
    pub per_endpoint: bool,
}

/// Network shaping configuration for simulating network conditions
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct NetworkShapingConfig {
    /// Enable network shaping
    pub enabled: bool,
    /// Bandwidth limit in bits per second
    pub bandwidth_limit_bps: u64,
    /// Packet loss percentage (0.0 to 1.0)
    pub packet_loss_percent: f64,
    /// Maximum concurrent connections allowed
    pub max_connections: u32,
}

#[cfg(test)]
mod fault_config_yaml_tests {
    use super::*;

    /// The YAML shape promised in the issue-#79 reply must round-trip cleanly,
    /// including the new fields (`request_matcher`, `connection_error_kind`,
    /// `partial_responses`, `payload_corruption`, `error_pattern`).
    #[test]
    fn full_yaml_roundtrip() {
        let yaml = r#"
enabled: true
http_errors: [503]
http_error_probability: 0.5
connection_errors: true
connection_error_probability: 0.05
connection_error_kind: tcp_reset
timeout_errors: true
timeout_ms: 30000
timeout_probability: 0.1
partial_responses: true
partial_response_probability: 0.1
payload_corruption: true
payload_corruption_probability: 0.05
corruption_type: bit_flip
error_pattern:
  type: burst
  count: 5
  interval_ms: 1000
mockai_enabled: false
request_matcher:
  source_ips: ["10.0.0.0/8", "192.168.1.42"]
  headers:
    - name: x-test
      value: yes
    - name: x-debug
  min_body_size_bytes: 1048576
  chunked_only: true
"#;
        let parsed: FaultConfig = serde_yaml::from_str(yaml).expect("parse YAML");
        assert!(parsed.enabled);
        assert_eq!(parsed.http_errors, vec![503]);
        assert_eq!(parsed.connection_error_kind, ConnectionErrorKindConfig::TcpReset);
        assert!(parsed.partial_responses);
        assert_eq!(parsed.partial_response_probability, 0.1);
        assert!(parsed.payload_corruption);
        assert_eq!(parsed.corruption_type, CorruptionTypeConfig::BitFlip);
        match parsed.error_pattern.as_ref().expect("error_pattern present") {
            ErrorPatternConfig::Burst { count, interval_ms } => {
                assert_eq!(*count, 5);
                assert_eq!(*interval_ms, 1000);
            }
            other => panic!("expected Burst, got {other:?}"),
        }
        let m = parsed.request_matcher.expect("matcher present");
        assert_eq!(m.source_ips, vec!["10.0.0.0/8", "192.168.1.42"]);
        assert_eq!(m.headers.len(), 2);
        assert_eq!(m.headers[0].name, "x-test");
        // YAML `yes` parses as bool true; serde-yaml puts it back as the
        // string "true" when typed as Option<String>. The exact value is
        // implementation-dependent; assert it's *some* non-empty value.
        assert!(m.headers[0].value.as_deref().is_some_and(|s| !s.is_empty()));
        assert!(m.headers[1].value.is_none());
        assert_eq!(m.min_body_size_bytes, Some(1048576));
        assert_eq!(m.chunked_only, Some(true));
    }

    /// Backward-compat: a YAML file written for the old (pre-0.3.129) FaultConfig
    /// must still parse without listing any of the new fields.
    #[test]
    fn legacy_yaml_without_new_fields_still_parses() {
        let yaml = r#"
enabled: true
http_errors: [500, 502, 503]
http_error_probability: 0.1
connection_errors: false
connection_error_probability: 0.0
timeout_errors: false
timeout_ms: 5000
timeout_probability: 0.0
"#;
        let parsed: FaultConfig = serde_yaml::from_str(yaml).expect("parse legacy YAML");
        assert!(parsed.enabled);
        assert_eq!(parsed.http_errors, vec![500, 502, 503]);
        // New fields default cleanly.
        assert_eq!(parsed.connection_error_kind, ConnectionErrorKindConfig::Http503);
        assert!(!parsed.partial_responses);
        assert!(!parsed.payload_corruption);
        assert!(parsed.error_pattern.is_none());
        assert!(parsed.request_matcher.is_none());
    }

    /// Same yaml that the live server failed on — read from disk via the
    /// production config-loader path (load_config), not just inline string.
    #[test]
    fn fault_config_via_load_config_from_disk() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("chaos.yaml");
        std::fs::write(&path, "observability:\n  chaos:\n    enabled: true\n    fault_injection:\n      enabled: true\n      http_errors: [503]\n      http_error_probability: 1.0\n      connection_errors: false\n      connection_error_probability: 0.0\n      timeout_errors: false\n      timeout_ms: 5000\n      timeout_probability: 0.0\n").unwrap();
        let cfg = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(crate::config::load_config(&path))
            .expect("load");
        let fi = cfg
            .observability
            .chaos
            .expect("chaos")
            .fault_injection
            .expect("fault_injection");
        assert_eq!(fi.http_error_probability, 1.0);
        assert_eq!(fi.timeout_ms, 5000);
    }

    /// Issue #79 follow-up (Srikanth's 3rd-round reply):
    /// my reply pointed at `--config chaos.yaml` for the new chaos fields,
    /// but his live server reports `http_error_probability: 0.05` even
    /// though the YAML says `1.0`. Reproduce against the real
    /// `ServerConfig` deserialization path that mockforge uses at startup,
    /// not just the inner FaultConfig in isolation.
    #[test]
    fn fault_config_through_server_config_observability_chaos() {
        let yaml = r#"
observability:
  chaos:
    enabled: true
    fault_injection:
      enabled: true
      http_errors: [503]
      http_error_probability: 1.0
      connection_errors: false
      connection_error_probability: 0.0
      timeout_errors: false
      timeout_ms: 5000
      timeout_probability: 0.0
"#;
        let cfg: crate::config::ServerConfig = serde_yaml::from_str(yaml).expect("parse");
        let chaos = cfg.observability.chaos.expect("chaos present");
        assert!(chaos.enabled);
        let fi = chaos.fault_injection.expect("fault_injection present");
        assert_eq!(
            fi.http_error_probability, 1.0,
            "http_error_probability lost during nested deserialization"
        );
        assert_eq!(fi.timeout_ms, 5000, "timeout_ms lost during nested deserialization");
    }

    /// The connection_error_kind enum must serialize as snake_case strings
    /// (not the Rust variant name) for YAML compatibility with the chaos crate's
    /// own enum. Same for corruption_type.
    #[test]
    fn enum_serde_uses_snake_case() {
        let cek = ConnectionErrorKindConfig::TcpClose;
        assert_eq!(serde_yaml::to_string(&cek).unwrap().trim(), "tcp_close");
        let ct = CorruptionTypeConfig::RandomBytes;
        assert_eq!(serde_yaml::to_string(&ct).unwrap().trim(), "random_bytes");
    }
}
