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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::federation::{Federation, FederationConfig, FederationService};
    use crate::service::{ServiceBoundary, ServiceRealityLevel};
    use std::collections::HashMap;
    use tempfile::TempDir;

    async fn create_test_db() -> (FederationDatabase, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db_url = format!("sqlite://{}?mode=rwc", db_path.display());

        let pool = SqlitePool::connect(&db_url).await.unwrap();
        let db = FederationDatabase::new(pool).await.unwrap();
        db.run_migrations().await.unwrap();

        (db, temp_dir)
    }

    fn create_test_federation() -> Federation {
        let org_id = Uuid::new_v4();
        let workspace_id1 = Uuid::new_v4();
        let workspace_id2 = Uuid::new_v4();

        let mut service1 = ServiceBoundary::new(
            "auth".to_string(),
            workspace_id1,
            "/auth".to_string(),
            ServiceRealityLevel::Real,
        );
        service1.config.insert("timeout".to_string(), serde_json::json!(5000));
        service1.dependencies.push("database".to_string());

        let service2 = ServiceBoundary::new(
            "payments".to_string(),
            workspace_id2,
            "/payments".to_string(),
            ServiceRealityLevel::MockV3,
        );

        Federation {
            id: Uuid::new_v4(),
            name: "test-federation".to_string(),
            description: "Test federation for unit tests".to_string(),
            org_id,
            services: vec![service1, service2],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_new_database() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db_url = format!("sqlite://{}?mode=rwc", db_path.display());

        let pool = SqlitePool::connect(&db_url).await.unwrap();
        let result = FederationDatabase::new(pool).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_run_migrations() {
        let (db, _temp_dir) = create_test_db().await;

        // Verify tables were created by trying to query them
        let result = sqlx::query("SELECT COUNT(*) FROM federations").fetch_one(&db.pool).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_federation() {
        let (db, _temp_dir) = create_test_db().await;
        let federation = create_test_federation();

        let result = db.create_federation(&federation).await;
        assert!(result.is_ok());

        // Verify it was inserted
        let row = sqlx::query("SELECT COUNT(*) FROM federations WHERE id = ?1")
            .bind(federation.id.to_string())
            .fetch_one(&db.pool)
            .await
            .unwrap();

        let count: i64 = row.get(0);
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_create_federation_with_services() {
        let (db, _temp_dir) = create_test_db().await;
        let federation = create_test_federation();
        let federation_id = federation.id;

        db.create_federation(&federation).await.unwrap();

        // Verify services were created
        let rows = sqlx::query("SELECT COUNT(*) FROM federation_services WHERE federation_id = ?1")
            .bind(federation_id.to_string())
            .fetch_one(&db.pool)
            .await
            .unwrap();

        let count: i64 = rows.get(0);
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_get_federation() {
        let (db, _temp_dir) = create_test_db().await;
        let original = create_test_federation();
        let federation_id = original.id;

        db.create_federation(&original).await.unwrap();

        let result = db.get_federation(&federation_id).await.unwrap();
        assert!(result.is_some());

        let retrieved = result.unwrap();
        assert_eq!(retrieved.id, original.id);
        assert_eq!(retrieved.name, original.name);
        assert_eq!(retrieved.description, original.description);
        assert_eq!(retrieved.org_id, original.org_id);
        assert_eq!(retrieved.services.len(), original.services.len());
    }

    #[tokio::test]
    async fn test_get_federation_not_found() {
        let (db, _temp_dir) = create_test_db().await;
        let non_existent_id = Uuid::new_v4();

        let result = db.get_federation(&non_existent_id).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_federation_with_services() {
        let (db, _temp_dir) = create_test_db().await;
        let original = create_test_federation();
        let federation_id = original.id;

        db.create_federation(&original).await.unwrap();

        let retrieved = db.get_federation(&federation_id).await.unwrap().unwrap();

        assert_eq!(retrieved.services.len(), 2);
        assert_eq!(retrieved.services[0].name, "auth");
        assert_eq!(retrieved.services[0].base_path, "/auth");
        assert_eq!(retrieved.services[0].reality_level, ServiceRealityLevel::Real);
        assert_eq!(retrieved.services[1].name, "payments");
        assert_eq!(retrieved.services[1].reality_level, ServiceRealityLevel::MockV3);
    }

    #[tokio::test]
    async fn test_get_federation_preserves_service_config() {
        let (db, _temp_dir) = create_test_db().await;
        let original = create_test_federation();
        let federation_id = original.id;

        db.create_federation(&original).await.unwrap();

        let retrieved = db.get_federation(&federation_id).await.unwrap().unwrap();

        // Check that config was preserved
        assert_eq!(retrieved.services[0].config.get("timeout"), Some(&serde_json::json!(5000)));
    }

    #[tokio::test]
    async fn test_get_federation_preserves_service_dependencies() {
        let (db, _temp_dir) = create_test_db().await;
        let original = create_test_federation();
        let federation_id = original.id;

        db.create_federation(&original).await.unwrap();

        let retrieved = db.get_federation(&federation_id).await.unwrap().unwrap();

        // Check that dependencies were preserved
        assert_eq!(retrieved.services[0].dependencies, vec!["database".to_string()]);
    }

    #[tokio::test]
    async fn test_list_federations() {
        let (db, _temp_dir) = create_test_db().await;
        let org_id = Uuid::new_v4();

        // Create multiple federations for the same org
        for i in 0..3 {
            let mut federation = create_test_federation();
            federation.org_id = org_id;
            federation.name = format!("federation-{}", i);
            db.create_federation(&federation).await.unwrap();
        }

        let federations = db.list_federations(&org_id).await.unwrap();
        assert_eq!(federations.len(), 3);
    }

    #[tokio::test]
    async fn test_list_federations_empty() {
        let (db, _temp_dir) = create_test_db().await;
        let org_id = Uuid::new_v4();

        let federations = db.list_federations(&org_id).await.unwrap();
        assert!(federations.is_empty());
    }

    #[tokio::test]
    async fn test_list_federations_filters_by_org() {
        let (db, _temp_dir) = create_test_db().await;
        let org_id1 = Uuid::new_v4();
        let org_id2 = Uuid::new_v4();

        // Create federation for org1
        let mut federation1 = create_test_federation();
        federation1.org_id = org_id1;
        db.create_federation(&federation1).await.unwrap();

        // Create federation for org2
        let mut federation2 = create_test_federation();
        federation2.org_id = org_id2;
        db.create_federation(&federation2).await.unwrap();

        let org1_feds = db.list_federations(&org_id1).await.unwrap();
        assert_eq!(org1_feds.len(), 1);
        assert_eq!(org1_feds[0].id, federation1.id);

        let org2_feds = db.list_federations(&org_id2).await.unwrap();
        assert_eq!(org2_feds.len(), 1);
        assert_eq!(org2_feds[0].id, federation2.id);
    }

    #[tokio::test]
    async fn test_list_federations_with_services() {
        let (db, _temp_dir) = create_test_db().await;
        let org_id = Uuid::new_v4();

        let mut federation = create_test_federation();
        federation.org_id = org_id;
        db.create_federation(&federation).await.unwrap();

        let federations = db.list_federations(&org_id).await.unwrap();
        assert_eq!(federations.len(), 1);
        assert_eq!(federations[0].services.len(), 2);
    }

    #[tokio::test]
    async fn test_update_federation() {
        let (db, _temp_dir) = create_test_db().await;
        let mut federation = create_test_federation();
        let federation_id = federation.id;

        // Create initial federation
        db.create_federation(&federation).await.unwrap();

        // Update the federation
        federation.name = "updated-federation".to_string();
        federation.description = "Updated description".to_string();

        db.update_federation(&federation).await.unwrap();

        // Verify the update
        let updated = db.get_federation(&federation_id).await.unwrap().unwrap();
        assert_eq!(updated.name, "updated-federation");
        assert_eq!(updated.description, "Updated description");
    }

    #[tokio::test]
    async fn test_update_federation_updates_services() {
        let (db, _temp_dir) = create_test_db().await;
        let mut federation = create_test_federation();
        let federation_id = federation.id;

        // Create initial federation
        db.create_federation(&federation).await.unwrap();

        // Add a new service
        federation.services.push(ServiceBoundary::new(
            "inventory".to_string(),
            Uuid::new_v4(),
            "/inventory".to_string(),
            ServiceRealityLevel::Blended,
        ));

        db.update_federation(&federation).await.unwrap();

        // Verify services were updated
        let updated = db.get_federation(&federation_id).await.unwrap().unwrap();
        assert_eq!(updated.services.len(), 3);
        assert!(updated.services.iter().any(|s| s.name == "inventory"));
    }

    #[tokio::test]
    async fn test_update_federation_removes_old_services() {
        let (db, _temp_dir) = create_test_db().await;
        let mut federation = create_test_federation();
        let federation_id = federation.id;

        // Create initial federation
        db.create_federation(&federation).await.unwrap();

        // Remove all services
        federation.services.clear();

        db.update_federation(&federation).await.unwrap();

        // Verify services were removed
        let updated = db.get_federation(&federation_id).await.unwrap().unwrap();
        assert!(updated.services.is_empty());
    }

    #[tokio::test]
    async fn test_delete_federation() {
        let (db, _temp_dir) = create_test_db().await;
        let federation = create_test_federation();
        let federation_id = federation.id;

        // Create federation
        db.create_federation(&federation).await.unwrap();

        // Delete it
        db.delete_federation(&federation_id).await.unwrap();

        // Verify it's gone
        let result = db.get_federation(&federation_id).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_delete_federation_cascades_services() {
        let (db, _temp_dir) = create_test_db().await;
        let federation = create_test_federation();
        let federation_id = federation.id;

        // Create federation
        db.create_federation(&federation).await.unwrap();

        // Delete it
        db.delete_federation(&federation_id).await.unwrap();

        // Verify services were also deleted
        let rows = sqlx::query("SELECT COUNT(*) FROM federation_services WHERE federation_id = ?1")
            .bind(federation_id.to_string())
            .fetch_one(&db.pool)
            .await
            .unwrap();

        let count: i64 = rows.get(0);
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_create_federation_service_internal() {
        let (db, _temp_dir) = create_test_db().await;
        let federation = create_test_federation();
        let federation_id = federation.id.to_string();

        // First create the federation without services
        sqlx::query(
            r"
            INSERT INTO federations (id, name, org_id, description, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ",
        )
        .bind(&federation_id)
        .bind("test")
        .bind(federation.org_id.to_string())
        .bind("")
        .bind(Utc::now().timestamp())
        .bind(Utc::now().timestamp())
        .execute(&db.pool)
        .await
        .unwrap();

        // Now test creating a service
        let service = ServiceBoundary::new(
            "test-service".to_string(),
            Uuid::new_v4(),
            "/test".to_string(),
            ServiceRealityLevel::Real,
        );

        let result = db.create_federation_service(&federation_id, &service).await;
        assert!(result.is_ok());

        // Verify it was created
        let rows = sqlx::query("SELECT COUNT(*) FROM federation_services WHERE federation_id = ?1")
            .bind(&federation_id)
            .fetch_one(&db.pool)
            .await
            .unwrap();

        let count: i64 = rows.get(0);
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_get_federation_services_internal() {
        let (db, _temp_dir) = create_test_db().await;
        let federation = create_test_federation();
        let federation_id = federation.id;

        db.create_federation(&federation).await.unwrap();

        let services = db.get_federation_services(&federation_id.to_string()).await.unwrap();
        assert_eq!(services.len(), 2);

        // Services should be ordered by base_path
        assert_eq!(services[0].base_path, "/auth");
        assert_eq!(services[1].base_path, "/payments");
    }

    #[tokio::test]
    async fn test_get_federation_services_empty() {
        let (db, _temp_dir) = create_test_db().await;
        let federation_id = Uuid::new_v4().to_string();

        let services = db.get_federation_services(&federation_id).await.unwrap();
        assert!(services.is_empty());
    }

    #[tokio::test]
    async fn test_federation_timestamps() {
        let (db, _temp_dir) = create_test_db().await;
        let federation = create_test_federation();
        let federation_id = federation.id;

        db.create_federation(&federation).await.unwrap();

        let retrieved = db.get_federation(&federation_id).await.unwrap().unwrap();

        // Timestamps should be set
        assert!(retrieved.created_at.timestamp() > 0);
        assert!(retrieved.updated_at.timestamp() > 0);
    }

    #[tokio::test]
    async fn test_service_reality_level_persistence() {
        let (db, _temp_dir) = create_test_db().await;
        let org_id = Uuid::new_v4();

        // Test all reality levels
        let reality_levels = vec![
            ServiceRealityLevel::Real,
            ServiceRealityLevel::MockV3,
            ServiceRealityLevel::Blended,
            ServiceRealityLevel::ChaosDriven,
        ];

        for (i, level) in reality_levels.iter().enumerate() {
            let service = ServiceBoundary::new(
                format!("service-{}", i),
                Uuid::new_v4(),
                format!("/service{}", i),
                *level,
            );

            let federation = Federation {
                id: Uuid::new_v4(),
                name: format!("fed-{}", i),
                description: String::new(),
                org_id,
                services: vec![service],
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };

            db.create_federation(&federation).await.unwrap();

            let retrieved = db.get_federation(&federation.id).await.unwrap().unwrap();
            assert_eq!(retrieved.services[0].reality_level, *level);
        }
    }

    #[tokio::test]
    async fn test_multiple_federations_same_org() {
        let (db, _temp_dir) = create_test_db().await;
        let org_id = Uuid::new_v4();

        // Create 5 federations
        for i in 0..5 {
            let mut federation = create_test_federation();
            federation.org_id = org_id;
            federation.name = format!("federation-{}", i);
            db.create_federation(&federation).await.unwrap();
        }

        let federations = db.list_federations(&org_id).await.unwrap();
        assert_eq!(federations.len(), 5);

        // Should be ordered by created_at DESC
        for i in 0..4 {
            assert!(federations[i].created_at >= federations[i + 1].created_at);
        }
    }

    #[tokio::test]
    async fn test_complex_service_config_persistence() {
        let (db, _temp_dir) = create_test_db().await;
        let org_id = Uuid::new_v4();

        let mut service = ServiceBoundary::new(
            "complex".to_string(),
            Uuid::new_v4(),
            "/complex".to_string(),
            ServiceRealityLevel::Blended,
        );

        // Add complex config
        service.config.insert("timeout".to_string(), serde_json::json!(5000));
        service.config.insert("retries".to_string(), serde_json::json!(3));
        service.config.insert(
            "features".to_string(),
            serde_json::json!({
                "auth": true,
                "metrics": false,
                "tracing": true
            }),
        );
        service.config.insert(
            "endpoints".to_string(),
            serde_json::json!(["/api/users", "/api/posts", "/api/comments"]),
        );

        // Add dependencies
        service.dependencies = vec![
            "auth".to_string(),
            "database".to_string(),
            "cache".to_string(),
        ];

        let federation = Federation {
            id: Uuid::new_v4(),
            name: "complex-test".to_string(),
            description: String::new(),
            org_id,
            services: vec![service],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        db.create_federation(&federation).await.unwrap();

        let retrieved = db.get_federation(&federation.id).await.unwrap().unwrap();
        let service = &retrieved.services[0];

        // Verify complex config was preserved
        assert_eq!(service.config.get("timeout"), Some(&serde_json::json!(5000)));
        assert_eq!(service.config.get("retries"), Some(&serde_json::json!(3)));
        assert!(service.config.contains_key("features"));
        assert!(service.config.contains_key("endpoints"));

        // Verify dependencies
        assert_eq!(service.dependencies.len(), 3);
        assert!(service.dependencies.contains(&"auth".to_string()));
        assert!(service.dependencies.contains(&"database".to_string()));
        assert!(service.dependencies.contains(&"cache".to_string()));
    }
}
