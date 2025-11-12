//! Template marketplace handlers
//!
//! Provides endpoints for the template marketplace (orchestration templates for chaos testing)

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    middleware::{AuthUser, OptionalAuthUser, resolve_org_context},
    models::{Template, TemplateVersion, TemplateCategory, Organization, User, UsageCounter},
    AppState,
};

/// Search templates
/// Supports org filtering: if user is authenticated, includes their org's private templates
pub async fn search_templates(
    State(state): State<AppState>,
    OptionalAuthUser(maybe_user_id): OptionalAuthUser,
    headers: HeaderMap,
    Json(query): Json<TemplateSearchQuery>,
) -> ApiResult<Json<TemplateSearchResults>> {
    let pool = state.db.pool();

    // Try to resolve org context for filtering (optional)
    // If user is authenticated, include their org's private templates
    let org_id = if let Some(user_id) = maybe_user_id {
        if let Ok(org_ctx) = resolve_org_context(&state, user_id, &headers, None).await {
            Some(org_ctx.org_id)
        } else {
            None
        }
    } else {
        None
    };

    let limit = query.per_page as i64;
    let offset = (query.page * query.per_page) as i64;

    // Search templates
    let templates = Template::search(
        pool,
        query.query.as_deref(),
        query.category.as_deref(),
        &query.tags,
        org_id,
        limit,
        offset,
    )
    .await
    .map_err(|e| ApiError::Database(e))?;

    // Get total count for pagination (before converting entries)
    let total = Template::count_search(
        pool,
        query.query.as_deref(),
        query.category.as_deref(),
        &query.tags,
        org_id,
    )
    .await
    .map_err(|e| ApiError::Database(e))? as usize;

    // Convert to response format
    let mut entries = Vec::new();
    for template in templates {
        let versions = TemplateVersion::get_by_template(pool, template.id)
            .await
            .map_err(|e| ApiError::Database(e))?;

        let author = User::find_by_id(pool, template.author_id)
            .await
            .map_err(|e| ApiError::Database(e))?
            .unwrap_or_else(|| User {
                id: template.author_id,
                username: "unknown".to_string(),
                email: "unknown@example.com".to_string(),
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

        let stats = template.stats_json.clone();
        let compatibility = template.compatibility_json.clone();

        entries.push(TemplateRegistryEntry {
            id: template.id.to_string(),
            name: template.name,
            description: template.description,
            author: author.username,
            author_email: Some(author.email),
            version: template.version,
            category: template.category(),
            tags: template.tags,
            content: template.content_json,
            readme: template.readme,
            example_usage: template.example_usage,
            requirements: template.requirements,
            compatibility: serde_json::from_value(compatibility).unwrap_or_else(|_| CompatibilityInfo {
                min_version: "0.1.0".to_string(),
                max_version: None,
                required_features: vec![],
                protocols: vec![],
            }),
            stats: serde_json::from_value(stats).unwrap_or_else(|_| TemplateStats {
                downloads: 0,
                stars: 0,
                forks: 0,
                rating: 0.0,
                rating_count: 0,
            }),
            created_at: template.created_at.to_rfc3339(),
            updated_at: template.updated_at.to_rfc3339(),
            published: template.published,
        });
    }

    Ok(Json(TemplateSearchResults {
        templates: entries,
        total,
        page: query.page,
        per_page: query.per_page,
    }))
}

/// Get template by name and version
pub async fn get_template(
    State(state): State<AppState>,
    Path((name, version)): Path<(String, String)>,
) -> ApiResult<Json<TemplateRegistryEntry>> {
    let pool = state.db.pool();

    let template = Template::find_by_name_version(pool, &name, &version)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidRequest(format!("Template '{}@{}' not found", name, version)))?;

    let author = User::find_by_id(pool, template.author_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .unwrap_or_else(|| User {
            id: template.author_id,
            username: "unknown".to_string(),
            email: "unknown@example.com".to_string(),
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

    let stats = template.stats_json.clone();
    let compatibility = template.compatibility_json.clone();

    Ok(Json(TemplateRegistryEntry {
        id: template.id.to_string(),
        name: template.name,
        description: template.description,
        author: author.username,
        author_email: Some(author.email),
        version: template.version,
        category: template.category(),
        tags: template.tags,
        content: template.content_json,
        readme: template.readme,
        example_usage: template.example_usage,
        requirements: template.requirements,
        compatibility: serde_json::from_value(compatibility).unwrap_or_else(|_| CompatibilityInfo {
            min_version: "0.1.0".to_string(),
            max_version: None,
            required_features: vec![],
            protocols: vec![],
        }),
        stats: serde_json::from_value(stats).unwrap_or_else(|_| TemplateStats {
            downloads: 0,
            stars: 0,
            forks: 0,
            rating: 0.0,
            rating_count: 0,
        }),
        created_at: template.created_at.to_rfc3339(),
        updated_at: template.updated_at.to_rfc3339(),
        published: template.published,
    }))
}

/// Publish a template
pub async fn publish_template(
    State(state): State<AppState>,
    AuthUser(author_id): AuthUser,
    headers: HeaderMap,
    Json(request): Json<PublishTemplateRequest>,
) -> ApiResult<Json<PublishTemplateResponse>> {
    let pool = state.db.pool();

    // Resolve org context
    let org_ctx = resolve_org_context(&state, author_id, &headers, None).await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".to_string()))?;

    // Check publishing limits
    let limits = &org_ctx.org.limits_json;
    let max_templates = limits
        .get("max_templates_published")
        .and_then(|v| v.as_i64())
        .unwrap_or(3);

    if max_templates >= 0 {
        let existing = Template::find_by_org(pool, org_ctx.org_id)
            .await
            .map_err(|e| ApiError::Database(e))?;

        if existing.len() as i64 >= max_templates {
            return Err(ApiError::InvalidRequest(format!(
                "Template limit exceeded. Your plan allows {} templates. Upgrade to publish more.",
                max_templates
            )));
        }
    }

    // Check storage limit
    let storage_limit_gb = limits
        .get("storage_gb")
        .and_then(|v| v.as_i64())
        .unwrap_or(1);
    let storage_limit_bytes = storage_limit_gb * 1_000_000_000;

    let usage = UsageCounter::get_or_create_current(pool, org_ctx.org_id)
        .await
        .map_err(|e| ApiError::Database(e))?;

    let new_storage = usage.storage_bytes + request.file_size;
    if new_storage > storage_limit_bytes {
        return Err(ApiError::InvalidRequest(format!(
            "Storage limit exceeded. Your plan allows {} GB.",
            storage_limit_gb
        )));
    }

    // Decode package data
    let package_data = base64::decode(&request.package)
        .map_err(|e| ApiError::InvalidRequest(format!("Invalid base64: {}", e)))?;

    // Verify checksum
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(&package_data);
    let calculated_checksum = hex::encode(hasher.finalize());

    if calculated_checksum != request.checksum {
        return Err(ApiError::InvalidRequest("Checksum mismatch".to_string()));
    }

    // Upload to storage
    let download_url = state
        .storage
        .upload_template(&request.name, &request.version, package_data)
        .await
        .map_err(|e| ApiError::Storage(e.to_string()))?;

    // Create or update template
    let template = if let Some(existing) = Template::find_by_name_version(pool, &request.name, &request.version).await.map_err(|e| ApiError::Database(e))? {
        existing
    } else {
        Template::create(
            pool,
            Some(org_ctx.org_id),
            &request.name,
            &request.slug,
            &request.description,
            author_id,
            &request.version,
            request.category,
            request.content_json,
        )
        .await
        .map_err(|e| ApiError::Database(e))?
    };

    // Create version entry
    TemplateVersion::create(
        pool,
        template.id,
        &request.version,
        request.content_json,
        Some(&download_url),
        Some(&request.checksum),
        request.file_size,
    )
    .await
    .map_err(|e| ApiError::Database(e))?;

    // Update storage usage
    UsageCounter::update_storage(pool, org_ctx.org_id, new_storage)
        .await
        .map_err(|e| ApiError::Database(e))?;

    // Track feature usage
    let _ = FeatureUsage::record(
        pool,
        org_ctx.org_id,
        Some(author_id),
        FeatureType::TemplatePublish,
        Some(serde_json::json!({
            "template_name": request.name,
            "version": request.version,
        })),
    )
    .await;

    Ok(Json(PublishTemplateResponse {
        name: request.name,
        version: request.version,
        download_url,
        published_at: chrono::Utc::now().to_rfc3339(),
    }))
}

// Request/Response types

#[derive(Debug, Deserialize)]
pub struct TemplateSearchQuery {
    pub query: Option<String>,
    pub category: Option<String>,
    pub tags: Vec<String>,
    #[serde(default = "default_page")]
    pub page: usize,
    #[serde(default = "default_per_page")]
    pub per_page: usize,
}

fn default_page() -> usize {
    0
}

fn default_per_page() -> usize {
    20
}

#[derive(Debug, Serialize)]
pub struct TemplateSearchResults {
    pub templates: Vec<TemplateRegistryEntry>,
    pub total: usize,
    pub page: usize,
    pub per_page: usize,
}

#[derive(Debug, Serialize)]
pub struct TemplateRegistryEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub author_email: Option<String>,
    pub version: String,
    #[serde(rename = "category")]
    pub category: TemplateCategory,
    pub tags: Vec<String>,
    pub content: serde_json::Value,
    pub readme: Option<String>,
    pub example_usage: Option<String>,
    pub requirements: Vec<String>,
    pub compatibility: CompatibilityInfo,
    pub stats: TemplateStats,
    pub created_at: String,
    pub updated_at: String,
    pub published: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompatibilityInfo {
    pub min_version: String,
    pub max_version: Option<String>,
    pub required_features: Vec<String>,
    pub protocols: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TemplateStats {
    pub downloads: u64,
    pub stars: u64,
    pub forks: u64,
    pub rating: f64,
    pub rating_count: u64,
}

#[derive(Debug, Deserialize)]
pub struct PublishTemplateRequest {
    pub name: String,
    pub slug: String,
    pub description: String,
    pub version: String,
    pub category: TemplateCategory,
    pub content_json: serde_json::Value,
    pub package: String, // Base64 encoded
    pub checksum: String,
    pub file_size: i64,
}

#[derive(Debug, Serialize)]
pub struct PublishTemplateResponse {
    pub name: String,
    pub version: String,
    pub download_url: String,
    pub published_at: String,
}
