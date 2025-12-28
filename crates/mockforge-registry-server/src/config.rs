//! Server configuration

use anyhow::{Context, Result};
use serde::Deserialize;

/// Helper to get a required environment variable with a descriptive error
fn required_env(name: &str) -> Result<String> {
    std::env::var(name).with_context(|| {
        format!(
            "Required environment variable '{name}' is not set. \
             Please set it before starting the server."
        )
    })
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// Server port
    pub port: u16,

    /// Database connection URL
    pub database_url: String,

    /// JWT secret for authentication
    pub jwt_secret: String,

    /// S3 configuration
    pub s3_bucket: String,
    pub s3_region: String,
    pub s3_endpoint: Option<String>, // For MinIO/custom S3

    /// Upload limits
    pub max_plugin_size: usize, // in bytes (default 50MB)

    /// Rate limiting
    pub rate_limit_per_minute: u32,

    /// Analytics database path (optional, defaults to "mockforge-analytics.db" in current directory)
    pub analytics_db_path: Option<String>,

    /// Graceful shutdown timeout in seconds
    pub shutdown_timeout_secs: u64,

    /// Redis URL for caching and temporary storage (optional)
    pub redis_url: Option<String>,

    /// Whether two-factor authentication is enabled (requires Redis)
    pub two_factor_enabled: Option<bool>,

    /// Base URL of the application (for OAuth callbacks and email links)
    pub app_base_url: String,

    /// Stripe secret key for billing
    pub stripe_secret_key: Option<String>,

    /// Stripe price ID for Pro plan
    pub stripe_price_id_pro: Option<String>,

    /// Stripe price ID for Team plan
    pub stripe_price_id_team: Option<String>,

    /// Stripe webhook secret for verifying webhook signatures
    pub stripe_webhook_secret: Option<String>,

    /// GitHub OAuth client ID
    pub oauth_github_client_id: Option<String>,

    /// GitHub OAuth client secret
    pub oauth_github_client_secret: Option<String>,

    /// Google OAuth client ID
    pub oauth_google_client_id: Option<String>,

    /// Google OAuth client secret
    pub oauth_google_client_secret: Option<String>,
}

