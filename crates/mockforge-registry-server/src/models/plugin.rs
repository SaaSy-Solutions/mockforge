//! Plugin model

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Plugin {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub current_version: String,
    pub category: String,
    pub license: String,
    pub repository: Option<String>,
    pub homepage: Option<String>,
    pub downloads_total: i64,
    #[sqlx(try_from = "f64")]
    pub rating_avg: f64,
    pub rating_count: i32,
    pub author_id: Uuid,
    pub verified_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct PluginVersion {
    pub id: Uuid,
    pub plugin_id: Uuid,
    pub version: String,
    pub download_url: String,
    pub checksum: String,
    pub file_size: i64,
    pub min_mockforge_version: Option<String>,
    pub yanked: bool,
    pub downloads: i32,
    pub published_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginWithVersions {
    #[serde(flatten)]
    pub plugin: Plugin,
    pub versions: Vec<PluginVersion>,
    pub tags: Vec<String>,
}

impl Plugin {
    /// Search plugins
    pub async fn search(
        pool: &sqlx::PgPool,
        query: Option<&str>,
        category: Option<&str>,
        tags: &[String],
        sort_by: &str,
        limit: i64,
        offset: i64,
    ) -> sqlx::Result<Vec<Self>> {
        let mut sql = String::from(
            r#"
            SELECT DISTINCT p.*
            FROM plugins p
            "#,
        );

        let mut conditions = Vec::new();
        let mut params_count = 0;

        // Add tag filtering if needed
        if !tags.is_empty() {
            sql.push_str(
                r#"
                INNER JOIN plugin_tags pt ON p.id = pt.plugin_id
                INNER JOIN tags t ON pt.tag_id = t.id
                "#,
            );
            params_count += 1;
            conditions.push(format!("t.name = ANY(${})", params_count));
        }

        sql.push_str(" WHERE 1=1 ");

        // Add search query
        if let Some(_q) = query {
            params_count += 1;
            conditions
                .push(format!("p.search_vector @@ plainto_tsquery('english', ${})", params_count));
        }

        // Add category filter
        if let Some(_cat) = category {
            params_count += 1;
            conditions.push(format!("p.category = ${}", params_count));
        }

        if !conditions.is_empty() {
            sql.push_str(" AND ");
            sql.push_str(&conditions.join(" AND "));
        }

        // Add sorting
        match sort_by {
            "downloads" => sql.push_str(" ORDER BY p.downloads_total DESC"),
            "rating" => sql.push_str(" ORDER BY p.rating_avg DESC"),
            "recent" => sql.push_str(" ORDER BY p.created_at DESC"),
            "name" => sql.push_str(" ORDER BY p.name ASC"),
            _ => sql.push_str(" ORDER BY p.downloads_total DESC"),
        }

        params_count += 2;
        sql.push_str(&format!(" LIMIT ${} OFFSET ${}", params_count - 1, params_count));

        let mut query_builder = sqlx::query_as::<_, Self>(&sql);

        // Bind parameters in order
        if !tags.is_empty() {
            query_builder = query_builder.bind(tags);
        }
        if let Some(q) = query {
            query_builder = query_builder.bind(q);
        }
        if let Some(cat) = category {
            query_builder = query_builder.bind(cat);
        }
        query_builder = query_builder.bind(limit).bind(offset);

        query_builder.fetch_all(pool).await
    }

    /// Find plugin by name
    pub async fn find_by_name(pool: &sqlx::PgPool, name: &str) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM plugins WHERE name = $1")
            .bind(name)
            .fetch_optional(pool)
            .await
    }

    /// Get plugin tags
    pub async fn get_tags(pool: &sqlx::PgPool, plugin_id: Uuid) -> sqlx::Result<Vec<String>> {
        let tags: Vec<(String,)> = sqlx::query_as(
            r#"
            SELECT t.name
            FROM tags t
            INNER JOIN plugin_tags pt ON t.id = pt.tag_id
            WHERE pt.plugin_id = $1
            "#,
        )
        .bind(plugin_id)
        .fetch_all(pool)
        .await?;

        Ok(tags.into_iter().map(|(name,)| name).collect())
    }

    /// Create new plugin
    pub async fn create(
        pool: &sqlx::PgPool,
        name: &str,
        description: &str,
        version: &str,
        category: &str,
        license: &str,
        repository: Option<&str>,
        homepage: Option<&str>,
        author_id: Uuid,
    ) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO plugins (
                name, description, current_version, category, license,
                repository, homepage, author_id
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(name)
        .bind(description)
        .bind(version)
        .bind(category)
        .bind(license)
        .bind(repository)
        .bind(homepage)
        .bind(author_id)
        .fetch_one(pool)
        .await
    }

    /// Increment download count
    pub async fn increment_downloads(pool: &sqlx::PgPool, plugin_id: Uuid) -> sqlx::Result<()> {
        sqlx::query("UPDATE plugins SET downloads_total = downloads_total + 1 WHERE id = $1")
            .bind(plugin_id)
            .execute(pool)
            .await?;
        Ok(())
    }
}

impl PluginVersion {
    /// Get all versions for a plugin
    pub async fn get_by_plugin(pool: &sqlx::PgPool, plugin_id: Uuid) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM plugin_versions WHERE plugin_id = $1 ORDER BY published_at DESC",
        )
        .bind(plugin_id)
        .fetch_all(pool)
        .await
    }

    /// Find specific version
    pub async fn find(
        pool: &sqlx::PgPool,
        plugin_id: Uuid,
        version: &str,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM plugin_versions WHERE plugin_id = $1 AND version = $2",
        )
        .bind(plugin_id)
        .bind(version)
        .fetch_optional(pool)
        .await
    }

    /// Create new version
    pub async fn create(
        pool: &sqlx::PgPool,
        plugin_id: Uuid,
        version: &str,
        download_url: &str,
        checksum: &str,
        file_size: i64,
        min_mockforge_version: Option<&str>,
    ) -> sqlx::Result<Self> {
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO plugin_versions (
                plugin_id, version, download_url, checksum, file_size, min_mockforge_version
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(plugin_id)
        .bind(version)
        .bind(download_url)
        .bind(checksum)
        .bind(file_size)
        .bind(min_mockforge_version)
        .fetch_one(pool)
        .await
    }

    /// Yank a version
    pub async fn yank(pool: &sqlx::PgPool, version_id: Uuid) -> sqlx::Result<()> {
        sqlx::query("UPDATE plugin_versions SET yanked = true WHERE id = $1")
            .bind(version_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Increment download count
    pub async fn increment_downloads(pool: &sqlx::PgPool, version_id: Uuid) -> sqlx::Result<()> {
        sqlx::query("UPDATE plugin_versions SET downloads = downloads + 1 WHERE id = $1")
            .bind(version_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Get dependencies for a version
    pub async fn get_dependencies(
        pool: &sqlx::PgPool,
        version_id: Uuid,
    ) -> sqlx::Result<std::collections::HashMap<String, String>> {
        let deps = sqlx::query_as::<_, (String, String)>(
            "SELECT depends_on_plugin, version_requirement FROM plugin_dependencies WHERE version_id = $1"
        )
        .bind(version_id)
        .fetch_all(pool)
        .await?;

        Ok(deps.into_iter().collect())
    }

    /// Add dependency
    pub async fn add_dependency(
        pool: &sqlx::PgPool,
        version_id: Uuid,
        plugin_name: &str,
        version_req: &str,
    ) -> sqlx::Result<()> {
        sqlx::query(
            "INSERT INTO plugin_dependencies (version_id, depends_on_plugin, version_requirement) VALUES ($1, $2, $3)"
        )
        .bind(version_id)
        .bind(plugin_name)
        .bind(version_req)
        .execute(pool)
        .await?;
        Ok(())
    }
}
