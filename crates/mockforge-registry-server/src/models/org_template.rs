//! Organization template model
//!
//! Org-level templates allow organization admins to define standard blueprints
//! and security baseline configs for workspace creation, enabling teams to
//! start from templates rather than scratch.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Organization template model
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct OrgTemplate {
    /// Unique identifier
    pub id: Uuid,
    /// Organization ID this template belongs to
    pub org_id: Uuid,
    /// Template name
    pub name: String,
    /// Template description
    pub description: Option<String>,
    /// Blueprint configuration (personas, reality defaults, flows, etc.)
    pub blueprint_config: serde_json::Value,
    /// Security baseline configuration (RBAC defaults, validation modes, etc.)
    pub security_baseline: serde_json::Value,
    /// User who created this template
    pub created_by: Uuid,
    /// Whether this is the default template for the org
    pub is_default: bool,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl OrgTemplate {
    /// Create a new organization template
    pub async fn create(
        pool: &sqlx::PgPool,
        org_id: Uuid,
        name: &str,
        description: Option<&str>,
        blueprint_config: Option<serde_json::Value>,
        security_baseline: Option<serde_json::Value>,
        created_by: Uuid,
        is_default: bool,
    ) -> sqlx::Result<Self> {
        // If this is being set as default, unset other defaults for this org
        if is_default {
            sqlx::query("UPDATE org_templates SET is_default = FALSE WHERE org_id = $1")
                .bind(org_id)
                .execute(pool)
                .await?;
        }

        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO org_templates (
                org_id, name, description, blueprint_config, security_baseline,
                created_by, is_default
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(name)
        .bind(description)
        .bind(blueprint_config.unwrap_or_else(|| serde_json::json!({})))
        .bind(security_baseline.unwrap_or_else(|| serde_json::json!({})))
        .bind(created_by)
        .bind(is_default)
        .fetch_one(pool)
        .await
    }

    /// Find by ID
    pub async fn find_by_id(pool: &sqlx::PgPool, id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM org_templates WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    /// List all templates for an organization
    pub async fn list_by_org(pool: &sqlx::PgPool, org_id: Uuid) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM org_templates WHERE org_id = $1 ORDER BY is_default DESC, name",
        )
        .bind(org_id)
        .fetch_all(pool)
        .await
    }

    /// Get the default template for an organization
    pub async fn get_default(pool: &sqlx::PgPool, org_id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM org_templates WHERE org_id = $1 AND is_default = TRUE LIMIT 1",
        )
        .bind(org_id)
        .fetch_optional(pool)
        .await
    }

    /// Update template
    pub async fn update(
        &self,
        pool: &sqlx::PgPool,
        name: Option<&str>,
        description: Option<&str>,
        blueprint_config: Option<serde_json::Value>,
        security_baseline: Option<serde_json::Value>,
        is_default: Option<bool>,
    ) -> sqlx::Result<Self> {
        // If setting as default, unset other defaults
        if is_default == Some(true) {
            sqlx::query(
                "UPDATE org_templates SET is_default = FALSE WHERE org_id = $1 AND id != $2",
            )
            .bind(self.org_id)
            .bind(self.id)
            .execute(pool)
            .await?;
        }

        sqlx::query_as::<_, Self>(
            r#"
            UPDATE org_templates
            SET
                name = COALESCE($1, name),
                description = COALESCE($2, description),
                blueprint_config = COALESCE($3, blueprint_config),
                security_baseline = COALESCE($4, security_baseline),
                is_default = COALESCE($5, is_default),
                updated_at = NOW()
            WHERE id = $6
            RETURNING *
            "#,
        )
        .bind(name)
        .bind(description)
        .bind(blueprint_config)
        .bind(security_baseline)
        .bind(is_default)
        .bind(self.id)
        .fetch_one(pool)
        .await
    }

    /// Delete template
    pub async fn delete(pool: &sqlx::PgPool, id: Uuid) -> sqlx::Result<()> {
        sqlx::query("DELETE FROM org_templates WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Set as default template for the organization
    pub async fn set_as_default(pool: &sqlx::PgPool, id: Uuid, org_id: Uuid) -> sqlx::Result<Self> {
        // Unset other defaults
        sqlx::query("UPDATE org_templates SET is_default = FALSE WHERE org_id = $1")
            .bind(org_id)
            .execute(pool)
            .await?;

        // Set this one as default
        sqlx::query_as::<_, Self>(
            "UPDATE org_templates SET is_default = TRUE, updated_at = NOW() WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .fetch_one(pool)
        .await
    }
}
