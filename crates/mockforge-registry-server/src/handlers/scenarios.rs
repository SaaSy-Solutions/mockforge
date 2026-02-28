//! Scenario marketplace handlers
//!
//! Provides endpoints for the scenario marketplace (data scenarios for mock systems)

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    middleware::{resolve_org_context, AuthUser, OptionalAuthUser},
    models::{
        record_audit_event, AuditEventType, FeatureType, FeatureUsage, Scenario, ScenarioReview,
        ScenarioVersion, UsageCounter, User,
    },
    AppState,
};

/// Search scenarios
/// Supports org filtering: if user is authenticated, includes their org's private scenarios
pub async fn search_scenarios(
    State(state): State<AppState>,
    OptionalAuthUser(maybe_user_id): OptionalAuthUser,
    headers: HeaderMap,
    Json(query): Json<ScenarioSearchQuery>,
) -> ApiResult<Json<ScenarioSearchResults>> {
    let metrics = crate::metrics::MarketplaceMetrics::start(state.metrics.clone(), "scenario");
    let pool = state.db.pool();

    // Try to resolve org context for filtering (optional)
    // If user is authenticated, include their org's private scenarios
    let org_id = if let Some(user_id) = maybe_user_id {
        if let Ok(org_ctx) = resolve_org_context(&state, user_id, &headers, None).await {
            Some(org_ctx.org_id)
        } else {
            None
        }
    } else {
        None
    };

    // Validate and limit pagination parameters
    let per_page = query.per_page.min(100).max(1); // Max 100 items per page
    let page = query.page;
    let limit = per_page as i64;
    let offset = (page * per_page) as i64;

    // Map sort order
    let sort = match query.sort {
        ScenarioSortOrder::Relevance => "downloads", // Default to downloads for relevance
        ScenarioSortOrder::Downloads => "downloads",
        ScenarioSortOrder::Rating => "rating",
        ScenarioSortOrder::Recent => "recent",
        ScenarioSortOrder::Name => "name",
    };

    // Get total count for pagination (before fetching results)
    let total = Scenario::count_search(
        pool,
        query.query.as_deref(),
        query.category.as_deref(),
        &query.tags,
        org_id,
    )
    .await
    .map_err(|e| ApiError::Database(e))? as usize;

    // Search scenarios
    let scenarios = Scenario::search(
        pool,
        query.query.as_deref(),
        query.category.as_deref(),
        &query.tags,
        org_id,
        sort,
        limit,
        offset,
    )
    .await
    .map_err(|e| ApiError::Database(e))?;

    // Convert to registry entries
    let mut entries = Vec::new();
    for scenario in scenarios {
        let versions = ScenarioVersion::get_by_scenario(pool, scenario.id)
            .await
            .map_err(|e| ApiError::Database(e))?;

        let author = User::find_by_id(pool, scenario.author_id)
            .await
            .map_err(|e| ApiError::Database(e))?
            .unwrap_or_else(|| User {
                id: scenario.author_id,
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

        let version_entries: Vec<ScenarioVersionEntry> = versions
            .into_iter()
            .filter(|v| !v.yanked)
            .map(|v| ScenarioVersionEntry {
                version: v.version,
                download_url: v.download_url,
                checksum: v.checksum,
                size: v.file_size as u64,
                published_at: v.published_at.to_rfc3339(),
                yanked: v.yanked,
                min_mockforge_version: v.min_mockforge_version,
            })
            .collect();

        // Load top 3 reviews (most helpful) for search results
        let reviews = ScenarioReview::get_by_scenario(pool, scenario.id, 3, 0)
            .await
            .map_err(|e| ApiError::Database(e))?;

        // Batch load all reviewers to avoid N+1 queries
        let reviewer_ids: Vec<Uuid> = reviews.iter().map(|r| r.reviewer_id).collect();
        let reviewers: std::collections::HashMap<Uuid, User> = if !reviewer_ids.is_empty() {
            User::find_by_ids(pool, &reviewer_ids)
                .await
                .map_err(|e| ApiError::Database(e))?
                .into_iter()
                .map(|u| (u.id, u))
                .collect()
        } else {
            std::collections::HashMap::new()
        };

        let review_responses: Vec<ScenarioReviewResponse> = reviews
            .into_iter()
            .map(|review| {
                let reviewer =
                    reviewers.get(&review.reviewer_id).cloned().unwrap_or_else(|| User {
                        id: review.reviewer_id,
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

                ScenarioReviewResponse {
                    id: review.id.to_string(),
                    reviewer: reviewer.username,
                    reviewer_email: Some(reviewer.email),
                    rating: review.rating as u8,
                    title: review.title,
                    comment: review.comment,
                    created_at: review.created_at.to_rfc3339(),
                    helpful_count: review.helpful_count as u32,
                    verified_purchase: review.verified_purchase,
                }
            })
            .collect();

        entries.push(ScenarioRegistryEntry {
            name: scenario.name,
            description: scenario.description,
            version: scenario.current_version,
            versions: version_entries,
            author: author.username,
            author_email: Some(author.email),
            tags: scenario.tags,
            category: scenario.category,
            downloads: scenario.downloads_total as u64,
            rating: scenario.rating_avg.to_string().parse::<f64>().unwrap_or(0.0),
            reviews_count: scenario.rating_count as u32,
            reviews: review_responses,
            repository: scenario.repository,
            homepage: scenario.homepage,
            license: scenario.license,
            created_at: scenario.created_at.to_rfc3339(),
            updated_at: scenario.updated_at.to_rfc3339(),
        });
    }

    // Record metrics
    metrics.record_search_success();

    Ok(Json(ScenarioSearchResults {
        scenarios: entries,
        total,
        page,
        per_page,
    }))
}

/// Get scenario by name
pub async fn get_scenario(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> ApiResult<Json<ScenarioRegistryEntry>> {
    let metrics = crate::metrics::MarketplaceMetrics::start(state.metrics.clone(), "scenario");
    let pool = state.db.pool();

    let scenario = Scenario::find_by_name(pool, &name)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::ScenarioNotFound(name.clone()))?;

    let versions = ScenarioVersion::get_by_scenario(pool, scenario.id)
        .await
        .map_err(|e| ApiError::Database(e))?;

    let author = User::find_by_id(pool, scenario.author_id)
        .await
        .map_err(|e| ApiError::Database(e))?
        .unwrap_or_else(|| User {
            id: scenario.author_id,
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

    let version_entries: Vec<ScenarioVersionEntry> = versions
        .into_iter()
        .map(|v| ScenarioVersionEntry {
            version: v.version,
            download_url: v.download_url,
            checksum: v.checksum,
            size: v.file_size as u64,
            published_at: v.published_at.to_rfc3339(),
            yanked: v.yanked,
            min_mockforge_version: v.min_mockforge_version,
        })
        .collect();

    // Load top 5 reviews (most helpful) for single scenario view
    let reviews = ScenarioReview::get_by_scenario(pool, scenario.id, 5, 0)
        .await
        .map_err(|e| ApiError::Database(e))?;

    // Batch load all reviewers to avoid N+1 queries
    let reviewer_ids: Vec<Uuid> = reviews.iter().map(|r| r.reviewer_id).collect();
    let reviewers: std::collections::HashMap<Uuid, User> = if !reviewer_ids.is_empty() {
        User::find_by_ids(pool, &reviewer_ids)
            .await
            .map_err(|e| ApiError::Database(e))?
            .into_iter()
            .map(|u| (u.id, u))
            .collect()
    } else {
        std::collections::HashMap::new()
    };

    let review_responses: Vec<ScenarioReviewResponse> = reviews
        .into_iter()
        .map(|review| {
            let reviewer = reviewers.get(&review.reviewer_id).cloned().unwrap_or_else(|| User {
                id: review.reviewer_id,
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

            ScenarioReviewResponse {
                id: review.id.to_string(),
                reviewer: reviewer.username,
                reviewer_email: Some(reviewer.email),
                rating: review.rating as u8,
                title: review.title,
                comment: review.comment,
                created_at: review.created_at.to_rfc3339(),
                helpful_count: review.helpful_count as u32,
                verified_purchase: review.verified_purchase,
            }
        })
        .collect();

    // Record metrics
    metrics.record_download_success();

    Ok(Json(ScenarioRegistryEntry {
        name: scenario.name,
        description: scenario.description,
        version: scenario.current_version,
        versions: version_entries,
        author: author.username,
        author_email: Some(author.email),
        tags: scenario.tags,
        category: scenario.category,
        downloads: scenario.downloads_total as u64,
        rating: scenario.rating_avg.to_string().parse::<f64>().unwrap_or(0.0),
        reviews_count: scenario.rating_count as u32,
        reviews: review_responses,
        repository: scenario.repository,
        homepage: scenario.homepage,
        license: scenario.license,
        created_at: scenario.created_at.to_rfc3339(),
        updated_at: scenario.updated_at.to_rfc3339(),
    }))
}

/// Get scenario version
pub async fn get_scenario_version(
    State(state): State<AppState>,
    Path((name, version)): Path<(String, String)>,
) -> ApiResult<Json<ScenarioVersionEntry>> {
    let pool = state.db.pool();

    let scenario = Scenario::find_by_name(pool, &name)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::ScenarioNotFound(name.clone()))?;

    let scenario_version = ScenarioVersion::find(pool, scenario.id, &version)
        .await
        .map_err(|e| ApiError::Database(e))?
        .ok_or_else(|| ApiError::InvalidVersion(version.clone()))?;

    Ok(Json(ScenarioVersionEntry {
        version: scenario_version.version,
        download_url: scenario_version.download_url,
        checksum: scenario_version.checksum,
        size: scenario_version.file_size as u64,
        published_at: scenario_version.published_at.to_rfc3339(),
        yanked: scenario_version.yanked,
        min_mockforge_version: scenario_version.min_mockforge_version,
    }))
}

/// Publish a scenario
pub async fn publish_scenario(
    State(state): State<AppState>,
    AuthUser(author_id): AuthUser,
    headers: HeaderMap,
    Json(request): Json<PublishScenarioRequest>,
) -> ApiResult<Json<PublishScenarioResponse>> {
    let metrics = crate::metrics::MarketplaceMetrics::start(state.metrics.clone(), "scenario");
    let pool = state.db.pool();

    // Resolve org context
    let org_ctx = resolve_org_context(&state, author_id, &headers, None)
        .await
        .map_err(|_| ApiError::OrganizationNotFound)?;

    // Check publishing limits
    let limits = &org_ctx.org.limits_json;
    let max_scenarios = limits.get("max_scenarios_published").and_then(|v| v.as_i64()).unwrap_or(1);

    if max_scenarios >= 0 {
        let existing = Scenario::find_by_org(pool, org_ctx.org_id)
            .await
            .map_err(|e| ApiError::Database(e))?;

        if existing.len() as i64 >= max_scenarios {
            return Err(ApiError::InvalidRequest(format!(
                "Scenario limit exceeded. Your plan allows {} scenarios. Upgrade to publish more.",
                max_scenarios
            )));
        }
    }

    // Check storage limit
    let storage_limit_gb = limits.get("storage_gb").and_then(|v| v.as_i64()).unwrap_or(1);
    let storage_limit_bytes = storage_limit_gb * 1_000_000_000;

    let usage = UsageCounter::get_or_create_current(pool, org_ctx.org_id)
        .await
        .map_err(|e| ApiError::Database(e))?;

    let new_storage = usage.storage_bytes + request.size as i64;
    if new_storage > storage_limit_bytes {
        return Err(ApiError::InvalidRequest(format!(
            "Storage limit exceeded. Your plan allows {} GB.",
            storage_limit_gb
        )));
    }

    // Validate checksum format
    crate::validation::validate_checksum(&request.checksum)?;

    // Validate base64 encoding
    crate::validation::validate_base64(&request.package)?;

    // Parse manifest
    let manifest: serde_json::Value = serde_json::from_str(&request.manifest)
        .map_err(|e| ApiError::InvalidRequest(format!("Invalid manifest JSON: {}", e)))?;

    // Extract scenario name and version from manifest for validation
    let name = manifest
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ApiError::InvalidRequest("Manifest must contain 'name' field".to_string()))?
        .to_string();

    let version = manifest
        .get("version")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            ApiError::InvalidRequest("Manifest must contain 'version' field".to_string())
        })?
        .to_string();

    // Validate name and version
    crate::validation::validate_name(&name)?;
    crate::validation::validate_version(&version)?;

    // Decode package data
    let package_data = base64::decode(&request.package)
        .map_err(|e| ApiError::InvalidRequest(format!("Invalid base64: {}", e)))?;

    // Validate package file
    crate::validation::validate_package_file(
        &package_data,
        request.size,
        crate::validation::MAX_SCENARIO_SIZE,
    )?;

    // Verify checksum
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(&package_data);
    let calculated_checksum = hex::encode(hasher.finalize());

    if calculated_checksum != request.checksum {
        return Err(ApiError::InvalidRequest("Checksum mismatch".to_string()));
    }

    // Generate slug from name
    let slug = name
        .as_str()
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .replace("--", "-");

    // Upload to storage
    let download_url = state
        .storage
        .upload_scenario(&name, &version, package_data)
        .await
        .map_err(|e| ApiError::Storage(e.to_string()))?;

    // Create or update scenario
    let scenario = if let Some(existing) =
        Scenario::find_by_name(pool, &name).await.map_err(|e| ApiError::Database(e))?
    {
        // Update existing scenario
        existing
    } else {
        // Create new scenario
        let category = manifest.get("category").and_then(|v| v.as_str()).unwrap_or("other");
        let description = manifest.get("description").and_then(|v| v.as_str()).unwrap_or("");
        let license = manifest.get("license").and_then(|v| v.as_str()).unwrap_or("MIT");

        Scenario::create(
            pool,
            Some(org_ctx.org_id),
            &name,
            &slug,
            description,
            author_id,
            &version,
            category,
            license,
            manifest.clone(),
        )
        .await
        .map_err(|e| ApiError::Database(e))?
    };

    // Create version entry
    let min_mockforge_version = manifest
        .get("compatibility")
        .and_then(|c| c.get("min_version"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    ScenarioVersion::create(
        pool,
        scenario.id,
        &version,
        manifest,
        &download_url,
        &request.checksum,
        request.size as i64,
        min_mockforge_version.as_deref(),
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
        FeatureType::ScenarioPublish,
        Some(serde_json::json!({
            "scenario_name": name,
            "version": version,
        })),
    )
    .await;

    // Record audit event
    let ip_address = headers
        .get("X-Forwarded-For")
        .or_else(|| headers.get("X-Real-IP"))
        .and_then(|h| h.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim());
    let user_agent = headers.get("User-Agent").and_then(|h| h.to_str().ok());

    record_audit_event(
        pool,
        org_ctx.org_id,
        Some(author_id),
        AuditEventType::ScenarioPublished,
        format!("Scenario {} version {} published", name, version),
        Some(serde_json::json!({
            "scenario_name": name,
            "version": version,
        })),
        ip_address,
        user_agent,
    )
    .await;

    // Record metrics
    metrics.record_publish_success();

    Ok(Json(PublishScenarioResponse {
        name: name.to_string(),
        version: version.to_string(),
        download_url,
        published_at: chrono::Utc::now().to_rfc3339(),
    }))
}

// Request/Response types (matching scenario registry client)

#[derive(Debug, Deserialize)]
pub struct ScenarioSearchQuery {
    pub query: Option<String>,
    pub category: Option<String>,
    pub tags: Vec<String>,
    #[serde(default)]
    pub sort: ScenarioSortOrder,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScenarioSortOrder {
    Relevance,
    Downloads,
    Rating,
    Recent,
    Name,
}

impl Default for ScenarioSortOrder {
    fn default() -> Self {
        ScenarioSortOrder::Relevance
    }
}

#[derive(Debug, Serialize)]
pub struct ScenarioSearchResults {
    pub scenarios: Vec<ScenarioRegistryEntry>,
    pub total: usize,
    pub page: usize,
    pub per_page: usize,
}

#[derive(Debug, Serialize)]
pub struct ScenarioRegistryEntry {
    pub name: String,
    pub description: String,
    pub version: String,
    pub versions: Vec<ScenarioVersionEntry>,
    pub author: String,
    pub author_email: Option<String>,
    pub tags: Vec<String>,
    pub category: String,
    pub downloads: u64,
    pub rating: f64,
    pub reviews_count: u32,
    pub reviews: Vec<ScenarioReviewResponse>,
    pub repository: Option<String>,
    pub homepage: Option<String>,
    pub license: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct ScenarioVersionEntry {
    pub version: String,
    pub download_url: String,
    pub checksum: String,
    pub size: u64,
    pub published_at: String,
    pub yanked: bool,
    pub min_mockforge_version: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ScenarioReviewResponse {
    pub id: String,
    pub reviewer: String,
    pub reviewer_email: Option<String>,
    pub rating: u8,
    pub title: Option<String>,
    pub comment: String,
    pub created_at: String,
    pub helpful_count: u32,
    pub verified_purchase: bool,
}

#[derive(Debug, Deserialize)]
pub struct PublishScenarioRequest {
    pub manifest: String, // JSON string
    pub package: String,  // Base64 encoded
    pub checksum: String,
    pub size: u64,
}

#[derive(Debug, Serialize)]
pub struct PublishScenarioResponse {
    pub name: String,
    pub version: String,
    pub download_url: String,
    pub published_at: String,
}
