//! Fixtures management API for hosted-mock deployments.
//!
//! The admin UI calls `/__mockforge/fixtures/*` for list/create/delete/
//! download. Those routes live on the *admin* server (port 9080), which
//! isn't exposed publicly on hosted-mock Fly machines — only port 3000
//! is. So UI calls 404'd against the deployed instance. This module
//! mounts an equivalent surface on the main HTTP app so it's reachable
//! from the cloud-side admin UI through the deployed instance.
//!
//! ## Storage
//!
//! Fixtures are written as JSON files into `MOCKFORGE_FIXTURES_DIR`
//! (default `/app/fixtures`). The startup-time `CustomFixtureLoader`
//! already reads from there, so newly-uploaded fixtures take effect on
//! the next deploy/restart. Live reload is a separate concern (would
//! require rebuilding the OpenAPI registry mid-flight).
//!
//! ## Endpoints (mounted under `/__mockforge/fixtures`)
//!
//! - `GET    /                     → list fixtures
//! - `POST   /                     → create or upsert by name
//! - `GET    /{id}/download        → return raw JSON content
//! - `DELETE /{id}                 → remove
//! - `DELETE /bulk                 → remove many (ids in JSON body)

use axum::extract::{Path as AxumPath, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;

/// Configuration for the fixtures API. Cheap to clone.
#[derive(Clone)]
pub struct FixturesApiState {
    /// Directory where fixture files live. Created on first write.
    pub fixtures_dir: PathBuf,
}

impl FixturesApiState {
    /// Construct from the same env var the startup-time loader reads.
    /// Defaults to `/app/fixtures` (matching the Dockerfile path).
    pub fn from_env() -> Self {
        let dir =
            std::env::var("MOCKFORGE_FIXTURES_DIR").unwrap_or_else(|_| "/app/fixtures".to_string());
        Self {
            fixtures_dir: PathBuf::from(dir),
        }
    }
}

/// Public list-shape entry returned by the fixtures API.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FixtureInfo {
    /// Stable ID — derived from the filename (without `.json`).
    pub id: String,
    /// Human-readable name; usually equals id.
    pub name: String,
    /// HTTP method this fixture targets, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    /// Path the fixture matches, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Free-form description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Tags for organisation.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// File size in bytes.
    pub size_bytes: u64,
    /// Last modified time (Unix seconds).
    pub modified_at: u64,
}

/// JSON body accepted by `POST /__mockforge/fixtures`.
#[derive(Debug, Deserialize)]
pub struct CreateFixturePayload {
    /// Identifier (becomes the filename `{name}.json`). Required.
    pub name: String,
    /// HTTP method this fixture targets.
    #[serde(default)]
    pub method: Option<String>,
    /// Path the fixture matches.
    #[serde(default)]
    pub path: Option<String>,
    /// Free-form description.
    #[serde(default)]
    pub description: Option<String>,
    /// Tags for organisation.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Response body (or arbitrary fixture payload).
    pub content: serde_json::Value,
}

/// Persisted shape on disk. Includes the metadata + content together so
/// the file is self-describing and round-trippable through download.
#[derive(Debug, Serialize, Deserialize)]
struct StoredFixture {
    name: String,
    #[serde(default)]
    method: Option<String>,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    content: serde_json::Value,
}

