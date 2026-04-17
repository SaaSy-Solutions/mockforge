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
    models::{AuditEventType, TokenScope, User},
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

    // Normalize language to lowercase before querying — UI sends display-case
    // values but the column is canonically lowercase. Empty/whitespace is
    // treated as "no filter".
    let language_filter = query.language.as_deref().and_then(|l| {
        let trimmed = l.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_ascii_lowercase())
        }
    });

    // Validate and limit pagination parameters
    let per_page = query.per_page.clamp(1, 100); // Max 100 items per page
    let page = query.page;
    let limit = per_page as i64;
    let offset = (page * per_page) as i64;

    // Search plugins
    let plugins = match state
        .store
        .search_plugins(
            query.query.as_deref(),
            category_str,
            language_filter.as_deref(),
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
            return Err(e.into());
        }
    };

    // Convert to registry entries
    let mut entries = Vec::new();
    for plugin in plugins {
        let tags = state.store.get_plugin_tags(plugin.id).await?;

        let versions = state.store.list_plugin_versions(plugin.id).await?;

        let category = map_category_from_string(&plugin.category);

        // Load versions with dependencies
        let mut version_entries = Vec::new();
        for v in versions {
            let dependencies = state.store.get_plugin_version_dependencies(v.id).await?;

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
            .map_err(ApiError::Database)?
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

        let security_score = derive_security_score(&plugin);
        let language = plugin.language.clone();
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
            security_score,
            language,
            repository: plugin.repository,
            homepage: plugin.homepage,
            license: plugin.license,
            created_at: plugin.created_at.to_rfc3339(),
            updated_at: plugin.updated_at.to_rfc3339(),
        });
    }

    // Count total matching results for pagination metadata
    let total = state
        .store
        .count_search_plugins(
            query.query.as_deref(),
            category_str,
            language_filter.as_deref(),
            &query.tags,
        )
        .await? as usize;

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

    let plugin = state
        .store
        .find_plugin_by_name(&name)
        .await?
        .ok_or_else(|| ApiError::PluginNotFound(name.clone()))?;

    let tags = state.store.get_plugin_tags(plugin.id).await?;

    let versions = state.store.list_plugin_versions(plugin.id).await?;

    let category = map_category_from_string(&plugin.category);

    // Load versions with dependencies
    let mut version_entries = Vec::new();
    for v in versions {
        let dependencies = state.store.get_plugin_version_dependencies(v.id).await?;

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
    let author = state.store.find_user_by_id(plugin.author_id).await?.unwrap_or_else(|| User {
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

    let security_score = derive_security_score(&plugin);
    let language = plugin.language.clone();
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
        security_score,
        language,
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

    let plugin = state
        .store
        .find_plugin_by_name(&name)
        .await?
        .ok_or_else(|| ApiError::PluginNotFound(name.clone()))?;

    let plugin_version = state
        .store
        .find_plugin_version(plugin.id, &version)
        .await?
        .ok_or_else(|| ApiError::InvalidVersion(version.clone()))?;

    // Load dependencies
    let dependencies = state.store.get_plugin_version_dependencies(plugin_version.id).await?;

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
    #[serde(alias = "fileSize")]
    pub file_size: i64,
    #[serde(alias = "wasmData")]
    pub wasm_data: String, // Base64 encoded WASM
    pub dependencies: Option<std::collections::HashMap<String, String>>,
    /// Source language of the plugin (rust/python/javascript/typescript/go/other).
    #[serde(default = "default_plugin_language")]
    pub language: String,
    /// Optional Software Bill of Materials (typically CycloneDX JSON).
    /// Stored verbatim on the plugin version and fed to the vulnerability
    /// scanner. Accepts either JSON object or `null`/omitted.
    #[serde(default)]
    pub sbom: Option<serde_json::Value>,
}

fn default_plugin_language() -> String {
    "rust".to_string()
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
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

    // Check if plugin exists
    let existing = state.store.find_plugin_by_name(&request.name).await?;

    let plugin = if let Some(mut plugin) = existing {
        // Update existing plugin
        plugin.current_version = request.version.clone();
        plugin.description = request.description.clone();
        plugin
    } else {
        // Create new plugin
        state
            .store
            .create_plugin(
                &request.name,
                &request.description,
                &request.version,
                &request.category,
                &request.license,
                request.repository.as_deref(),
                request.homepage.as_deref(),
                author_id,
                &request.language,
            )
            .await?
    };

    // Validate input fields
    crate::validation::validate_name(&request.name)?;
    crate::validation::validate_version(&request.version)?;
    crate::validation::validate_checksum(&request.checksum)?;

    // Validate base64 encoding
    crate::validation::validate_base64(&request.wasm_data)?;

    // Decode WASM data
    use base64::Engine;
    let wasm_bytes = base64::engine::general_purpose::STANDARD
        .decode(&request.wasm_data)
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
    let version = state
        .store
        .create_plugin_version(
            plugin.id,
            &request.version,
            &download_url,
            &request.checksum,
            request.file_size,
            None,
            request.sbom.as_ref(),
        )
        .await?;

    // Add dependencies if provided
    if let Some(deps) = request.dependencies {
        for (dep_name, dep_version) in deps {
            state
                .store
                .add_plugin_version_dependency(version.id, &dep_name, &dep_version)
                .await?;
        }
    }

    // Queue the version for a security scan. The background worker
    // (`workers::plugin_scanner`) drains these rows, re-downloads the
    // artifact, and overwrites the result. Writing `"pending"` here rather
    // than attempting an inline scan keeps publish latency predictable and
    // lets us retry by just re-running the worker.
    let pending_findings = serde_json::json!([
        {
            "severity": "info",
            "category": "other",
            "title": "Automated scan pending",
            "description": "This plugin version is queued for automated security scanning. Results usually appear within a minute."
        }
    ]);
    state
        .store
        .upsert_plugin_security_scan(version.id, "pending", 50, &pending_findings, None)
        .await?;

    // Record audit event
    state
        .store
        .record_audit_event(
            uuid::Uuid::nil(),
            Some(author_id),
            AuditEventType::PluginPublished,
            format!("Plugin {} version {} published", request.name, request.version),
            Some(serde_json::json!({
                "plugin_name": request.name,
                "version": request.version,
                "category": request.category,
            })),
            None,
            None,
        )
        .await;

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

    let plugin = state
        .store
        .find_plugin_by_name(&name)
        .await?
        .ok_or_else(|| ApiError::PluginNotFound(name.clone()))?;

    let plugin_version = state
        .store
        .find_plugin_version(plugin.id, &version)
        .await?
        .ok_or_else(|| ApiError::InvalidVersion(version.clone()))?;

    state.store.yank_plugin_version(plugin_version.id).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": format!("Version {} of {} yanked successfully", version, name)
    })))
}

