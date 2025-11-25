//! Database connection and migration support for mockforge-http
//!
//! This module provides optional database support for persistent storage
//! of drift budgets, incidents, and consumer contracts.

#[cfg(feature = "database")]
use anyhow::Result as AnyhowResult;
#[cfg(feature = "database")]
use sqlx::{postgres::PgPoolOptions, PgPool};
#[cfg(feature = "database")]
use std::sync::Arc;

/// Database connection wrapper
#[derive(Clone)]
pub struct Database {
    #[cfg(feature = "database")]
    pool: Option<Arc<PgPool>>,
    #[cfg(not(feature = "database"))]
    _phantom: std::marker::PhantomData<()>,
}

impl Database {
    /// Create a new database connection (optional)
    ///
    /// If DATABASE_URL is not set or database feature is disabled,
    /// returns a Database with no connection.
    /// This allows the application to run without a database.
    #[cfg(feature = "database")]
    pub async fn connect_optional(database_url: Option<&str>) -> AnyhowResult<Self> {
        let pool = if let Some(url) = database_url {
            if url.is_empty() {
                None
            } else {
                let pool = PgPoolOptions::new().max_connections(10).connect(url).await?;
                Some(Arc::new(pool))
            }
        } else {
            None
        };

        Ok(Self { pool })
    }

    /// Connect to database (no-op when database feature is disabled)
    #[cfg(not(feature = "database"))]
    pub async fn connect_optional(_database_url: Option<&str>) -> anyhow::Result<Self> {
        Ok(Self {
            _phantom: std::marker::PhantomData,
        })
    }

    /// Run migrations if database is connected
    #[cfg(feature = "database")]
    pub async fn migrate_if_connected(&self) -> AnyhowResult<()> {
        if let Some(ref pool) = self.pool {
            // Run migrations from the migrations directory
            // Note: This requires the migrations directory to be accessible at runtime
            match sqlx::migrate!("./migrations").run(pool.as_ref()).await {
                Ok(_) => {
                    tracing::info!("Database migrations completed successfully");
                    Ok(())
                }
                Err(e) => {
                    // If migration was manually applied, log warning but continue
                    if e.to_string().contains("previously applied but is missing") {
                        tracing::warn!(
                            "Migration tracking issue (manually applied migration): {:?}",
                            e
                        );
                        tracing::info!(
                            "Continuing despite migration tracking issue - database is up to date"
                        );
                        Ok(())
                    } else {
                        Err(e.into())
                    }
                }
            }
        } else {
            tracing::debug!("No database connection, skipping migrations");
            Ok(())
        }
    }

    /// Run database migrations (no-op when database feature is disabled)
    #[cfg(not(feature = "database"))]
    pub async fn migrate_if_connected(&self) -> anyhow::Result<()> {
        tracing::debug!("Database feature not enabled, skipping migrations");
        Ok(())
    }

    /// Get the database pool if connected
    #[cfg(feature = "database")]
    pub fn pool(&self) -> Option<&PgPool> {
        self.pool.as_deref()
    }

    /// Get the database pool (returns None when database feature is disabled)
    #[cfg(not(feature = "database"))]
    pub fn pool(&self) -> Option<()> {
        None
    }

    /// Check if database is connected
    pub fn is_connected(&self) -> bool {
        #[cfg(feature = "database")]
        {
            self.pool.is_some()
        }
        #[cfg(not(feature = "database"))]
        {
            false
        }
    }
}