fn safe_id(name: &str) -> Option<String> {
    // Defence-in-depth path traversal protection. We accept only
    // [a-zA-Z0-9._-]; reject anything that could escape the directory.
    if name.is_empty() || name.len() > 200 {
        return None;
    }
    if name
        .chars()
        .any(|c| !(c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.'))
    {
        return None;
    }
    if name == "." || name == ".." || name.starts_with('.') {
        return None;
    }
    Some(name.to_string())
}

async fn list_handler(State(state): State<FixturesApiState>) -> Response {
    if let Err(e) = fs::create_dir_all(&state.fixtures_dir).await {
        return io_error("create_dir_failed", &e.to_string());
    }
    let mut entries = match fs::read_dir(&state.fixtures_dir).await {
        Ok(e) => e,
        Err(e) => return io_error("read_dir_failed", &e.to_string()),
    };
    let mut out: Vec<FixtureInfo> = Vec::new();
    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let id = match path.file_stem().and_then(|s| s.to_str()) {
            Some(s) => s.to_string(),
            None => continue,
        };
        let metadata = match fs::metadata(&path).await {
            Ok(m) => m,
            Err(_) => continue,
        };
        let modified_at = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let stored: Option<StoredFixture> =
            fs::read_to_string(&path).await.ok().and_then(|s| serde_json::from_str(&s).ok());
        let info = match stored {
            Some(s) => FixtureInfo {
                id: id.clone(),
                name: s.name,
                method: s.method,
                path: s.path,
                description: s.description,
                tags: s.tags,
                size_bytes: metadata.len(),
                modified_at,
            },
            None => FixtureInfo {
                id: id.clone(),
                name: id,
                method: None,
                path: None,
                description: None,
                tags: vec![],
                size_bytes: metadata.len(),
                modified_at,
            },
        };
        out.push(info);
    }
    Json(out).into_response()
}

async fn create_handler(
    State(state): State<FixturesApiState>,
    Json(payload): Json<CreateFixturePayload>,
) -> Response {
    let Some(id) = safe_id(&payload.name) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "invalid_name",
                "message": "Fixture name must match [a-zA-Z0-9._-]{1,200} and not start with '.'",
            })),
        )
            .into_response();
    };
    if let Err(e) = fs::create_dir_all(&state.fixtures_dir).await {
        return io_error("create_dir_failed", &e.to_string());
    }
    let path = state.fixtures_dir.join(format!("{}.json", id));
    let stored = StoredFixture {
        name: payload.name.clone(),
        method: payload.method,
        path: payload.path,
        description: payload.description,
        tags: payload.tags,
        content: payload.content,
    };
    let body = match serde_json::to_string_pretty(&stored) {
        Ok(b) => b,
        Err(e) => return io_error("serialize_failed", &e.to_string()),
    };
    if let Err(e) = fs::write(&path, body).await {
        return io_error("write_failed", &e.to_string());
    }
    let metadata = fs::metadata(&path).await.ok();
    let info = FixtureInfo {
        id,
        name: stored.name,
        method: stored.method,
        path: stored.path,
        description: stored.description,
        tags: stored.tags,
        size_bytes: metadata.as_ref().map(|m| m.len()).unwrap_or(0),
        modified_at: metadata
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0),
    };
    (StatusCode::CREATED, Json(info)).into_response()
}

async fn download_handler(
    State(state): State<FixturesApiState>,
    AxumPath(id): AxumPath<String>,
) -> Response {
    let Some(safe) = safe_id(&id) else {
        return invalid_id();
    };
    let path = state.fixtures_dir.join(format!("{}.json", safe));
    match fs::read_to_string(&path).await {
        Ok(s) => (StatusCode::OK, [(axum::http::header::CONTENT_TYPE, "application/json")], s)
            .into_response(),
        Err(_) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "fixture_not_found",
                "message": format!("No fixture with id '{}'", id),
            })),
        )
            .into_response(),
    }
}

async fn delete_handler(
    State(state): State<FixturesApiState>,
    AxumPath(id): AxumPath<String>,
) -> Response {
    let Some(safe) = safe_id(&id) else {
        return invalid_id();
    };
    let path = state.fixtures_dir.join(format!("{}.json", safe));
    match fs::remove_file(&path).await {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "fixture_not_found",
                "message": format!("No fixture with id '{}'", id),
            })),
        )
            .into_response(),
        Err(e) => io_error("remove_failed", &e.to_string()),
    }
}

#[derive(Debug, Deserialize)]
struct BulkDeletePayload {
    ids: Vec<String>,
}

