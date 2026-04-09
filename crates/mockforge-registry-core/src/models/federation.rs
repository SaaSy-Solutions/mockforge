//! Federation model for composing multiple workspaces into virtual systems

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Federation {
    pub id: Uuid,
    pub org_id: Uuid,
    pub name: String,
    pub description: String,
    pub services: serde_json::Value,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg(feature = "postgres")]
impl Federation {
    /// Create a new federation
    pub async fn create(
        pool: &sqlx::PgPool,
        org_id: Uuid,
        created_by: Uuid,
        name: &str,
        description: &str,
        services: &serde_json::Value,
    ) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO federations (org_id, name, description, services, created_by)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(name)
        .bind(description)
        .bind(services)
        .bind(created_by)
        .fetch_one(pool)
        .await
    }

    /// Find a federation by ID
    pub async fn find_by_id(pool: &sqlx::PgPool, id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM federations WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    /// Find all federations for an organization
    pub async fn find_by_org(pool: &sqlx::PgPool, org_id: Uuid) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM federations WHERE org_id = $1 ORDER BY created_at DESC",
        )
        .bind(org_id)
        .fetch_all(pool)
        .await
    }

    /// Update a federation
    pub async fn update(
        pool: &sqlx::PgPool,
        id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        services: Option<&serde_json::Value>,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE federations
            SET
                name = COALESCE($2, name),
                description = COALESCE($3, description),
                services = COALESCE($4, services),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(description)
        .bind(services)
        .fetch_optional(pool)
        .await
    }

    /// Delete a federation
    pub async fn delete(pool: &sqlx::PgPool, id: Uuid) -> sqlx::Result<()> {
        sqlx::query("DELETE FROM federations WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
