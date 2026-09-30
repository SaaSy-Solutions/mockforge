//! Project model (scoped to organizations)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Project visibility
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum ProjectVisibility {
    #[default]
    Private,
    Public,
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

#[cfg(feature = "postgres")]
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
        executor: impl sqlx::PgExecutor<'_>,
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
        .fetch_one(executor)
        .await
    }

    /// Find project by ID
    pub async fn find_by_id(
        executor: impl sqlx::PgExecutor<'_>,
        id: Uuid,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM projects WHERE id = $1")
            .bind(id)
            .fetch_optional(executor)
            .await
    }

    /// Find project by org and slug
    pub async fn find_by_slug(
        executor: impl sqlx::PgExecutor<'_>,
        org_id: Uuid,
        slug: &str,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM projects WHERE org_id = $1 AND slug = $2")
            .bind(org_id)
            .bind(slug)
            .fetch_optional(executor)
            .await
    }

    /// Get all projects for an organization
    pub async fn find_by_org(
        executor: impl sqlx::PgExecutor<'_>,
        org_id: Uuid,
    ) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM projects WHERE org_id = $1 ORDER BY created_at DESC",
        )
        .bind(org_id)
        .fetch_all(executor)
        .await
    }

    /// Update project
    pub async fn update(
        executor: impl sqlx::PgExecutor<'_>,
        id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        visibility: Option<ProjectVisibility>,
        default_env: Option<&str>,
    ) -> sqlx::Result<()> {
        let mut updates = Vec::new();
        let mut param_count = 1;

        if name.is_some() {
            updates.push(format!("name = ${}", param_count));
            param_count += 1;
        }
        if description.is_some() {
            updates.push(format!("description = ${}", param_count));
            param_count += 1;
        }
        if visibility.is_some() {
            updates.push(format!("visibility = ${}", param_count));
            param_count += 1;
        }
        if default_env.is_some() {
            updates.push(format!("default_env = ${}", param_count));
            param_count += 1;
        }

        if updates.is_empty() {
            return Ok(());
        }

        updates.push("updated_at = NOW()".to_string());
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

        query.execute(executor).await?;
        Ok(())
    }

    /// Delete project
    pub async fn delete(executor: impl sqlx::PgExecutor<'_>, id: Uuid) -> sqlx::Result<()> {
        sqlx::query("DELETE FROM projects WHERE id = $1")
            .bind(id)
            .execute(executor)
            .await?;

        Ok(())
    }
}

impl std::fmt::Display for ProjectVisibility {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectVisibility::Private => write!(f, "private"),
            ProjectVisibility::Public => write!(f, "public"),
        }
    }
}
