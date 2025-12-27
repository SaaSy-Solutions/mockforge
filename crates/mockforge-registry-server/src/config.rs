//! Server configuration

use anyhow::Result;
use serde::Deserialize;

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
}

impl Config {
    pub fn load() -> Result<Self> {
        dotenvy::dotenv().ok();

        let config = Self {
            port: std::env::var("PORT").unwrap_or_else(|_| "8080".to_string()).parse()?,
            database_url: std::env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
            jwt_secret: std::env::var("JWT_SECRET").expect("JWT_SECRET must be set"),
            s3_bucket: std::env::var("S3_BUCKET")
                .unwrap_or_else(|_| "mockforge-plugins".to_string()),
            s3_region: std::env::var("S3_REGION").unwrap_or_else(|_| "us-east-1".to_string()),
            s3_endpoint: std::env::var("S3_ENDPOINT").ok(),
            max_plugin_size: std::env::var("MAX_PLUGIN_SIZE")
                .unwrap_or_else(|_| "52428800".to_string()) // 50MB
                .parse()?,
            rate_limit_per_minute: std::env::var("RATE_LIMIT_PER_MINUTE")
                .unwrap_or_else(|_| "60".to_string())
                .parse()?,
            analytics_db_path: std::env::var("ANALYTICS_DB_PATH").ok(),
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
    }

    #[test]
    fn test_config_missing_required_database_url() {
        let _guard = ENV_MUTEX.lock().unwrap();
        std::env::remove_var("DATABASE_URL");
        std::env::set_var("JWT_SECRET", "test-secret");

        let result = std::panic::catch_unwind(|| Config::load());

        assert!(result.is_err());

        // Clean up
        std::env::remove_var("JWT_SECRET");
    }

    #[test]
    fn test_config_missing_required_jwt_secret() {
        let _guard = ENV_MUTEX.lock().unwrap();
        std::env::set_var("DATABASE_URL", "postgres://localhost/test");
        std::env::remove_var("JWT_SECRET");

        let result = std::panic::catch_unwind(|| Config::load());

        assert!(result.is_err());

        // Clean up
        std::env::remove_var("DATABASE_URL");
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
