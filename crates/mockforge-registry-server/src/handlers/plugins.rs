//! Plugin-related handlers

use axum::{
    extract::{Path, State},
    Json,
};
use mockforge_plugin_registry::{
    AuthorInfo, PluginCategory, RegistryEntry, SearchQuery, SearchResults, SortOrder, VersionEntry,
};
use serde::{Deserialize, Serialize};

use crate::{
    error::{ApiError, ApiResult},
    middleware::{AuthUser, ScopedAuth},
    models::{Plugin, PluginVersion, TokenScope, User},
    AppState,
};

pub async fn search_plugins(
    State(state): State<AppState>,
    Json(query): Json<SearchQuery>,
) -> ApiResult<Json<SearchResults>> {
    let metrics = crate::metrics::MarketplaceMetrics::start(state.metrics.clone(), "plugin");
    let pool = state.db.pool();

    // Map sort order
    let sort_by = match query.sort {
        SortOrder::Downloads => "downloads",
        SortOrder::Rating => "rating",
        SortOrder::Recent => "recent",
        SortOrder::Name => "name",
        _ => "downloads",
    };

    // Map category to string
    let category_str = query.category.as_ref().map(|c| match c {
        PluginCategory::Auth => "auth",
        PluginCategory::Template => "template",
        PluginCategory::Response => "response",
        PluginCategory::DataSource => "datasource",
        PluginCategory::Middleware => "middleware",
        PluginCategory::Testing => "testing",
        PluginCategory::Observability => "observability",
        PluginCategory::Other => "other",
    });

    // Validate and limit pagination parameters
    let per_page = query.per_page.min(100).max(1); // Max 100 items per page
    let page = query.page;
    let limit = per_page as i64;
    let offset = (page * per_page) as i64;

    // Search plugins
    let plugins = match Plugin::search(
        pool,
        query.query.as_deref(),
        category_str,
        &query.tags,
        sort_by,
        limit,
        offset,
    )
    .await
    {
        Ok(plugins) => plugins,
        Err(e) => {
            metrics.record_search_error("database_error");
            return Err(ApiError::Database(e));
        }
    };

    // Convert to registry entries
    let mut entries = Vec::new();
    for plugin in plugins {
        let tags = Plugin::get_tags(pool, plugin.id).await.map_err(|e| ApiError::Database(e))?;

        let versions = PluginVersion::get_by_plugin(pool, plugin.id)
            .await
            .map_err(|e| ApiError::Database(e))?;

        let category = map_category_from_string(&plugin.category);

        // Load versions with dependencies
        let mut version_entries = Vec::new();
        for v in versions {
            let dependencies = PluginVersion::get_dependencies(pool, v.id)
                .await
                .map_err(|e| ApiError::Database(e))?;

            version_entries.push(VersionEntry {
                version: v.version,
                download_url: v.download_url,
                checksum: v.checksum,
                size: v.file_size as u64,
                published_at: v.published_at.to_rfc3339(),
                yanked: v.yanked,
                min_mockforge_version: v.min_mockforge_version,
                dependencies,
            });
        }

        // Fetch author information
        let author = User::find_by_id(pool, plugin.author_id)
            .await
            .map_err(|e| ApiError::Database(e))?
            .unwrap_or_else(|| {
                // Create a minimal user struct for display purposes
                // This should not happen in production, but handle gracefully
                User {
                    id: plugin.author_id,
                    username: "Unknown".to_string(),
                    email: String::new(),
                    password_hash: String::new(),
                    api_token: None,
                    is_verified: false,
                    is_admin: false,
                    two_factor_enabled: false,
                    two_factor_secret: None,
                    two_factor_backup_codes: None,
                    two_factor_verified_at: None,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                }
            });

        entries.push(RegistryEntry {
            name: plugin.name.clone(),
            description: plugin.description.clone(),
            version: plugin.current_version.clone(),
            versions: version_entries,
            author: AuthorInfo {
                name: author.username,
                email: Some(author.email),
                url: None,
            },
            tags,
            category,
            downloads: plugin.downloads_total as u64,
            rating: plugin.rating_avg,
            reviews_count: plugin.rating_count as u32,
            repository: plugin.repository,
            homepage: plugin.homepage,
            license: plugin.license,
            created_at: plugin.created_at.to_rfc3339(),
            updated_at: plugin.updated_at.to_rfc3339(),
        });
    }

    // Count total (simplified - just return current count for MVP)
    let total = entries.len();

    let results = SearchResults {
        plugins: entries,
        total,
        page,
        per_page,
    };

    // Record metrics
    metrics.record_search_success();

    Ok(Json(results))
}

