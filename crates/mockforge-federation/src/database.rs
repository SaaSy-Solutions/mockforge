//! Database persistence for federations
//!
//! Provides methods to store and retrieve federation configurations from the database.

use crate::federation::Federation;
use crate::service::{ServiceBoundary, ServiceRealityLevel};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde_json;
use sqlx::{sqlite::SqlitePool, Row};
use tracing::info;
use uuid::Uuid;

/// Database layer for federation persistence
pub struct FederationDatabase {
    pool: SqlitePool,
}

impl FederationDatabase {
    /// Create a new federation database instance
    pub async fn new(pool: SqlitePool) -> Result<Self> {
        // Run migrations
        // Note: In a real implementation, you'd use sqlx::migrate! macro
        // For now, we'll run migrations manually or via a migration runner
        // sqlx::migrate!("./migrations")
        //     .run(&pool)
        //     .await
        //     .context("Failed to run federation migrations")?;

        Ok(Self { pool })
    }

    /// Run migrations manually
    pub async fn run_migrations(&self) -> Result<()> {
        let migration_sql = include_str!("../migrations/001_federation.sql");

        sqlx::query(migration_sql)
            .execute(&self.pool)
            .await
            .context("Failed to run federation migrations")?;

        info!("Federation database migrations completed");
        Ok(())
    }