async fn delete_bulk_handler(
    State(state): State<FixturesApiState>,
    Json(payload): Json<BulkDeletePayload>,
) -> Response {
    let mut deleted = 0usize;
    let mut skipped: Vec<String> = Vec::new();
    for id in payload.ids {
        let Some(safe) = safe_id(&id) else {
            skipped.push(id);
            continue;
        };
        let path = state.fixtures_dir.join(format!("{}.json", safe));
        if fs::remove_file(&path).await.is_ok() {
            deleted += 1;
        } else {
            skipped.push(id);
        }
    }
    Json(serde_json::json!({
        "deleted": deleted,
        "skipped": skipped,
    }))
    .into_response()
}

#[derive(Debug, Deserialize)]
struct DownloadQuery {
    #[serde(default)]
    _format: Option<String>,
}

fn invalid_id() -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({
            "error": "invalid_id",
            "message": "Fixture id must match [a-zA-Z0-9._-]{1,200} and not start with '.'",
        })),
    )
        .into_response()
}

fn io_error(code: &str, msg: &str) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({
            "error": code,
            "message": msg,
        })),
    )
        .into_response()
}

async fn download_with_query_handler(
    state: State<FixturesApiState>,
    id: AxumPath<String>,
    Query(_q): Query<DownloadQuery>,
) -> Response {
    download_handler(state, id).await
}

/// Build the fixtures API router. Mount under `/__mockforge/fixtures`.
pub fn fixtures_api_router(state: FixturesApiState) -> Router {
    Router::new()
        .route("/", get(list_handler).post(create_handler))
        .route("/bulk", delete(delete_bulk_handler))
        .route("/{id}", delete(delete_handler))
        .route("/{id}/download", get(download_with_query_handler))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn state_for(dir: &std::path::Path) -> FixturesApiState {
        FixturesApiState {
            fixtures_dir: dir.to_path_buf(),
        }
    }

    #[test]
    fn safe_id_rejects_traversal() {
        assert!(safe_id("../etc/passwd").is_none());
        assert!(safe_id("hello/world").is_none());
        assert!(safe_id(".hidden").is_none());
        assert!(safe_id("").is_none());
    }

    #[test]
    fn safe_id_accepts_normal_names() {
        assert_eq!(safe_id("user-by-id"), Some("user-by-id".to_string()));
        assert_eq!(safe_id("user_42.v2"), Some("user_42.v2".to_string()));
    }

    #[tokio::test]
    async fn create_then_list_round_trips() {
        let dir = tempdir().unwrap();
        let st = state_for(dir.path());
        let payload = CreateFixturePayload {
            name: "users-list".to_string(),
            method: Some("GET".to_string()),
            path: Some("/users".to_string()),
            description: Some("seed".to_string()),
            tags: vec!["e2e".into()],
            content: serde_json::json!([{"id": 1}]),
        };
        let resp = create_handler(State(st.clone()), Json(payload)).await;
        assert_eq!(resp.status(), StatusCode::CREATED);

        let listed = list_handler(State(st)).await;
        assert_eq!(listed.status(), StatusCode::OK);
        // Body is wrapped — pull bytes and verify name appears.
        let body = axum::body::to_bytes(listed.into_body(), 64 * 1024).await.unwrap();
        let s = std::str::from_utf8(&body).unwrap();
        assert!(s.contains("users-list"));
    }

    #[tokio::test]
    async fn delete_removes_and_subsequent_returns_404() {
        let dir = tempdir().unwrap();
        let st = state_for(dir.path());
        let payload = CreateFixturePayload {
            name: "doomed".to_string(),
            method: None,
            path: None,
            description: None,
            tags: vec![],
            content: serde_json::json!({}),
        };
        let _ = create_handler(State(st.clone()), Json(payload)).await;
        let resp = delete_handler(State(st.clone()), AxumPath("doomed".to_string())).await;
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
        let resp = delete_handler(State(st), AxumPath("doomed".to_string())).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