pub async fn get_plugin(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> ApiResult<Json<RegistryEntry>> {
    let metrics = crate::metrics::MarketplaceMetrics::start(state.metrics.clone(), "plugin");
    let pool = state.db.pool();

    let plugin = Plugin::find_by_name(pool, &name)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::PluginNotFound(name.clone()))?;

    let tags = Plugin::get_tags(pool, plugin.id).await.map_err(|e| ApiError::Database(e))?;

    let versions = PluginVersion::get_by_plugin(pool, plugin.id)
        .await
        .map_err(|e| ApiError::Database(e))?;

    let category = map_category_from_string(&plugin.category);

    // Load versions with dependencies
    let mut version_entries = Vec::new();
    for v in versions {
        let dependencies = PluginVersion::get_dependencies(pool, v.id)
            .await
            .map_err(|e| ApiError::Database(e))?;

        version_entries.push(VersionEntry {
            version: v.version,
            download_url: v.download_url,
            checksum: v.checksum,
            size: v.file_size as u64,
            published_at: v.published_at.to_rfc3339(),
            yanked: v.yanked,
            min_mockforge_version: v.min_mockforge_version,
            dependencies,
        });
    }

    // Fetch author information
    let author = User::find_by_id(pool, plugin.author_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .unwrap_or_else(|| User {
            id: plugin.author_id,
            username: "Unknown".to_string(),
            email: String::new(),
            password_hash: String::new(),
            api_token: None,
            is_verified: false,
            is_admin: false,
            two_factor_enabled: false,
            two_factor_secret: None,
            two_factor_backup_codes: None,
            two_factor_verified_at: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        });

    let entry = RegistryEntry {
        name: plugin.name.clone(),
        description: plugin.description.clone(),
        version: plugin.current_version.clone(),
        versions: version_entries,
        author: AuthorInfo {
            name: author.username,
            email: Some(author.email),
            url: None,
        },
        tags,
        category,
        downloads: plugin.downloads_total as u64,
        rating: plugin.rating_avg.to_string().parse().unwrap_or(0.0),
        reviews_count: plugin.rating_count as u32,
        repository: plugin.repository,
        homepage: plugin.homepage,
        license: plugin.license,
        created_at: plugin.created_at.to_rfc3339(),
        updated_at: plugin.updated_at.to_rfc3339(),
    };

    // Record metrics
    metrics.record_download_success();

    Ok(Json(entry))
}

pub async fn get_version(
    State(state): State<AppState>,
    Path((name, version)): Path<(String, String)>,
) -> ApiResult<Json<VersionEntry>> {
    let metrics = crate::metrics::MarketplaceMetrics::start(state.metrics.clone(), "plugin");
    let pool = state.db.pool();

    let plugin = Plugin::find_by_name(pool, &name)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::PluginNotFound(name.clone()))?;

    let plugin_version = PluginVersion::find(pool, plugin.id, &version)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidVersion(version.clone()))?;

    // Load dependencies
    let dependencies = PluginVersion::get_dependencies(pool, plugin_version.id)
        .await
        .map_err(|e| ApiError::Database(e))?;

    let entry = VersionEntry {
        version: plugin_version.version,
        download_url: plugin_version.download_url,
        checksum: plugin_version.checksum,
        size: plugin_version.file_size as u64,
        published_at: plugin_version.published_at.to_rfc3339(),
        yanked: plugin_version.yanked,
        min_mockforge_version: plugin_version.min_mockforge_version,
        dependencies,
    };

    // Record metrics
    metrics.record_download_success();

    Ok(Json(entry))
}

#[derive(Debug, Deserialize)]
pub struct PublishRequest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub category: String,
    pub license: String,
    pub repository: Option<String>,
    pub homepage: Option<String>,
    pub tags: Vec<String>,
    pub checksum: String,
    pub file_size: i64,
    pub wasm_data: String, // Base64 encoded WASM
    pub dependencies: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Serialize)]
pub struct PublishResponse {
    pub success: bool,
    pub upload_url: String,
    pub message: String,
}

