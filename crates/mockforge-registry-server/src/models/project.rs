//! Project model (scoped to organizations)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Project visibility
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectVisibility {
    Private,
    Public,
}

impl Default for ProjectVisibility {
    fn default() -> Self {
        ProjectVisibility::Private
    }
}

/// Project model
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Project {
    pub id: Uuid,
    pub org_id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub visibility: String, // Stored as VARCHAR, converted via methods
    pub default_env: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Project {
    /// Get visibility as enum
    pub fn visibility(&self) -> ProjectVisibility {
        match self.visibility.as_str() {
            "public" => ProjectVisibility::Public,
            "private" => ProjectVisibility::Private,
            _ => ProjectVisibility::Private,
        }
    }

    /// Create a new project
    pub async fn create(
        pool: &sqlx::PgPool,
        org_id: Uuid,
        slug: &str,
        name: &str,
        description: Option<&str>,
        visibility: ProjectVisibility,
        default_env: &str,
    ) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO projects (org_id, slug, name, description, visibility, default_env)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(slug)
        .bind(name)
        .bind(description)
        .bind(visibility.to_string())
        .bind(default_env)
        .fetch_one(pool)
        .await
    }

    /// Find project by ID
    pub async fn find_by_id(pool: &sqlx::PgPool, id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM projects WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    /// Find project by org and slug
    pub async fn find_by_slug(
        pool: &sqlx::PgPool,
        org_id: Uuid,
        slug: &str,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM projects WHERE org_id = $1 AND slug = $2")
            .bind(org_id)
            .bind(slug)
            .fetch_optional(pool)
            .await
    }

    /// Get all projects for an organization
    pub async fn find_by_org(pool: &sqlx::PgPool, org_id: Uuid) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM projects WHERE org_id = $1 ORDER BY created_at DESC",
        )
        .bind(org_id)
        .fetch_all(pool)
        .await
    }

    /// Update project
    pub async fn update(
        pool: &sqlx::PgPool,
        id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        visibility: Option<ProjectVisibility>,
        default_env: Option<&str>,
    ) -> sqlx::Result<()> {
        let mut updates = Vec::new();
        let mut param_count = 1;

        if let Some(_n) = name {
            updates.push(format!("name = ${}", param_count));
            param_count += 1;
        }
        if let Some(_d) = description {
            updates.push(format!("description = ${}", param_count));
            param_count += 1;
        }
        if let Some(_v) = visibility {
            updates.push(format!("visibility = ${}", param_count));
            param_count += 1;
        }
        if let Some(_e) = default_env {
            updates.push(format!("default_env = ${}", param_count));
            param_count += 1;
        }

        if updates.is_empty() {
            return Ok(());
        }

        updates.push(format!("updated_at = NOW()"));
        updates.push(format!("id = ${}", param_count));

        let sql = format!("UPDATE projects SET {} WHERE id = ${}", updates.join(", "), param_count);

        let mut query = sqlx::query(&sql);
        if let Some(n) = name {
            query = query.bind(n);
        }
        if let Some(d) = description {
            query = query.bind(d);
        }
        if let Some(v) = visibility {
            query = query.bind(v.to_string());
        }
        if let Some(e) = default_env {
            query = query.bind(e);
        }
        query = query.bind(id);

        query.execute(pool).await?;
        Ok(())
    }

    /// Delete project
    pub async fn delete(pool: &sqlx::PgPool, id: Uuid) -> sqlx::Result<()> {
        sqlx::query("DELETE FROM projects WHERE id = $1").bind(id).execute(pool).await?;

        Ok(())
    }
}

impl ProjectVisibility {
    pub fn to_string(&self) -> String {
        match self {
            ProjectVisibility::Private => "private".to_string(),
            ProjectVisibility::Public => "public".to_string(),
        }
    }
}
