//! Scenario marketplace models
//!
//! Handles data scenarios for mock systems

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Scenario model
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Scenario {
    pub id: Uuid,
    pub org_id: Option<Uuid>,
    pub name: String,
    pub slug: String,
    pub description: String,
    pub author_id: Uuid,
    pub current_version: String,
    pub category: String,
    pub tags: Vec<String>,
    pub license: String,
    pub repository: Option<String>,
    pub homepage: Option<String>,
    pub manifest_json: serde_json::Value,
    pub downloads_total: i64,
    pub rating_avg: rust_decimal::Decimal,
    pub rating_count: i32,
    pub verified_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Scenario {
    /// Create a new scenario
    pub async fn create(
        pool: &sqlx::PgPool,
        org_id: Option<Uuid>,
        name: &str,
        slug: &str,
        description: &str,
        author_id: Uuid,
        current_version: &str,
        category: &str,
        license: &str,
        manifest_json: serde_json::Value,
    ) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO scenarios (
                org_id, name, slug, description, author_id, current_version,
                category, license, manifest_json
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(name)
        .bind(slug)
        .bind(description)
        .bind(author_id)
        .bind(current_version)
        .bind(category)
        .bind(license)
        .bind(manifest_json)
        .fetch_one(pool)
        .await
    }

    /// Find by ID
    pub async fn find_by_id(pool: &sqlx::PgPool, id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM scenarios WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    /// Find by name
    pub async fn find_by_name(pool: &sqlx::PgPool, name: &str) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM scenarios WHERE name = $1")
            .bind(name)
            .fetch_optional(pool)
            .await
    }

    /// Build WHERE clause for search queries
    fn build_search_where_clause(
        query: Option<&str>,
        category: Option<&str>,
        tags: &[String],
        org_id: Option<Uuid>,
    ) -> String {
        let mut where_clause = String::from("WHERE 1=1");

        if let Some(org) = org_id {
            where_clause.push_str(&format!(" AND (org_id = '{}' OR org_id IS NULL)", org));
        } else {
            // Public scenarios only if no org context
            where_clause.push_str(" AND org_id IS NULL");
        }

        if let Some(cat) = category {
            where_clause.push_str(&format!(" AND category = '{}'", cat));
        }

        if !tags.is_empty() {
            where_clause.push_str(&format!(" AND tags && ARRAY[{}]",
                tags.iter().map(|t| format!("'{}'", t.replace("'", "''"))).collect::<Vec<_>>().join(",")));
        }

        if let Some(q) = query {
            where_clause.push_str(&format!(
                " AND (to_tsvector('english', name || ' ' || COALESCE(description, '')) @@ plainto_tsquery('english', '{}'))",
                q.replace("'", "''")
            ));
        }

        where_clause
    }

    /// Count scenarios matching search criteria
    pub async fn count_search(
        pool: &sqlx::PgPool,
        query: Option<&str>,
        category: Option<&str>,
        tags: &[String],
        org_id: Option<Uuid>,
    ) -> sqlx::Result<i64> {
        let where_clause = Self::build_search_where_clause(query, category, tags, org_id);
        let sql = format!("SELECT COUNT(*) FROM scenarios {}", where_clause);

        let result: (i64,) = sqlx::query_as(&sql)
            .fetch_one(pool)
            .await?;

        Ok(result.0)
    }

    /// Search scenarios
    pub async fn search(
        pool: &sqlx::PgPool,
        query: Option<&str>,
        category: Option<&str>,
        tags: &[String],
        org_id: Option<Uuid>,
        sort: &str,
        limit: i64,
        offset: i64,
    ) -> sqlx::Result<Vec<Self>> {
        let where_clause = Self::build_search_where_clause(query, category, tags, org_id);

        // Sort
        let order_by = match sort {
            "downloads" => "ORDER BY downloads_total DESC",
            "rating" => "ORDER BY rating_avg DESC",
            "recent" => "ORDER BY created_at DESC",
            "name" => "ORDER BY name ASC",
            _ => "ORDER BY downloads_total DESC",
        };

        let sql = format!(
            "SELECT * FROM scenarios {} {} LIMIT $1 OFFSET $2",
            where_clause, order_by
        );

        sqlx::query_as::<_, Self>(&sql)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await
    }

    /// Find scenarios by organization
    pub async fn find_by_org(
        pool: &sqlx::PgPool,
        org_id: Uuid,
    ) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM scenarios WHERE org_id = $1 ORDER BY created_at DESC",
        )
        .bind(org_id)
        .fetch_all(pool)
        .await
    }
}

/// Scenario version
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ScenarioVersion {
    pub id: Uuid,
    pub scenario_id: Uuid,
    pub version: String,
    pub manifest_json: serde_json::Value,
    pub download_url: String,
    pub checksum: String,
    pub file_size: i64,
    pub min_mockforge_version: Option<String>,
    pub yanked: bool,
    pub downloads: i32,
    pub published_at: DateTime<Utc>,
}

impl ScenarioVersion {
    /// Create a new version
    pub async fn create(
        pool: &sqlx::PgPool,
        scenario_id: Uuid,
        version: &str,
        manifest_json: serde_json::Value,
        download_url: &str,
        checksum: &str,
        file_size: i64,
        min_mockforge_version: Option<&str>,
    ) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO scenario_versions (
                scenario_id, version, manifest_json, download_url,
                checksum, file_size, min_mockforge_version
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(scenario_id)
        .bind(version)
        .bind(manifest_json)
        .bind(download_url)
        .bind(checksum)
        .bind(file_size)
        .bind(min_mockforge_version)
        .fetch_one(pool)
        .await
    }

    /// Find by scenario and version
    pub async fn find(
        pool: &sqlx::PgPool,
        scenario_id: Uuid,
        version: &str,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM scenario_versions WHERE scenario_id = $1 AND version = $2",
        )
        .bind(scenario_id)
        .bind(version)
        .fetch_optional(pool)
        .await
    }

    /// Get all versions for a scenario
    pub async fn get_by_scenario(
        pool: &sqlx::PgPool,
        scenario_id: Uuid,
    ) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM scenario_versions WHERE scenario_id = $1 ORDER BY published_at DESC",
        )
        .bind(scenario_id)
        .fetch_all(pool)
        .await
    }
}