/// Heuristic security score for a plugin (0-100).
///
/// This is a placeholder until the `SecurityScanner` pipeline is wired to the
/// publish flow and scan results are persisted. It rewards plugins that an
/// admin has explicitly verified, plus modest signals from freshness and
/// community traction.
fn derive_security_score(plugin: &crate::models::Plugin) -> u8 {
    let mut score: i32 = 50;
    if plugin.verified_at.is_some() {
        score += 35;
    }
    let ninety_days_ago = chrono::Utc::now() - chrono::Duration::days(90);
    if plugin.updated_at > ninety_days_ago {
        score += 5;
    }
    if plugin.rating_avg >= 4.0 && plugin.rating_count >= 5 {
        score += 5;
    }
    score.clamp(0, 100) as u8
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityScanResponse {
    pub status: String,
    pub score: u8,
    pub findings: Vec<SecurityFindingDto>,
    pub scanned: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityFindingDto {
    pub severity: String,
    pub category: String,
    pub title: String,
    pub description: String,
}

pub async fn get_plugin_security(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> ApiResult<Json<SecurityScanResponse>> {
    let plugin = state
        .store
        .find_plugin_by_name(&name)
        .await?
        .ok_or_else(|| ApiError::PluginNotFound(name.clone()))?;

    // Prefer persisted scan data when available; fall back to the heuristic
    // score derived from verification + activity if nothing has been written
    // yet (e.g. legacy rows from before the scan table existed).
    if let Some(scan) = state.store.latest_security_scan_for_plugin(plugin.id).await? {
        let status = match scan.status.as_str() {
            "pass" | "warning" | "fail" | "pending" => scan.status.clone(),
            _ => "pending".to_string(),
        };
        let findings: Vec<serde_json::Value> =
            serde_json::from_value(scan.findings).unwrap_or_default();
        let finding_dtos = findings
            .into_iter()
            .filter_map(|f| {
                Some(SecurityFindingDto {
                    severity: map_severity(f.get("severity")?.as_str()?).to_string(),
                    category: map_finding_category(f.get("category")?.as_str()?).to_string(),
                    title: f.get("title")?.as_str()?.to_string(),
                    description: f.get("description")?.as_str()?.to_string(),
                })
            })
            .collect();
        let scanned = scan.status != "pending";
        return Ok(Json(SecurityScanResponse {
            status,
            score: scan.score.clamp(0, 100) as u8,
            findings: finding_dtos,
            scanned,
        }));
    }

    // Fallback for plugins without a persisted scan row.
    let score = derive_security_score(&plugin);
    let status = if score >= 70 {
        "pass"
    } else if score >= 50 {
        "warning"
    } else {
        "fail"
    };
    let findings = if plugin.verified_at.is_some() {
        Vec::new()
    } else {
        vec![SecurityFindingDto {
            severity: "info".to_string(),
            category: "other".to_string(),
            title: "Automated scan pending".to_string(),
            description:
                "This plugin has not yet been processed by the security scanner. The score shown is a heuristic based on verification status and activity."
                    .to_string(),
        }]
    };

    Ok(Json(SecurityScanResponse {
        status: status.to_string(),
        score,
        findings,
        scanned: false,
    }))
}

fn map_severity(s: &str) -> &'static str {
    match s.to_ascii_lowercase().as_str() {
        "critical" => "critical",
        "high" => "high",
        "medium" => "medium",
        "low" => "low",
        _ => "info",
    }
}

fn map_finding_category(s: &str) -> &'static str {
    match s.to_ascii_lowercase().as_str() {
        "malware" => "malware",
        "vulnerable_dependency" | "vulnerabledependency" => "vulnerable_dependency",
        "insecure_coding" | "insecurecoding" => "insecure_coding",
        "data_exfiltration" | "dataexfiltration" => "data_exfiltration",
        "supply_chain" | "supplychain" => "supply_chain",
        "licensing" => "licensing",
        "configuration" => "configuration",
        "obfuscation" => "obfuscation",
        _ => "other",
    }
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
