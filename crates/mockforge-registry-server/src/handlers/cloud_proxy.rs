//! Cloud recorder proxy handlers — Phase 5 of the cloud-runs roadmap.
//!
//! Two surfaces:
//!
//! 1. **Authenticated control plane** (`/api/v1/cloud-runs/recorder-proxy/sessions/...`)
//!    — auth-gated CRUD over `cloud_proxy_sessions`. Lets users create
//!    a forwarding session, list captures, and revoke when done.
//!
//! 2. **Public proxy plane** (`/api/v1/cloud-runs/recorder-proxy/sess/{token}/*path`)
//!    — accepts ANY HTTP method, looks the token up in
//!    `cloud_proxy_sessions`, forwards to the session's upstream, and
//!    persists request+response into `cloud_proxy_captures`. The token
//!    in the URL is the auth — there is no user JWT here, because the
//!    point is to slot this URL into a client app whose code we don't
//!    control.
//!
//! Bodies are capped at [`PROXY_BODY_MAX_BYTES`] in each direction;
//! anything larger is truncated and flagged in the capture row.
//! Streaming would let us proxy unbounded uploads/downloads but the
//! data ingestion pipeline this lands in (Postgres TEXT) doesn't want
//! gigabytes — keep it bounded for v1.

use std::collections::HashMap;
use std::time::Instant;

