//! Cloud Runs control-plane endpoints — DX helpers that aren't tied
//! to one specific run kind.
//!
//! Routes:
//!
//! * `POST /api/v1/cloud-runs/data-driven/upload-url` — returns a
//!   presigned PUT URL the UI uses to upload a CSV/JSON test-vector
//!   file directly to Tigris (skipping the registry as a relay), plus
//!   a longer-lived GET URL the user pastes into their suite config
//!   so the runner can fetch the data when the run fires.
//!
//! Per-kind endpoints (bench / conformance / OWASP / ...) ride the
//! existing `/api/v1/test-suites/{id}/runs` trigger flow with payload
//! flags (`use_cloud_api: true`, `kind: "data_driven"`, etc.).

use std::time::Duration;

use axum::{extract::State, http::HeaderMap, Json};
use serde::{Deserialize, Serialize};

use crate::{
    error::{ApiError, ApiResult},
    middleware::{resolve_org_context, AuthUser},
    AppState,
};

/// Default TTL for the PUT URL — five minutes is plenty for a browser
/// upload and short enough that an exfiltrated URL has limited useful
/// life.
const DEFAULT_UPLOAD_TTL_SECS: u64 = 5 * 60;

/// Hard cap on the PUT URL TTL. One hour is the max anyone reasonably
/// needs for a single upload.
const MAX_UPLOAD_TTL_SECS: u64 = 60 * 60;

/// Default TTL for the GET URL — 24 hours covers most "create suite,
/// trigger later that day" workflows.
const DEFAULT_DATA_TTL_SECS: u64 = 24 * 60 * 60;

/// Hard cap on the GET URL TTL. One week is the upper bound — past
/// that, users should regenerate.
const MAX_DATA_TTL_SECS: u64 = 7 * 24 * 60 * 60;

#[derive(Debug, Deserialize)]
pub struct DataDrivenUploadUrlRequest {
    /// File extension hint (`"csv"`, `"json"`, etc.). Sanitized
    /// server-side. Defaults to `"csv"`.
    #[serde(default)]
    pub extension: Option<String>,
    /// PUT URL lifetime in seconds. Defaults to 300, capped at 3600.
    #[serde(default)]
    pub upload_ttl_seconds: Option<u64>,
    /// GET URL lifetime in seconds. Defaults to 86400, capped at 604800.
    #[serde(default)]
    pub data_ttl_seconds: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct DataDrivenUploadUrlResponse {
    /// Presigned PUT URL — upload directly to Tigris (skip the
    /// registry server). Short-lived.
    pub upload_url: String,
    /// Presigned GET URL the user pastes into their suite config as
    /// `data_url`. Longer-lived.
    pub data_url: String,
    /// Tigris object key for reference (e.g. for manual cleanup).
    pub object_key: String,
    pub upload_expires_in_seconds: u64,
    pub data_expires_in_seconds: u64,
}

/// `POST /api/v1/cloud-runs/data-driven/upload-url`
pub async fn data_driven_upload_url(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Json(request): Json<DataDrivenUploadUrlRequest>,
) -> ApiResult<Json<DataDrivenUploadUrlResponse>> {
    let ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".into()))?;

    let upload_ttl = Duration::from_secs(
        request
            .upload_ttl_seconds
            .unwrap_or(DEFAULT_UPLOAD_TTL_SECS)
            .clamp(60, MAX_UPLOAD_TTL_SECS),
    );
    let data_ttl = Duration::from_secs(
        request
            .data_ttl_seconds
            .unwrap_or(DEFAULT_DATA_TTL_SECS)
            .clamp(60, MAX_DATA_TTL_SECS),
    );
    let extension = request.extension.as_deref().unwrap_or("csv");

    let urls = state
        .storage
        .presign_data_driven_upload(ctx.org_id, extension, upload_ttl, data_ttl)
        .await
        .map_err(|e| {
            // Local-storage backends explicitly don't support presigning.
            // Surface the reason cleanly so the UI can tell the user
            // "this only works in cloud mode" instead of a 500.
            ApiError::InvalidRequest(format!("upload URL generation failed: {}", e))
        })?;

    Ok(Json(DataDrivenUploadUrlResponse {
        upload_url: urls.upload_url,
        data_url: urls.data_url,
        object_key: urls.object_key,
        upload_expires_in_seconds: urls.upload_expires_in_seconds,
        data_expires_in_seconds: urls.data_expires_in_seconds,
    }))
}

// Compile-time invariants on the TTL policy. If anyone bumps the
// constants out of order, the build fails before tests even run.
const _: () = assert!(DEFAULT_UPLOAD_TTL_SECS <= MAX_UPLOAD_TTL_SECS);
const _: () = assert!(DEFAULT_DATA_TTL_SECS <= MAX_DATA_TTL_SECS);
const _: () = assert!(MAX_UPLOAD_TTL_SECS <= MAX_DATA_TTL_SECS);
const _: () = assert!(DEFAULT_UPLOAD_TTL_SECS >= 60);

#[cfg(test)]
mod tests {
    use super::*;

    fn clamp_upload(input: Option<u64>) -> u64 {
        input.unwrap_or(DEFAULT_UPLOAD_TTL_SECS).clamp(60, MAX_UPLOAD_TTL_SECS)
    }

    fn clamp_data(input: Option<u64>) -> u64 {
        input.unwrap_or(DEFAULT_DATA_TTL_SECS).clamp(60, MAX_DATA_TTL_SECS)
    }

    #[test]
    fn upload_ttl_default() {
        assert_eq!(clamp_upload(None), DEFAULT_UPLOAD_TTL_SECS);
    }

    #[test]
    fn upload_ttl_clamps_low() {
        assert_eq!(clamp_upload(Some(1)), 60);
    }

    #[test]
    fn upload_ttl_clamps_high() {
        assert_eq!(clamp_upload(Some(u64::MAX)), MAX_UPLOAD_TTL_SECS);
    }

    #[test]
    fn upload_ttl_honors_intermediate() {
        assert_eq!(clamp_upload(Some(900)), 900);
    }

    #[test]
    fn data_ttl_default() {
        assert_eq!(clamp_data(None), DEFAULT_DATA_TTL_SECS);
    }

    #[test]
    fn data_ttl_clamps_high() {
        assert_eq!(clamp_data(Some(u64::MAX)), MAX_DATA_TTL_SECS);
    }
}
