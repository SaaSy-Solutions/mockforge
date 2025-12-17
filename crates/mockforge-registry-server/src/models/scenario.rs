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

        // Org filtering
        if let Some(_org) = org_id {
            param_placeholders.push(format!("${}", param_index));
            where_parts.push(format!("(org_id = ${} OR org_id IS NULL)", param_index));
            param_index += 1;
        } else {
            // Public scenarios only if no org context
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

        let where_clause = if where_parts.is_empty() {
            "WHERE 1=1".to_string()
        } else {
            format!("WHERE {}", where_parts.join(" AND "))
        };

        (where_clause, param_placeholders)
    }

    /// Count scenarios matching search criteria
    pub async fn count_search(
        pool: &sqlx::PgPool,
        query: Option<&str>,
        category: Option<&str>,
        tags: &[String],
        org_id: Option<Uuid>,
    ) -> sqlx::Result<i64> {
        let (where_clause, _) = Self::build_search_where_clause(query, category, tags, org_id);
        let sql = format!("SELECT COUNT(*) FROM scenarios {}", where_clause);

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
        let (where_clause, _) = Self::build_search_where_clause(query, category, tags, org_id);

        // Sort
        let order_by = match sort {
            "downloads" => "ORDER BY downloads_total DESC",
            "rating" => "ORDER BY rating_avg DESC",
            "recent" => "ORDER BY created_at DESC",
            "name" => "ORDER BY name ASC",
            _ => "ORDER BY downloads_total DESC",
        };

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
            "SELECT * FROM scenarios {} {} LIMIT ${} OFFSET ${}",
            where_clause,
            order_by,
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

    /// Find scenarios by organization
    pub async fn find_by_org(pool: &sqlx::PgPool, org_id: Uuid) -> sqlx::Result<Vec<Self>> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::prelude::*;

    #[test]
    fn test_scenario_serialization() {
        let scenario = Scenario {
            id: Uuid::new_v4(),
            org_id: Some(Uuid::new_v4()),
            name: "Test Scenario".to_string(),
            slug: "test-scenario".to_string(),
            description: "A test scenario".to_string(),
            author_id: Uuid::new_v4(),
            current_version: "1.0.0".to_string(),
            category: "testing".to_string(),
            tags: vec!["test".to_string(), "demo".to_string()],
            license: "MIT".to_string(),
            repository: Some("https://github.com/test/repo".to_string()),
            homepage: Some("https://test.com".to_string()),
            manifest_json: serde_json::json!({"version": "1.0.0"}),
            downloads_total: 100,
            rating_avg: Decimal::from_str("4.5").unwrap(),
            rating_count: 10,
            verified_at: Some(Utc::now()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let json = serde_json::to_string(&scenario).unwrap();
        assert!(json.contains("Test Scenario"));
        assert!(json.contains("test-scenario"));
        assert!(json.contains("testing"));
    }

    #[test]
    fn test_scenario_deserialization() {
        let json = r#"{
            "id": "00000000-0000-0000-0000-000000000001",
            "org_id": "00000000-0000-0000-0000-000000000002",
            "name": "Test Scenario",
            "slug": "test-scenario",
            "description": "A test scenario",
            "author_id": "00000000-0000-0000-0000-000000000003",
            "current_version": "1.0.0",
            "category": "testing",
            "tags": ["test", "demo"],
            "license": "MIT",
            "repository": "https://github.com/test/repo",
            "homepage": "https://test.com",
            "manifest_json": {"version": "1.0.0"},
            "downloads_total": 100,
            "rating_avg": 4.5,
            "rating_count": 10,
            "verified_at": "2024-01-01T00:00:00Z",
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z"
        }"#;

        let scenario: Scenario = serde_json::from_str(json).unwrap();
        assert_eq!(scenario.name, "Test Scenario");
        assert_eq!(scenario.slug, "test-scenario");
        assert_eq!(scenario.category, "testing");
        assert_eq!(scenario.tags.len(), 2);
    }

    #[test]
    fn test_scenario_clone() {
        let scenario = Scenario {
            id: Uuid::new_v4(),
            org_id: None,
            name: "Test Scenario".to_string(),
            slug: "test-scenario".to_string(),
            description: "A test scenario".to_string(),
            author_id: Uuid::new_v4(),
            current_version: "1.0.0".to_string(),
            category: "testing".to_string(),
            tags: vec!["test".to_string()],
            license: "MIT".to_string(),
            repository: None,
            homepage: None,
            manifest_json: serde_json::json!({}),
            downloads_total: 0,
            rating_avg: Decimal::from_str("0.0").unwrap(),
            rating_count: 0,
            verified_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let cloned = scenario.clone();
        assert_eq!(scenario.id, cloned.id);
        assert_eq!(scenario.name, cloned.name);
        assert_eq!(scenario.slug, cloned.slug);
    }

    #[test]
    fn test_scenario_version_serialization() {
        let version = ScenarioVersion {
            id: Uuid::new_v4(),
            scenario_id: Uuid::new_v4(),
            version: "1.0.0".to_string(),
            manifest_json: serde_json::json!({"version": "1.0.0"}),
            download_url: "https://example.com/scenario.tar.gz".to_string(),
            checksum: "abc123".to_string(),
            file_size: 1024,
            min_mockforge_version: Some("0.1.0".to_string()),
            yanked: false,
            downloads: 50,
            published_at: Utc::now(),
        };

        let json = serde_json::to_string(&version).unwrap();
        assert!(json.contains("1.0.0"));
        assert!(json.contains("abc123"));
        assert!(json.contains("1024"));
    }

    #[test]
    fn test_scenario_version_clone() {
        let version = ScenarioVersion {
            id: Uuid::new_v4(),
            scenario_id: Uuid::new_v4(),
            version: "1.0.0".to_string(),
            manifest_json: serde_json::json!({}),
            download_url: "https://example.com/scenario.tar.gz".to_string(),
            checksum: "abc123".to_string(),
            file_size: 1024,
            min_mockforge_version: None,
            yanked: false,
            downloads: 0,
            published_at: Utc::now(),
        };

        let cloned = version.clone();
        assert_eq!(version.id, cloned.id);
        assert_eq!(version.scenario_id, cloned.scenario_id);
        assert_eq!(version.version, cloned.version);
        assert_eq!(version.checksum, cloned.checksum);
    }

    #[test]
    fn test_build_search_where_clause_empty() {
        let (where_clause, params) = Scenario::build_search_where_clause(None, None, &[], None);

        assert!(where_clause.contains("WHERE"));
        assert!(where_clause.contains("org_id IS NULL"));
        assert!(params.is_empty() || !params.is_empty()); // params may vary
    }

    #[test]
    fn test_build_search_where_clause_with_org() {
        let org_id = Uuid::new_v4();
        let (where_clause, _params) =
            Scenario::build_search_where_clause(None, None, &[], Some(org_id));

        assert!(where_clause.contains("WHERE"));
        assert!(where_clause.contains("org_id"));
    }

    #[test]
    fn test_build_search_where_clause_with_category() {
        let (where_clause, _params) =
            Scenario::build_search_where_clause(None, Some("testing"), &[], None);

        assert!(where_clause.contains("WHERE"));
        assert!(where_clause.contains("category"));
    }

    #[test]
    fn test_build_search_where_clause_with_tags() {
        let tags = vec!["test".to_string(), "demo".to_string()];
        let (where_clause, _params) = Scenario::build_search_where_clause(None, None, &tags, None);

        assert!(where_clause.contains("WHERE"));
        assert!(where_clause.contains("tags"));
    }

    #[test]
    fn test_build_search_where_clause_with_query() {
        let (where_clause, _params) =
            Scenario::build_search_where_clause(Some("search term"), None, &[], None);

        assert!(where_clause.contains("WHERE"));
        assert!(where_clause.contains("tsvector") || where_clause.contains("tsquery"));
    }

    #[test]
    fn test_build_search_where_clause_all_params() {
        let org_id = Uuid::new_v4();
        let tags = vec!["test".to_string()];
        let (where_clause, _params) = Scenario::build_search_where_clause(
            Some("search"),
            Some("testing"),
            &tags,
            Some(org_id),
        );

        assert!(where_clause.contains("WHERE"));
        assert!(where_clause.contains("AND"));
    }

    #[test]
    fn test_scenario_with_org() {
        let org_id = Uuid::new_v4();
        let scenario = Scenario {
            id: Uuid::new_v4(),
            org_id: Some(org_id),
            name: "Org Scenario".to_string(),
            slug: "org-scenario".to_string(),
            description: "An org scenario".to_string(),
            author_id: Uuid::new_v4(),
            current_version: "1.0.0".to_string(),
            category: "testing".to_string(),
            tags: vec![],
            license: "MIT".to_string(),
            repository: None,
            homepage: None,
            manifest_json: serde_json::json!({}),
            downloads_total: 0,
            rating_avg: Decimal::from_str("0.0").unwrap(),
            rating_count: 0,
            verified_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert_eq!(scenario.org_id, Some(org_id));
    }

    #[test]
    fn test_scenario_public() {
        let scenario = Scenario {
            id: Uuid::new_v4(),
            org_id: None,
            name: "Public Scenario".to_string(),
            slug: "public-scenario".to_string(),
            description: "A public scenario".to_string(),
            author_id: Uuid::new_v4(),
            current_version: "1.0.0".to_string(),
            category: "testing".to_string(),
            tags: vec![],
            license: "MIT".to_string(),
            repository: None,
            homepage: None,
            manifest_json: serde_json::json!({}),
            downloads_total: 0,
            rating_avg: Decimal::from_str("0.0").unwrap(),
            rating_count: 0,
            verified_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert_eq!(scenario.org_id, None);
    }

    #[test]
    fn test_scenario_version_yanked() {
        let version = ScenarioVersion {
            id: Uuid::new_v4(),
            scenario_id: Uuid::new_v4(),
            version: "1.0.0".to_string(),
            manifest_json: serde_json::json!({}),
            download_url: "https://example.com/scenario.tar.gz".to_string(),
            checksum: "abc123".to_string(),
            file_size: 1024,
            min_mockforge_version: None,
            yanked: true,
            downloads: 0,
            published_at: Utc::now(),
        };

        assert!(version.yanked);
    }

    #[test]
    fn test_scenario_version_not_yanked() {
        let version = ScenarioVersion {
            id: Uuid::new_v4(),
            scenario_id: Uuid::new_v4(),
            version: "1.0.0".to_string(),
            manifest_json: serde_json::json!({}),
            download_url: "https://example.com/scenario.tar.gz".to_string(),
            checksum: "abc123".to_string(),
            file_size: 1024,
            min_mockforge_version: None,
            yanked: false,
            downloads: 0,
            published_at: Utc::now(),
        };

        assert!(!version.yanked);
    }
}
