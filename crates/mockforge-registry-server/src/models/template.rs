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
        sqlx::query_as::<_, Self>("SELECT * FROM templates WHERE name = $1 AND version = $2")
            .bind(name)
            .bind(version)
            .fetch_optional(pool)
            .await
    }

    /// Build WHERE clause for search queries (using parameterized queries for security)
    fn build_search_where_clause(
        query: Option<&str>,
        category: Option<&str>,
        tags: &[String],
        org_id: Option<Uuid>,
    ) -> (String, Vec<String>) {
        let mut where_parts = Vec::new();
        let mut param_placeholders = Vec::new();
        let mut param_index = 1;

        // Published filter
        where_parts.push("published = TRUE".to_string());

        // Org filtering
        if let Some(_org) = org_id {
            param_placeholders.push(format!("${}", param_index));
            where_parts.push(format!("(org_id = ${} OR org_id IS NULL)", param_index));
            param_index += 1;
        } else {
            // Public templates only if no org context
            where_parts.push("org_id IS NULL".to_string());
        }

        // Category filter
        if let Some(_cat) = category {
            param_placeholders.push(format!("${}", param_index));
            where_parts.push(format!("category = ${}", param_index));
            param_index += 1;
        }

        // Tags filter
        if !tags.is_empty() {
            param_placeholders.push(format!("${}", param_index));
            where_parts.push(format!("tags && ${}::text[]", param_index));
            param_index += 1;
        }

        // Full-text search
        if let Some(_q) = query {
            param_placeholders.push(format!("${}", param_index));
            where_parts.push(format!(
                "to_tsvector('english', name || ' ' || COALESCE(description, '')) @@ plainto_tsquery('english', ${})",
                param_index
            ));
            param_index += 1;
        }

        let where_clause = format!("WHERE {}", where_parts.join(" AND "));
        (where_clause, param_placeholders)
    }

    /// Count templates matching search criteria
    pub async fn count_search(
        pool: &sqlx::PgPool,
        query: Option<&str>,
        category: Option<&str>,
        tags: &[String],
        org_id: Option<Uuid>,
    ) -> sqlx::Result<i64> {
        let (where_clause, _) = Self::build_search_where_clause(query, category, tags, org_id);
        let sql = format!("SELECT COUNT(*) FROM templates {}", where_clause);

        let mut query_builder = sqlx::query_as::<_, (i64,)>(&sql);

        // Bind parameters in order
        if let Some(org) = org_id {
            query_builder = query_builder.bind(org);
        }
        if let Some(cat) = category {
            query_builder = query_builder.bind(cat);
        }
        if !tags.is_empty() {
            query_builder = query_builder.bind(tags);
        }
        if let Some(q) = query {
            query_builder = query_builder.bind(q);
        }

        let result = query_builder.fetch_one(pool).await?;
        Ok(result.0)
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
        let (where_clause, _) = Self::build_search_where_clause(query, category, tags, org_id);

        // Calculate parameter offset for LIMIT/OFFSET
        let mut param_count = 1;
        if org_id.is_some() {
            param_count += 1;
        }
        if category.is_some() {
            param_count += 1;
        }
        if !tags.is_empty() {
            param_count += 1;
        }
        if query.is_some() {
            param_count += 1;
        }

        let sql = format!(
            "SELECT * FROM templates {} ORDER BY created_at DESC LIMIT ${} OFFSET ${}",
            where_clause,
            param_count,
            param_count + 1
        );

        let mut query_builder = sqlx::query_as::<_, Self>(&sql);

        // Bind parameters in order
        if let Some(org) = org_id {
            query_builder = query_builder.bind(org);
        }
        if let Some(cat) = category {
            query_builder = query_builder.bind(cat);
        }
        if !tags.is_empty() {
            query_builder = query_builder.bind(tags);
        }
        if let Some(q) = query {
            query_builder = query_builder.bind(q);
        }
        query_builder = query_builder.bind(limit).bind(offset);

        query_builder.fetch_all(pool).await
    }

    /// Find templates by organization
    pub async fn find_by_org(pool: &sqlx::PgPool, org_id: Uuid) -> sqlx::Result<Vec<Self>> {
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
