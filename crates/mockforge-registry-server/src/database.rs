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
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        Ok(())
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
        let total: (Option<i64>,) =
            sqlx::query_as("SELECT COALESCE(SUM(downloads_total), 0) FROM plugins")
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
