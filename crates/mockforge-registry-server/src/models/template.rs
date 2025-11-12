//! Template marketplace models
//!
//! Handles orchestration templates for chaos testing

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Template category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TemplateCategory {
    NetworkChaos,
    ServiceFailure,
    LoadTesting,
    ResilienceTesting,
    SecurityTesting,
    DataCorruption,
    MultiProtocol,
    CustomScenario,
}

impl TemplateCategory {
    pub fn to_string(&self) -> String {
        match self {
            TemplateCategory::NetworkChaos => "network-chaos".to_string(),
            TemplateCategory::ServiceFailure => "service-failure".to_string(),
            TemplateCategory::LoadTesting => "load-testing".to_string(),
            TemplateCategory::ResilienceTesting => "resilience-testing".to_string(),
            TemplateCategory::SecurityTesting => "security-testing".to_string(),
            TemplateCategory::DataCorruption => "data-corruption".to_string(),
            TemplateCategory::MultiProtocol => "multi-protocol".to_string(),
            TemplateCategory::CustomScenario => "custom-scenario".to_string(),
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "network-chaos" => Some(TemplateCategory::NetworkChaos),
            "service-failure" => Some(TemplateCategory::ServiceFailure),
            "load-testing" => Some(TemplateCategory::LoadTesting),
            "resilience-testing" => Some(TemplateCategory::ResilienceTesting),
            "security-testing" => Some(TemplateCategory::SecurityTesting),
            "data-corruption" => Some(TemplateCategory::DataCorruption),
            "multi-protocol" => Some(TemplateCategory::MultiProtocol),
            "custom-scenario" => Some(TemplateCategory::CustomScenario),
            _ => None,
        }
    }
}

/// Template model
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Template {
    pub id: Uuid,
    pub org_id: Option<Uuid>,
    pub name: String,
    pub slug: String,
    pub description: String,
    pub author_id: Uuid,
    pub version: String,
    pub category: String, // Stored as VARCHAR, converted via methods
    pub tags: Vec<String>,
    pub content_json: serde_json::Value,
    pub readme: Option<String>,
    pub example_usage: Option<String>,
    pub requirements: Vec<String>,
    pub compatibility_json: serde_json::Value,
    pub stats_json: serde_json::Value,
    pub published: bool,
    pub verified_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Template {
    /// Get category as enum
    pub fn category(&self) -> TemplateCategory {
        TemplateCategory::from_str(&self.category).unwrap_or(TemplateCategory::CustomScenario)
    }

    /// Create a new template
    pub async fn create(
        pool: &sqlx::PgPool,
        org_id: Option<Uuid>,
        name: &str,
        slug: &str,
        description: &str,
        author_id: Uuid,
        version: &str,
        category: TemplateCategory,
        content_json: serde_json::Value,
    ) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO templates (
                org_id, name, slug, description, author_id, version,
                category, content_json, published
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, FALSE)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(name)
        .bind(slug)
        .bind(description)
        .bind(author_id)
        .bind(version)
        .bind(category.to_string())
        .bind(content_json)
        .fetch_one(pool)
        .await
    }

    /// Find by ID
    pub async fn find_by_id(pool: &sqlx::PgPool, id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM templates WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    /// Find by name and version
    pub async fn find_by_name_version(
        pool: &sqlx::PgPool,
        name: &str,
        version: &str,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM templates WHERE name = $1 AND version = $2",
        )
        .bind(name)
        .bind(version)
        .fetch_optional(pool)
        .await
    }

    /// Search templates
    pub async fn search(
        pool: &sqlx::PgPool,
        query: Option<&str>,
        category: Option<&str>,
        tags: &[String],
        org_id: Option<Uuid>,
        limit: i64,
        offset: i64,
    ) -> sqlx::Result<Vec<Self>> {
        let mut sql = String::from(
            "SELECT * FROM templates WHERE published = TRUE",
        );

        if let Some(org) = org_id {
            sql.push_str(&format!(" AND (org_id = '{}' OR org_id IS NULL)", org));
        } else {
            // Public templates only if no org context
            sql.push_str(" AND org_id IS NULL");
        }

        if let Some(cat) = category {
            sql.push_str(&format!(" AND category = '{}'", cat));
        }

        if !tags.is_empty() {
            sql.push_str(&format!(" AND tags && ARRAY[{}]",
                tags.iter().map(|t| format!("'{}'", t)).collect::<Vec<_>>().join(",")));
        }

        if let Some(q) = query {
            sql.push_str(&format!(
                " AND (to_tsvector('english', name || ' ' || COALESCE(description, '')) @@ plainto_tsquery('english', '{}'))",
                q.replace("'", "''")
            ));
        }

        sql.push_str(" ORDER BY created_at DESC LIMIT $1 OFFSET $2");

        sqlx::query_as::<_, Self>(&sql)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await
    }

    /// Find templates by organization
    pub async fn find_by_org(
        pool: &sqlx::PgPool,
        org_id: Uuid,
    ) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM templates WHERE org_id = $1 ORDER BY created_at DESC",
        )
        .bind(org_id)
        .fetch_all(pool)
        .await
    }
}

/// Template version
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct TemplateVersion {
    pub id: Uuid,
    pub template_id: Uuid,
    pub version: String,
    pub content_json: serde_json::Value,
    pub download_url: Option<String>,
    pub checksum: Option<String>,
    pub file_size: i64,
    pub yanked: bool,
    pub published_at: DateTime<Utc>,
}

impl TemplateVersion {
    /// Create a new version
    pub async fn create(
        pool: &sqlx::PgPool,
        template_id: Uuid,
        version: &str,
        content_json: serde_json::Value,
        download_url: Option<&str>,
        checksum: Option<&str>,
        file_size: i64,
    ) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO template_versions (
                template_id, version, content_json, download_url, checksum, file_size
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(template_id)
        .bind(version)
        .bind(content_json)
        .bind(download_url)
        .bind(checksum)
        .bind(file_size)
        .fetch_one(pool)
        .await
    }

    /// Find by template and version
    pub async fn find(
        pool: &sqlx::PgPool,
        template_id: Uuid,
        version: &str,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM template_versions WHERE template_id = $1 AND version = $2",
        )
        .bind(template_id)
        .bind(version)
        .fetch_optional(pool)
        .await
    }

    /// Get all versions for a template
    pub async fn get_by_template(
        pool: &sqlx::PgPool,
        template_id: Uuid,
    ) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM template_versions WHERE template_id = $1 ORDER BY published_at DESC",
        )
        .bind(template_id)
        .fetch_all(pool)
        .await
    }
}