impl Config {
    /// Load configuration from environment variables.
    ///
    /// Required environment variables:
    /// - `DATABASE_URL`: Database connection URL
    /// - `JWT_SECRET`: Secret key for JWT token signing
    ///
    /// Optional environment variables (with defaults):
    /// - `PORT`: Server port (default: 8080)
    /// - `S3_BUCKET`: S3 bucket name (default: "mockforge-plugins")
    /// - `S3_REGION`: S3 region (default: "us-east-1")
    /// - `S3_ENDPOINT`: Custom S3 endpoint for MinIO/compatible storage
    /// - `MAX_PLUGIN_SIZE`: Maximum plugin size in bytes (default: 52428800 / 50MB)
    /// - `RATE_LIMIT_PER_MINUTE`: Rate limit per minute (default: 60)
    /// - `ANALYTICS_DB_PATH`: Path to analytics database
    /// - `SHUTDOWN_TIMEOUT_SECS`: Graceful shutdown timeout in seconds (default: 30)
    pub fn load() -> Result<Self> {
        dotenvy::dotenv().ok();

        // Collect all missing required variables first for better error reporting
        let mut missing_vars = Vec::new();

        let database_url = match required_env("DATABASE_URL") {
            Ok(url) => Some(url),
            Err(_) => {
                missing_vars.push("DATABASE_URL");
                None
            }
        };

        let jwt_secret = match required_env("JWT_SECRET") {
            Ok(secret) => Some(secret),
            Err(_) => {
                missing_vars.push("JWT_SECRET");
                None
            }
        };

        // Report all missing required variables at once
        if !missing_vars.is_empty() {
            anyhow::bail!(
                "Missing required environment variables: {}. \
                 Please ensure these are set before starting the server.",
                missing_vars.join(", ")
            );
        }

        let config = Self {
            port: std::env::var("PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .context("PORT must be a valid port number (0-65535)")?,
            database_url: database_url.unwrap(),
            jwt_secret: jwt_secret.unwrap(),
            s3_bucket: std::env::var("S3_BUCKET")
                .unwrap_or_else(|_| "mockforge-plugins".to_string()),
            s3_region: std::env::var("S3_REGION").unwrap_or_else(|_| "us-east-1".to_string()),
            s3_endpoint: std::env::var("S3_ENDPOINT").ok(),
            max_plugin_size: std::env::var("MAX_PLUGIN_SIZE")
                .unwrap_or_else(|_| "52428800".to_string()) // 50MB
                .parse()
                .context("MAX_PLUGIN_SIZE must be a valid number")?,
            rate_limit_per_minute: std::env::var("RATE_LIMIT_PER_MINUTE")
                .unwrap_or_else(|_| "60".to_string())
                .parse()
                .context("RATE_LIMIT_PER_MINUTE must be a valid number")?,
            analytics_db_path: std::env::var("ANALYTICS_DB_PATH").ok(),
            shutdown_timeout_secs: std::env::var("SHUTDOWN_TIMEOUT_SECS")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .context("SHUTDOWN_TIMEOUT_SECS must be a valid number")?,
            redis_url: std::env::var("REDIS_URL").ok(),
            two_factor_enabled: std::env::var("TWO_FACTOR_ENABLED")
                .ok()
                .map(|v| v.to_lowercase() == "true" || v == "1"),
            app_base_url: std::env::var("APP_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:3000".to_string()),
            stripe_secret_key: std::env::var("STRIPE_SECRET_KEY").ok(),
            stripe_price_id_pro: std::env::var("STRIPE_PRICE_ID_PRO").ok(),
            stripe_price_id_team: std::env::var("STRIPE_PRICE_ID_TEAM").ok(),
            stripe_webhook_secret: std::env::var("STRIPE_WEBHOOK_SECRET").ok(),
            oauth_github_client_id: std::env::var("OAUTH_GITHUB_CLIENT_ID").ok(),
            oauth_github_client_secret: std::env::var("OAUTH_GITHUB_CLIENT_SECRET").ok(),
            oauth_google_client_id: std::env::var("OAUTH_GOOGLE_CLIENT_ID").ok(),
            oauth_google_client_secret: std::env::var("OAUTH_GOOGLE_CLIENT_SECRET").ok(),
        };

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Mutex to serialize tests that modify environment variables
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn test_config_defaults() {
        let _guard = ENV_MUTEX.lock().unwrap();
        // Set required env vars
        std::env::set_var("DATABASE_URL", "postgres://localhost/test");
        std::env::set_var("JWT_SECRET", "test-secret");

        let config = Config::load().unwrap();

        // Check defaults
        assert_eq!(config.s3_bucket, "mockforge-plugins");
        assert_eq!(config.s3_region, "us-east-1");
        assert_eq!(config.max_plugin_size, 52428800); // 50MB
        assert_eq!(config.rate_limit_per_minute, 60);
        assert!(config.s3_endpoint.is_none());
        assert!(config.analytics_db_path.is_none());
        assert_eq!(config.shutdown_timeout_secs, 30); // Default shutdown timeout

        // Clean up
        std::env::remove_var("DATABASE_URL");
        std::env::remove_var("JWT_SECRET");
    }

    #[test]
    fn test_config_custom_values() {
        let _guard = ENV_MUTEX.lock().unwrap();
        // Set all env vars
        std::env::set_var("PORT", "9090");
        std::env::set_var("DATABASE_URL", "postgres://custom/db");
        std::env::set_var("JWT_SECRET", "custom-secret");
        std::env::set_var("S3_BUCKET", "custom-bucket");
        std::env::set_var("S3_REGION", "eu-west-1");
        std::env::set_var("S3_ENDPOINT", "http://localhost:9000");
        std::env::set_var("MAX_PLUGIN_SIZE", "10485760"); // 10MB
        std::env::set_var("RATE_LIMIT_PER_MINUTE", "120");
        std::env::set_var("ANALYTICS_DB_PATH", "/custom/path/analytics.db");
        std::env::set_var("SHUTDOWN_TIMEOUT_SECS", "60");

        let config = Config::load().unwrap();

        assert_eq!(config.port, 9090);
        assert_eq!(config.database_url, "postgres://custom/db");
        assert_eq!(config.jwt_secret, "custom-secret");
        assert_eq!(config.s3_bucket, "custom-bucket");
        assert_eq!(config.s3_region, "eu-west-1");
        assert_eq!(config.s3_endpoint, Some("http://localhost:9000".to_string()));
        assert_eq!(config.max_plugin_size, 10485760);
        assert_eq!(config.rate_limit_per_minute, 120);
        assert_eq!(config.analytics_db_path, Some("/custom/path/analytics.db".to_string()));
        assert_eq!(config.shutdown_timeout_secs, 60);

        // Clean up
        std::env::remove_var("PORT");
        std::env::remove_var("DATABASE_URL");
        std::env::remove_var("JWT_SECRET");
        std::env::remove_var("S3_BUCKET");
        std::env::remove_var("S3_REGION");
        std::env::remove_var("S3_ENDPOINT");
        std::env::remove_var("MAX_PLUGIN_SIZE");
        std::env::remove_var("RATE_LIMIT_PER_MINUTE");
        std::env::remove_var("ANALYTICS_DB_PATH");
        std::env::remove_var("SHUTDOWN_TIMEOUT_SECS");
    }

    #[test]
    fn test_config_missing_required_database_url() {
        let _guard = ENV_MUTEX.lock().unwrap();
        std::env::remove_var("DATABASE_URL");
        std::env::set_var("JWT_SECRET", "test-secret");

        let result = Config::load();

        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("DATABASE_URL"),
            "Error should mention DATABASE_URL: {error_msg}"
        );

        // Clean up
        std::env::remove_var("JWT_SECRET");
    }

    #[test]
    fn test_config_missing_required_jwt_secret() {
        let _guard = ENV_MUTEX.lock().unwrap();
        std::env::set_var("DATABASE_URL", "postgres://localhost/test");
        std::env::remove_var("JWT_SECRET");

        let result = Config::load();

        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("JWT_SECRET"), "Error should mention JWT_SECRET: {error_msg}");

        // Clean up
        std::env::remove_var("DATABASE_URL");
    }