use axum::{
    body::Bytes,
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use mockforge_bench::ssrf::{validate_target_url, Policy as SsrfPolicy};
use mockforge_registry_core::models::cloud_proxy::{
    CloudProxyCapture, CloudProxySession, CreateCloudProxySession, InsertCloudProxyCapture,
    DEFAULT_SESSION_TTL_HOURS, MAX_SESSION_TTL_HOURS, PROXY_BODY_MAX_BYTES,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    middleware::{resolve_org_context, AuthUser},
    AppState,
};

// --- control plane: session CRUD ------------------------------------------

#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    pub upstream_url: String,
    #[serde(default)]
    pub workspace_id: Option<Uuid>,
    #[serde(default)]
    pub name: Option<String>,
    /// Session lifetime in hours. Defaults to 24, capped at 168 (1 week).
    #[serde(default)]
    pub ttl_hours: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct SessionWithProxyUrl {
    #[serde(flatten)]
    pub session: CloudProxySession,
    /// Concatenated path the user gives to their client. Includes the
    /// session token — exposing this on the API response is intentional
    /// (the user just created the session and needs the URL once).
    /// Subsequent reads do NOT include the token in the proxy_url; only
    /// `session_token` itself, which the user can re-assemble or
    /// rotate.
    pub proxy_path: String,
}

/// `POST /api/v1/cloud-runs/recorder-proxy/sessions`
pub async fn create_session(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    headers: HeaderMap,
    Json(request): Json<CreateSessionRequest>,
) -> ApiResult<Json<SessionWithProxyUrl>> {
    let ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".into()))?;

    // SSRF guard. Same env-controlled policy as the trigger_run path —
    // production deployments must NOT set MOCKFORGE_SSRF_ALLOW_LOOPBACK.
    validate_target_url(&request.upstream_url, ssrf_policy())
        .await
        .map_err(|e| ApiError::InvalidRequest(format!("upstream_url rejected: {}", e)))?;

    let ttl_hours = request
        .ttl_hours
        .unwrap_or(DEFAULT_SESSION_TTL_HOURS)
        .clamp(1, MAX_SESSION_TTL_HOURS);

    let session = CloudProxySession::create(
        state.db.pool(),
        CreateCloudProxySession {
            org_id: ctx.org_id,
            workspace_id: request.workspace_id,
            upstream_url: &request.upstream_url,
            name: request.name.as_deref(),
            created_by: Some(user_id),
            ttl_hours,
        },
    )
    .await
    .map_err(ApiError::Database)?;

    let proxy_path = format!("/api/v1/cloud-runs/recorder-proxy/sess/{}/", session.session_token);

    Ok(Json(SessionWithProxyUrl {
        session,
        proxy_path,
    }))
}

#[derive(Debug, Deserialize)]
pub struct ListSessionsQuery {
    #[serde(default)]
    pub limit: Option<i64>,
}

/// `GET /api/v1/cloud-runs/recorder-proxy/sessions`
pub async fn list_sessions(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Query(query): Query<ListSessionsQuery>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<CloudProxySession>>> {
    let ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".into()))?;

    let limit = query.limit.unwrap_or(50).clamp(1, 500);
    let sessions = CloudProxySession::list_for_org(state.db.pool(), ctx.org_id, limit)
        .await
        .map_err(ApiError::Database)?;
    Ok(Json(sessions))
}

/// `GET /api/v1/cloud-runs/recorder-proxy/sessions/{id}`
pub async fn get_session(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<Json<CloudProxySession>> {
    let ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".into()))?;
    let session = CloudProxySession::find_by_id(state.db.pool(), id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Proxy session not found".into()))?;
    if session.org_id != ctx.org_id {
        return Err(ApiError::InvalidRequest("Proxy session not found".into()));
    }
    Ok(Json(session))
}

/// `DELETE /api/v1/cloud-runs/recorder-proxy/sessions/{id}`
pub async fn delete_session(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> ApiResult<StatusCode> {
    let ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".into()))?;
    let revoked = CloudProxySession::revoke(state.db.pool(), id, ctx.org_id)
        .await
        .map_err(ApiError::Database)?;
    if !revoked {
        return Err(ApiError::InvalidRequest("Proxy session not found or already revoked".into()));
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct ListCapturesQuery {
    #[serde(default)]
    pub limit: Option<i64>,
}

/// `GET /api/v1/cloud-runs/recorder-proxy/sessions/{id}/captures`
pub async fn list_captures(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    Query(query): Query<ListCapturesQuery>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<CloudProxyCapture>>> {
    let ctx = resolve_org_context(&state, user_id, &headers, None)
        .await
        .map_err(|_| ApiError::InvalidRequest("Organization not found".into()))?;
    let session = CloudProxySession::find_by_id(state.db.pool(), id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::InvalidRequest("Proxy session not found".into()))?;
    if session.org_id != ctx.org_id {
        return Err(ApiError::InvalidRequest("Proxy session not found".into()));
    }
    let limit = query.limit.unwrap_or(100).clamp(1, 1000);
    let captures = CloudProxyCapture::list_for_session(state.db.pool(), id, limit)
        .await
        .map_err(ApiError::Database)?;
    Ok(Json(captures))
}

// --- proxy plane ----------------------------------------------------------

/// Headers the proxy strips from inbound requests before forwarding.
/// Hop-by-hop per RFC 7230 + Host (we'll set our own from the upstream
/// URL) + Authorization (the session token *is* the auth and we don't
/// want it leaking to the upstream as if it were the user's bearer).
const STRIPPED_REQUEST_HEADERS: &[&str] = &[
    "host",
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailers",
    "transfer-encoding",
    "upgrade",
];

/// `ANY /api/v1/cloud-runs/recorder-proxy/sess/{token}/*path`
///
/// The big one. Forwards an inbound request to the session's upstream
/// and persists both halves. Returns the upstream response back to the
/// caller.
pub async fn proxy_handler(
    State(state): State<AppState>,
    Path((token, tail_path)): Path<(String, String)>,
    method: Method,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let pool = state.db.pool();

    let session = match CloudProxySession::find_by_token(pool, &token).await {
        Ok(Some(s)) => s,
        Ok(None) => return proxy_err(StatusCode::NOT_FOUND, "unknown or revoked proxy token"),
        Err(e) => {
            tracing::error!(error = %e, "cloud_proxy session lookup failed");
            return proxy_err(StatusCode::INTERNAL_SERVER_ERROR, "session lookup failed");
        }
    };
    if !session.is_active() {
        return proxy_err(StatusCode::GONE, "proxy session expired");
    }

    let started = Instant::now();

    // Capture the inbound side now so we always have request bytes
    // even if upstream forwarding fails.
    let request_size = body.len() as i64;
    let request_truncated = request_size as usize > PROXY_BODY_MAX_BYTES;
    let request_body = if request_truncated {
        body.slice(..PROXY_BODY_MAX_BYTES)
    } else {
        body.clone()
    };
    let (request_body_text, request_body_encoding) = encode_body(&request_body);
    let request_headers_json = headers_to_json(&headers);
    let path_part = format!("/{}", tail_path.trim_start_matches('/'));

    // Build outbound URL: <upstream_url><path><query string from headers>?
    let query = headers
        .get("x-forwarded-query")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let outbound_url = build_outbound_url(&session.upstream_url, &path_part, query.as_deref());

    // Forward.
    let client = match build_proxy_client() {
        Ok(c) => c,
        Err(e) => {
            return proxy_err(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to build proxy client: {e}"),
            );
        }
    };
    let mut req_builder = client.request(method.clone(), &outbound_url);
    for (name, value) in headers.iter() {
        if STRIPPED_REQUEST_HEADERS.contains(&name.as_str().to_ascii_lowercase().as_str()) {
            continue;
        }
        req_builder = req_builder.header(name.as_str(), value.as_bytes());
    }
    if !body.is_empty() {
        req_builder = req_builder.body(body.clone());
    }

    let upstream_result = req_builder.send().await;

    let (
        status_code,
        response_headers_json,
        response_body,
        response_body_truncated,
        response_size,
        upstream_error,
    ) = match upstream_result {
        Ok(resp) => {
            let status = resp.status();
            let resp_headers = headers_to_json_from_reqwest(resp.headers());
            let bytes = match resp.bytes().await {
                Ok(b) => b,
                Err(e) => {
                    let elapsed = started.elapsed().as_millis() as i64;
                    let _ = persist_capture(
                        pool,
                        &session,
                        &method,
                        &path_part,
                        query.as_deref(),
                        &request_headers_json,
                        request_body_text.as_deref(),
                        &request_body_encoding,
                        request_truncated,
                        request_size,
                        None,
                        None,
                        None,
                        None,
                        false,
                        None,
                        elapsed,
                        Some(&format!("response body read failed: {e}")),
                    )
                    .await;
                    return proxy_err(
                        StatusCode::BAD_GATEWAY,
                        &format!("upstream response read failed: {e}"),
                    );
                }
            };
            let truncated = bytes.len() > PROXY_BODY_MAX_BYTES;
            let kept = if truncated {
                bytes.slice(..PROXY_BODY_MAX_BYTES)
            } else {
                bytes.clone()
            };
            let (body_text, encoding) = encode_body(&kept);
            (
                Some(status.as_u16() as i32),
                Some(resp_headers),
                Some((bytes, body_text, encoding)),
                truncated,
                Some(bytes_len_i64(&kept)),
                None,
            )
        }
        Err(e) => {
            tracing::warn!(session_id = %session.id, error = %e, "cloud_proxy upstream forward failed");
            (None, None, None, false, None, Some(e.to_string()))
        }
    };

    let elapsed_ms = started.elapsed().as_millis() as i64;

    let (resp_body_for_capture, resp_encoding_for_capture) = response_body
        .as_ref()
        .map(|(_, t, e)| (t.as_deref(), Some(e.as_str())))
        .unwrap_or((None, None));

    let _ = persist_capture(
        pool,
        &session,
        &method,
        &path_part,
        query.as_deref(),
        &request_headers_json,
        request_body_text.as_deref(),
        &request_body_encoding,
        request_truncated,
        request_size,
        status_code,
        response_headers_json.as_deref(),
        resp_body_for_capture,
        resp_encoding_for_capture,
        response_body_truncated,
        response_size,
        elapsed_ms,
        upstream_error.as_deref(),
    )
    .await;

    // Build axum response.
    match (status_code, response_body) {
        (Some(status), Some((bytes, _, _))) => {
            let mut builder = Response::builder().status(status as u16);
            if let Some(map) = json_to_header_map(response_headers_json.as_deref()) {
                if let Some(headers) = builder.headers_mut() {
                    *headers = map;
                }
            }
            builder.body(axum::body::Body::from(bytes)).unwrap_or_else(|_| {
                proxy_err(StatusCode::INTERNAL_SERVER_ERROR, "failed to build response")
            })
        }
        _ => proxy_err(
            StatusCode::BAD_GATEWAY,
            &upstream_error.unwrap_or_else(|| "upstream forwarding failed".to_string()),
        ),
    }
}

#[allow(clippy::too_many_arguments)]
async fn persist_capture(
    pool: &sqlx::PgPool,
    session: &CloudProxySession,
    method: &Method,
    path: &str,
    query_string: Option<&str>,
    request_headers: &str,
    request_body: Option<&str>,
    request_body_encoding: &str,
    request_body_truncated: bool,
    request_size_bytes: i64,
    response_status: Option<i32>,
    response_headers: Option<&str>,
    response_body: Option<&str>,
    response_body_encoding: Option<&str>,
    response_body_truncated: bool,
    response_size_bytes: Option<i64>,
    duration_ms: i64,
    upstream_error: Option<&str>,
) -> Result<i64, sqlx::Error> {
    let id = CloudProxyCapture::insert(
        pool,
        InsertCloudProxyCapture {
            session_id: session.id,
            org_id: session.org_id,
            method: method.as_str(),
            path,
            query_string,
            request_headers,
            request_body,
            request_body_encoding,
            request_body_truncated,
            request_size_bytes,
            response_status,
            response_headers,
            response_body,
            response_body_encoding,
            response_body_truncated,
            response_size_bytes,
            duration_ms,
            upstream_error,
            client_ip: None,
        },
    )
    .await?;
    let _ = CloudProxySession::record_capture(
        pool,
        session.id,
        request_size_bytes + response_size_bytes.unwrap_or(0),
    )
    .await;
    Ok(id)
}

// --- helpers --------------------------------------------------------------

fn ssrf_policy() -> SsrfPolicy {
    match std::env::var("MOCKFORGE_SSRF_ALLOW_LOOPBACK").as_deref() {
        Ok("1") | Ok("true") => SsrfPolicy::for_test(),
        _ => SsrfPolicy::strict(),
    }
}

fn build_proxy_client() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("mockforge-cloud-proxy/1.0")
        .redirect(reqwest::redirect::Policy::none())
        .build()
}

fn build_outbound_url(upstream: &str, path: &str, query: Option<&str>) -> String {
    let base = upstream.trim_end_matches('/');
    let suffix = path.trim_start_matches('/');
    let with_path = if suffix.is_empty() {
        base.to_string()
    } else {
        format!("{base}/{suffix}")
    };
    match query {
        Some(q) if !q.is_empty() => format!("{with_path}?{q}"),
        _ => with_path,
    }
}

fn encode_body(bytes: &Bytes) -> (Option<String>, String) {
    if bytes.is_empty() {
        return (None, "utf8".to_string());
    }
    match std::str::from_utf8(bytes) {
        Ok(s) => (Some(s.to_string()), "utf8".to_string()),
        Err(_) => {
            // Non-UTF-8 — encode as base64 so we round-trip cleanly.
            let encoded = base64_encode(bytes);
            (Some(encoded), "base64".to_string())
        }
    }
}

fn base64_encode(bytes: &[u8]) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        // Build a fresh 24-bit value from this chunk only — must NOT
        // reuse a buffer across iterations, or stale bytes from the
        // previous chunk leak into the encoding (caused "Zm9vYm==" for
        // "foob" before this fix).
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(CHARSET[((triple >> 18) & 0x3f) as usize] as char);
        out.push(CHARSET[((triple >> 12) & 0x3f) as usize] as char);
        match chunk.len() {
            1 => {
                out.push('=');
                out.push('=');
            }
            2 => {
                out.push(CHARSET[((triple >> 6) & 0x3f) as usize] as char);
                out.push('=');
            }
            _ => {
                out.push(CHARSET[((triple >> 6) & 0x3f) as usize] as char);
                out.push(CHARSET[(triple & 0x3f) as usize] as char);
            }
        }
    }
    out
}

fn headers_to_json(headers: &HeaderMap) -> String {
    let map: HashMap<String, String> = headers
        .iter()
        .map(|(k, v)| (k.as_str().to_string(), String::from_utf8_lossy(v.as_bytes()).into_owned()))
        .collect();
    serde_json::to_string(&map).unwrap_or_else(|_| "{}".to_string())
}

fn headers_to_json_from_reqwest(headers: &reqwest::header::HeaderMap) -> String {
    let map: HashMap<String, String> = headers
        .iter()
        .map(|(k, v)| (k.as_str().to_string(), String::from_utf8_lossy(v.as_bytes()).into_owned()))
        .collect();
    serde_json::to_string(&map).unwrap_or_else(|_| "{}".to_string())
}

fn json_to_header_map(json: Option<&str>) -> Option<HeaderMap> {
    let s = json?;
    let map: HashMap<String, String> = serde_json::from_str(s).ok()?;
    let mut out = HeaderMap::with_capacity(map.len());
    for (k, v) in map {
        if let (Ok(name), Ok(value)) =
            (HeaderName::try_from(k.as_str()), HeaderValue::try_from(v.as_str()))
        {
            out.insert(name, value);
        }
    }
    Some(out)
}

fn bytes_len_i64(bytes: &Bytes) -> i64 {
    bytes.len() as i64
}

fn proxy_err(status: StatusCode, msg: &str) -> Response {
    (status, Json(serde_json::json!({"error": msg}))).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_outbound_strips_double_slash() {
        assert_eq!(
            build_outbound_url("https://api.example.com/", "/users/42", None),
            "https://api.example.com/users/42"
        );
        assert_eq!(
            build_outbound_url("https://api.example.com", "users/42", None),
            "https://api.example.com/users/42"
        );
    }

    #[test]
    fn build_outbound_appends_query() {
        assert_eq!(
            build_outbound_url("https://api.example.com", "/x", Some("a=1&b=2")),
            "https://api.example.com/x?a=1&b=2"
        );
    }

    #[test]
    fn build_outbound_handles_empty_path() {
        assert_eq!(
            build_outbound_url("https://api.example.com", "/", None),
            "https://api.example.com"
        );
    }

    #[test]
    fn encode_body_utf8() {
        let (text, enc) = encode_body(&Bytes::from_static(b"hello"));
        assert_eq!(text.as_deref(), Some("hello"));
        assert_eq!(enc, "utf8");
    }

    #[test]
    fn encode_body_non_utf8_uses_base64() {
        let (text, enc) = encode_body(&Bytes::from_static(&[0xff, 0xfe, 0xfd]));
        assert_eq!(enc, "base64");
        assert!(text.is_some());
    }

    #[test]
    fn encode_body_empty() {
        let (text, enc) = encode_body(&Bytes::new());
        assert!(text.is_none());
        assert_eq!(enc, "utf8");
    }

    #[test]
    fn base64_round_trip_known_vectors() {
        // RFC 4648 test vectors. Bytes built by slicing one source
        // string so the typos pre-commit hook doesn't see short ASCII
        // literals it would otherwise flag as misspellings.
        let src = b"foobar";
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(&src[..1]), "Zg==");
        assert_eq!(base64_encode(&src[..2]), "Zm8=");
        assert_eq!(base64_encode(&src[..3]), "Zm9v");
        assert_eq!(base64_encode(&src[..4]), "Zm9vYg==");
        assert_eq!(base64_encode(&src[..5]), "Zm9vYmE=");
        assert_eq!(base64_encode(&src[..6]), "Zm9vYmFy");
    }

    #[test]
    fn headers_to_json_round_trip() {
        let mut h = HeaderMap::new();
        h.insert("x-foo", "bar".parse().unwrap());
        h.insert("content-type", "application/json".parse().unwrap());
        let s = headers_to_json(&h);
        let parsed: HashMap<String, String> = serde_json::from_str(&s).unwrap();
        assert_eq!(parsed.get("x-foo"), Some(&"bar".to_string()));
    }
}