pub async fn publish_plugin(
    AuthUser(author_id): AuthUser,
    scoped_auth: ScopedAuth,
    State(state): State<AppState>,
    Json(request): Json<PublishRequest>,
) -> ApiResult<Json<PublishResponse>> {
    // Check for PublishPackages scope
    scoped_auth.require_scope(TokenScope::PublishPackages)?;

    let metrics = crate::metrics::MarketplaceMetrics::start(state.metrics.clone(), "plugin");
    let pool = state.db.pool();

    // Check if plugin exists
    let existing = Plugin::find_by_name(pool, &request.name)
        .await
        .map_err(|e| ApiError::Database(e))?;

    let plugin = if let Some(mut plugin) = existing {
        // Update existing plugin
        plugin.current_version = request.version.clone();
        plugin.description = request.description.clone();
        plugin
    } else {
        // Create new plugin
        Plugin::create(
            pool,
            &request.name,
            &request.description,
            &request.version,
            &request.category,
            &request.license,
            request.repository.as_deref(),
            request.homepage.as_deref(),
            author_id,
        )
        .await
        .map_err(|e| ApiError::Database(e))?
    };

    // Validate input fields
    crate::validation::validate_name(&request.name)?;
    crate::validation::validate_version(&request.version)?;
    crate::validation::validate_checksum(&request.checksum)?;

    // Validate base64 encoding
    crate::validation::validate_base64(&request.wasm_data)?;

    // Decode WASM data
    let wasm_bytes = base64::decode(&request.wasm_data)
        .map_err(|e| ApiError::InvalidRequest(format!("Invalid base64: {}", e)))?;

    // Validate WASM file
    crate::validation::validate_wasm_file(&wasm_bytes, request.file_size as u64)?;

    // Verify checksum matches uploaded data
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(&wasm_bytes);
    let calculated_checksum = hex::encode(hasher.finalize());

    if calculated_checksum != request.checksum {
        return Err(ApiError::InvalidRequest(format!(
            "Checksum mismatch: expected {}, got {}",
            request.checksum, calculated_checksum
        )));
    }

    // Upload to S3
    let download_url = state
        .storage
        .upload_plugin(&request.name, &request.version, wasm_bytes)
        .await
        .map_err(|e| ApiError::Storage(e.to_string()))?;

    // Create version entry
    let version = PluginVersion::create(
        pool,
        plugin.id,
        &request.version,
        &download_url,
        &request.checksum,
        request.file_size,
        None,
    )
    .await
    .map_err(|e| ApiError::Database(e))?;

    // Add dependencies if provided
    if let Some(deps) = request.dependencies {
        for (dep_name, dep_version) in deps {
            PluginVersion::add_dependency(pool, version.id, &dep_name, &dep_version)
                .await
                .map_err(|e| ApiError::Database(e))?;
        }
    }

    // Record metrics
    metrics.record_publish_success();

    Ok(Json(PublishResponse {
        success: true,
        upload_url: download_url.clone(),
        message: format!(
            "Plugin {} version {} published successfully",
            request.name, request.version
        ),
    }))
}

pub async fn yank_version(
    scoped_auth: ScopedAuth,
    State(state): State<AppState>,
    Path((name, version)): Path<(String, String)>,
) -> ApiResult<Json<serde_json::Value>> {
    // Check for PublishPackages scope (yanking is a publishing operation)
    scoped_auth.require_scope(TokenScope::PublishPackages)?;

    let pool = state.db.pool();

    let plugin = Plugin::find_by_name(pool, &name)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::PluginNotFound(name.clone()))?;

    let plugin_version = PluginVersion::find(pool, plugin.id, &version)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidVersion(version.clone()))?;

    PluginVersion::yank(pool, plugin_version.id)
        .await
        .map_err(|e| ApiError::Database(e))?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Version {} of {} yanked successfully", version, name)
    })))
}

fn map_category_from_string(cat: &str) -> PluginCategory {
    match cat {
        "auth" => PluginCategory::Auth,
        "template" => PluginCategory::Template,
        "response" => PluginCategory::Response,
        "datasource" => PluginCategory::DataSource,
        "middleware" => PluginCategory::Middleware,
        "testing" => PluginCategory::Testing,
        "observability" => PluginCategory::Observability,
        _ => PluginCategory::Other,
    }
}