    #[test]
    fn test_config_missing_both_required_vars() {
        let _guard = ENV_MUTEX.lock().unwrap();
        std::env::remove_var("DATABASE_URL");
        std::env::remove_var("JWT_SECRET");

        let result = Config::load();

        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        // Should report both missing variables
        assert!(
            error_msg.contains("DATABASE_URL") && error_msg.contains("JWT_SECRET"),
            "Error should mention both missing variables: {error_msg}"
        );
    }

    #[test]
    fn test_config_invalid_port() {
        let _guard = ENV_MUTEX.lock().unwrap();
        std::env::set_var("PORT", "invalid");
        std::env::set_var("DATABASE_URL", "postgres://localhost/test");
        std::env::set_var("JWT_SECRET", "test-secret");

        let result = Config::load();
        assert!(result.is_err());

        // Clean up
        std::env::remove_var("PORT");
        std::env::remove_var("DATABASE_URL");
        std::env::remove_var("JWT_SECRET");
    }

    #[test]
    fn test_config_invalid_max_plugin_size() {
        let _guard = ENV_MUTEX.lock().unwrap();
        std::env::set_var("DATABASE_URL", "postgres://localhost/test");
        std::env::set_var("JWT_SECRET", "test-secret");
        std::env::set_var("MAX_PLUGIN_SIZE", "not-a-number");

        let result = Config::load();
        assert!(result.is_err());

        // Clean up
        std::env::remove_var("DATABASE_URL");
        std::env::remove_var("JWT_SECRET");
        std::env::remove_var("MAX_PLUGIN_SIZE");
    }

    #[test]
    fn test_config_invalid_rate_limit() {
        let _guard = ENV_MUTEX.lock().unwrap();
        std::env::set_var("DATABASE_URL", "postgres://localhost/test");
        std::env::set_var("JWT_SECRET", "test-secret");
        std::env::set_var("RATE_LIMIT_PER_MINUTE", "not-a-number");

        let result = Config::load();
        assert!(result.is_err());

        // Clean up
        std::env::remove_var("DATABASE_URL");
        std::env::remove_var("JWT_SECRET");
        std::env::remove_var("RATE_LIMIT_PER_MINUTE");
    }

    #[test]
    fn test_config_port_boundary_values() {
        let _guard = ENV_MUTEX.lock().unwrap();
        std::env::set_var("DATABASE_URL", "postgres://localhost/test");
        std::env::set_var("JWT_SECRET", "test-secret");

        // Test port 0
        std::env::set_var("PORT", "0");
        let config = Config::load().unwrap();
        assert_eq!(config.port, 0);

        // Test max port
        std::env::set_var("PORT", "65535");
        let config = Config::load().unwrap();
        assert_eq!(config.port, 65535);

        // Clean up
        std::env::remove_var("PORT");
        std::env::remove_var("DATABASE_URL");
        std::env::remove_var("JWT_SECRET");
    }

    #[test]
    fn test_config_clone() {
        let _guard = ENV_MUTEX.lock().unwrap();
        std::env::set_var("DATABASE_URL", "postgres://localhost/test");
        std::env::set_var("JWT_SECRET", "test-secret");

        let config = Config::load().unwrap();
        let cloned = config.clone();

        assert_eq!(config.database_url, cloned.database_url);
        assert_eq!(config.jwt_secret, cloned.jwt_secret);
        assert_eq!(config.port, cloned.port);

        // Clean up
        std::env::remove_var("DATABASE_URL");
        std::env::remove_var("JWT_SECRET");
    }

    #[test]
    fn test_config_debug() {
        let _guard = ENV_MUTEX.lock().unwrap();
        std::env::set_var("DATABASE_URL", "postgres://localhost/test");
        std::env::set_var("JWT_SECRET", "test-secret");

        let config = Config::load().unwrap();
        let debug_str = format!("{:?}", config);

        // Should contain field names
        assert!(debug_str.contains("port"));
        assert!(debug_str.contains("database_url"));

        // Clean up
        std::env::remove_var("DATABASE_URL");
        std::env::remove_var("JWT_SECRET");
    }
}
