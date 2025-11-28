//! Database connection and models

use anyhow::Result;
use sqlx::{postgres::PgPoolOptions, PgPool};

#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    pub async fn connect(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new().max_connections(5).connect(database_url).await?;

        Ok(Self { pool })
    }

    pub async fn migrate(&self) -> Result<()> {
        // Run migrations - ignore "previously applied but missing" errors for manually applied migrations
        match sqlx::migrate!("./migrations").run(&self.pool).await {
            Ok(_) => Ok(()),
            Err(e) => {
                // If migration was manually applied (e.g., timestamp fix), log warning but continue
                if e.to_string().contains("previously applied but is missing") {
                    tracing::warn!("Migration tracking issue (manually applied migration): {:?}", e);
                    tracing::info!("Continuing despite migration tracking issue - database is up to date");
                    Ok(())
                } else {
                    Err(e.into())
                }
            }
        }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Get total number of plugins
    pub async fn get_total_plugins(&self) -> Result<i64> {
        let count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM plugins").fetch_one(&self.pool).await?;
        Ok(count.0)
    }

    /// Get total downloads across all plugins
    pub async fn get_total_downloads(&self) -> Result<i64> {
        // downloads_total is NUMERIC in database, so we need to cast it
        let total: (Option<i64>,) =
            sqlx::query_as("SELECT COALESCE(SUM(downloads_total)::BIGINT, 0) FROM plugins")
                .fetch_one(&self.pool)
                .await?;
        Ok(total.0.unwrap_or(0))
    }

    /// Get total number of users
    pub async fn get_total_users(&self) -> Result<i64> {
        let count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM users").fetch_one(&self.pool).await?;
        Ok(count.0)
    }
}
