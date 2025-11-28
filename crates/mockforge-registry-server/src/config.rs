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