    /// Create a new federation
    pub async fn create_federation(&self, federation: &Federation) -> Result<()> {
        let id_str = federation.id.to_string();
        let org_id_str = federation.org_id.to_string();
        let created_at = federation.created_at.timestamp();
        let updated_at = federation.updated_at.timestamp();

        sqlx::query(
            r"
            INSERT INTO federations (id, name, org_id, description, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ",
        )
        .bind(&id_str)
        .bind(&federation.name)
        .bind(&org_id_str)
        .bind(&federation.description)
        .bind(created_at)
        .bind(updated_at)
        .execute(&self.pool)
        .await
        .context("Failed to create federation")?;

        // Insert services
        for service in &federation.services {
            self.create_federation_service(&id_str, service).await?;
        }

        info!(
            federation_id = %federation.id,
            federation_name = %federation.name,
            "Created federation"
        );

        Ok(())
    }

    /// Create a federation service
    async fn create_federation_service(
        &self,
        federation_id: &str,
        service: &ServiceBoundary,
    ) -> Result<()> {
        let service_id = Uuid::new_v4().to_string();
        let workspace_id_str = service.workspace_id.to_string();
        let config_json =
            serde_json::to_string(&service.config).context("Failed to serialize service config")?;
        let dependencies_json = serde_json::to_string(&service.dependencies)
            .context("Failed to serialize dependencies")?;
        let created_at = Utc::now().timestamp();

        sqlx::query(
            r"
            INSERT INTO federation_services
            (id, federation_id, service_name, workspace_id, base_path, reality_level, config, dependencies, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            ",
        )
        .bind(&service_id)
        .bind(federation_id)
        .bind(&service.name)
        .bind(&workspace_id_str)
        .bind(&service.base_path)
        .bind(service.reality_level.as_str())
        .bind(&config_json)
        .bind(&dependencies_json)
        .bind(created_at)
        .execute(&self.pool)
        .await
        .context("Failed to create federation service")?;

        Ok(())
    }

    /// Get a federation by ID
    pub async fn get_federation(&self, federation_id: &Uuid) -> Result<Option<Federation>> {
        let id_str = federation_id.to_string();

        let row = sqlx::query(
            r"
            SELECT id, name, org_id, description, created_at, updated_at
            FROM federations
            WHERE id = ?1
            ",
        )
        .bind(&id_str)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query federation")?;

        if let Some(row) = row {
            let id = Uuid::parse_str(row.get::<String, _>(0).as_str())
                .context("Invalid federation ID")?;
            let name: String = row.get(1);
            let org_id =
                Uuid::parse_str(row.get::<String, _>(2).as_str()).context("Invalid org ID")?;
            let description: String = row.get(3);
            let created_at = DateTime::from_timestamp(row.get::<i64, _>(4), 0)
                .unwrap_or_else(Utc::now)
                .with_timezone(&Utc);
            let updated_at = DateTime::from_timestamp(row.get::<i64, _>(5), 0)
                .unwrap_or_else(Utc::now)
                .with_timezone(&Utc);

            // Load services
            let services = self.get_federation_services(&id_str).await?;

            Ok(Some(Federation {
                id,
                name,
                description,
                org_id,
                services,
                created_at,
                updated_at,
            }))
        } else {
            Ok(None)
        }
    }

    /// Get all services for a federation
    async fn get_federation_services(&self, federation_id: &str) -> Result<Vec<ServiceBoundary>> {
        let rows = sqlx::query(
            r"
            SELECT service_name, workspace_id, base_path, reality_level, config, dependencies
            FROM federation_services
            WHERE federation_id = ?1
            ORDER BY base_path
            ",
        )
        .bind(federation_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query federation services")?;

        let mut services = Vec::new();

        for row in rows {
            let name: String = row.get(0);
            let workspace_id = Uuid::parse_str(row.get::<String, _>(1).as_str())
                .context("Invalid workspace ID")?;
            let base_path: String = row.get(2);
            let reality_level_str: String = row.get(3);
            let config_json: String = row.get(4);
            let dependencies_json: String = row.get(5);

            let reality_level = ServiceRealityLevel::from_str(&reality_level_str)
                .ok_or_else(|| anyhow::anyhow!("Invalid reality level: {reality_level_str}"))?;

            let config: std::collections::HashMap<String, serde_json::Value> =
                serde_json::from_str(&config_json).context("Failed to parse service config")?;
            let dependencies: Vec<String> =
                serde_json::from_str(&dependencies_json).context("Failed to parse dependencies")?;

            let mut service = ServiceBoundary::new(name, workspace_id, base_path, reality_level);
            service.config = config;
            service.dependencies = dependencies;

            services.push(service);
        }

        Ok(services)
    }

    /// List all federations for an organization
    pub async fn list_federations(&self, org_id: &Uuid) -> Result<Vec<Federation>> {
        let org_id_str = org_id.to_string();

        let rows = sqlx::query(
            r"
            SELECT id, name, org_id, description, created_at, updated_at
            FROM federations
            WHERE org_id = ?1
            ORDER BY created_at DESC
            ",
        )
        .bind(&org_id_str)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query federations")?;

        let mut federations = Vec::new();

        for row in rows {
            let id = Uuid::parse_str(row.get::<String, _>(0).as_str())
                .context("Invalid federation ID")?;
            let name: String = row.get(1);
            let org_id =
                Uuid::parse_str(row.get::<String, _>(2).as_str()).context("Invalid org ID")?;
            let description: String = row.get(3);
            let created_at = DateTime::from_timestamp(row.get::<i64, _>(4), 0)
                .unwrap_or_else(Utc::now)
                .with_timezone(&Utc);
            let updated_at = DateTime::from_timestamp(row.get::<i64, _>(5), 0)
                .unwrap_or_else(Utc::now)
                .with_timezone(&Utc);

            let id_str = id.to_string();
            let services = self.get_federation_services(&id_str).await?;

            federations.push(Federation {
                id,
                name,
                description,
                org_id,
                services,
                created_at,
                updated_at,
            });
        }

        Ok(federations)
    }

    /// Update a federation
    pub async fn update_federation(&self, federation: &Federation) -> Result<()> {
        let id_str = federation.id.to_string();
        let updated_at = Utc::now().timestamp();

        sqlx::query(
            r"
            UPDATE federations
            SET name = ?1, description = ?2, updated_at = ?3
            WHERE id = ?4
            ",
        )
        .bind(&federation.name)
        .bind(&federation.description)
        .bind(updated_at)
        .bind(&id_str)
        .execute(&self.pool)
        .await
        .context("Failed to update federation")?;

        // Delete existing services and recreate
        sqlx::query("DELETE FROM federation_services WHERE federation_id = ?1")
            .bind(&id_str)
            .execute(&self.pool)
            .await
            .context("Failed to delete existing services")?;

        // Recreate services
        for service in &federation.services {
            self.create_federation_service(&id_str, service).await?;
        }

        info!(
            federation_id = %federation.id,
            "Updated federation"
        );

        Ok(())
    }

    /// Delete a federation
    pub async fn delete_federation(&self, federation_id: &Uuid) -> Result<()> {
        let id_str = federation_id.to_string();

        sqlx::query("DELETE FROM federations WHERE id = ?1")
            .bind(&id_str)
            .execute(&self.pool)
            .await
            .context("Failed to delete federation")?;

        info!(
            federation_id = %federation_id,
            "Deleted federation"
        );

        Ok(())
    }
}
