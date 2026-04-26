//! Time-travel runtime API for hosted-mock deployments.
//!
//! Time-travel routes existed on the *admin* server (port 9080) via
//! `mockforge-ui::time_travel_handlers`. Hosted-mock Fly machines only
//! expose port 3000 publicly, so the admin server's routes were
//! unreachable from outside the container — operators couldn't set or
//! advance virtual time on a deployed mock without redeploying.
//!
//! This module mirrors the same surface on the main HTTP app. Handlers
//! talk to a local `OnceLock<Arc<TimeTravelManager>>`; serve.rs
//! initialises it alongside the existing admin-server init so both
//! paths see the same manager (the inner `Arc` is shared).
//!
//! ## Endpoints (mounted under `/__mockforge/time-travel`)
//!
//! - `GET    /status            → current clock status
//! - `POST   /enable            → start virtual clock at given time
//! - `POST   /disable           → stop virtual clock
//! - `POST   /advance           → advance virtual time by a duration
//! - `POST   /set               → set virtual time to a specific instant
//! - `POST   /scale             → set time scale factor (e.g., 60.0 = 1min/sec)
//! - `POST   /reset             → reset to real time
//!
//! Scheduled responses, scenarios, and cron jobs are intentionally not
//! mirrored here — they're only useful from the admin UI flow that
//! already exists, and adding them here would more than double the
//! surface area for a marginal gain.

use axum::extract::Json as AxumJson;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, Duration, Utc};
use mockforge_core::TimeTravelManager;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::OnceLock;

/// Process-wide handle to the active TimeTravelManager. Set once at
/// server startup (see `init_time_travel_manager`); read by every
/// request handler in this module.
static MANAGER: OnceLock<Arc<TimeTravelManager>> = OnceLock::new();

/// Register a TimeTravelManager for use by the HTTP-port time-travel
/// API. Idempotent — subsequent calls are no-ops.
pub fn init_time_travel_manager(manager: Arc<TimeTravelManager>) {
    let _ = MANAGER.set(manager);
}

fn manager() -> Option<Arc<TimeTravelManager>> {
    MANAGER.get().cloned()
}

#[derive(Debug, Deserialize)]
struct EnableRequest {
    /// Time to anchor at. Defaults to now.
    #[serde(default)]
    time: Option<DateTime<Utc>>,
    /// Optional scale factor — 1.0 = real-time, 60.0 = 1min per real second.
    #[serde(default)]
    scale: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct AdvanceRequest {
    /// e.g. "2h", "30m", "10s", "1d", "1week".
    duration: String,
}

#[derive(Debug, Deserialize)]
struct SetTimeRequest {
    time: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct ScaleRequest {
    scale: f64,
}

fn not_initialised() -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({
            "error": "time_travel_not_initialised",
            "message": "TimeTravelManager hasn't been registered on this server",
        })),
    )
        .into_response()
}

#[derive(Debug, Serialize)]
struct OkResponse<S> {
    success: bool,
    status: S,
}

async fn status_handler() -> Response {
    let Some(m) = manager() else {
        return not_initialised();
    };
    Json(m.clock().status()).into_response()
}

async fn enable_handler(AxumJson(req): AxumJson<EnableRequest>) -> Response {
    let Some(m) = manager() else {
        return not_initialised();
    };
    let time = req.time.unwrap_or_else(Utc::now);
    m.enable_and_set(time);
    if let Some(scale) = req.scale {
        m.set_scale(scale);
    }
    Json(OkResponse {
        success: true,
        status: m.clock().status(),
    })
    .into_response()
}

async fn disable_handler() -> Response {
    let Some(m) = manager() else {
        return not_initialised();
    };
    m.disable();
    Json(OkResponse {
        success: true,
        status: m.clock().status(),
    })
    .into_response()
}

async fn advance_handler(AxumJson(req): AxumJson<AdvanceRequest>) -> Response {
    let Some(m) = manager() else {
        return not_initialised();
    };
    match parse_duration(&req.duration) {
        Ok(dur) => {
            m.advance(dur);
            Json(OkResponse {
                success: true,
                status: m.clock().status(),
            })
            .into_response()
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "invalid_duration",
                "message": e,
            })),
        )
            .into_response(),
    }
}

async fn set_handler(AxumJson(req): AxumJson<SetTimeRequest>) -> Response {
    let Some(m) = manager() else {
        return not_initialised();
    };
    m.clock().set_time(req.time);
    Json(OkResponse {
        success: true,
        status: m.clock().status(),
    })
    .into_response()
}

async fn scale_handler(AxumJson(req): AxumJson<ScaleRequest>) -> Response {
    let Some(m) = manager() else {
        return not_initialised();
    };
    m.set_scale(req.scale);
    Json(OkResponse {
        success: true,
        status: m.clock().status(),
    })
    .into_response()
}

async fn reset_handler() -> Response {
    let Some(m) = manager() else {
        return not_initialised();
    };
    m.clock().reset();
    Json(OkResponse {
        success: true,
        status: m.clock().status(),
    })
    .into_response()
}

/// Parse a duration string like "2h", "30m", "10s", "1d", "1week".
/// Mirrors the admin-server parser; kept here so this module doesn't
/// reach across crates for a 30-line helper.
fn parse_duration(s: &str) -> Result<Duration, String> {
    let s = s.trim().trim_start_matches('+').trim_start_matches('-');
    if s.is_empty() {
        return Err("empty duration".to_string());
    }

    // Multi-character unit suffixes first, longest match wins. Avoids
    // "1ms" being read as "1m" + "s".
    type DurationCtor = fn(i64) -> Duration;
    let units: &[(&str, DurationCtor)] = &[
        ("weeks", |n| Duration::days(n * 7)),
        ("week", |n| Duration::days(n * 7)),
        ("days", Duration::days),
        ("day", Duration::days),
        ("hours", Duration::hours),
        ("hour", Duration::hours),
        ("minutes", Duration::minutes),
        ("minute", Duration::minutes),
        ("seconds", Duration::seconds),
        ("second", Duration::seconds),
        ("ms", Duration::milliseconds),
        ("d", Duration::days),
        ("h", Duration::hours),
        ("m", Duration::minutes),
        ("s", Duration::seconds),
    ];

    for (suffix, ctor) in units {
        if let Some(num_str) = s.strip_suffix(suffix) {
            let num_str = num_str.trim();
            let n: i64 =
                num_str.parse().map_err(|e| format!("invalid number '{}': {}", num_str, e))?;
            return Ok(ctor(n));
        }
    }

    Err(format!("unknown duration suffix in '{}'; expected w/d/h/m/s/ms", s))
}

/// Build the time-travel runtime router. Mount under `/__mockforge/time-travel`.
pub fn time_travel_router() -> Router {
    Router::new()
        .route("/status", get(status_handler))
        .route("/enable", post(enable_handler))
        .route("/disable", post(disable_handler))
        .route("/advance", post(advance_handler))
        .route("/set", post(set_handler))
        .route("/scale", post(scale_handler))
        .route("/reset", post(reset_handler))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_duration_basic_units() {
        assert_eq!(parse_duration("30s").unwrap(), Duration::seconds(30));
        assert_eq!(parse_duration("5m").unwrap(), Duration::minutes(5));
        assert_eq!(parse_duration("2h").unwrap(), Duration::hours(2));
        assert_eq!(parse_duration("3d").unwrap(), Duration::days(3));
    }

    #[test]
    fn parse_duration_weeks() {
        assert_eq!(parse_duration("1week").unwrap(), Duration::days(7));
        assert_eq!(parse_duration("2weeks").unwrap(), Duration::days(14));
    }

    #[test]
    fn parse_duration_milliseconds() {
        assert_eq!(parse_duration("250ms").unwrap(), Duration::milliseconds(250));
    }

    #[test]
    fn parse_duration_relative_prefix() {
        assert_eq!(parse_duration("+1h").unwrap(), Duration::hours(1));
        assert_eq!(parse_duration("-30m").unwrap(), Duration::minutes(30));
    }

    #[test]
    fn parse_duration_rejects_empty() {
        assert!(parse_duration("").is_err());
        assert!(parse_duration("   ").is_err());
    }

    #[test]
    fn parse_duration_rejects_unknown_suffix() {
        assert!(parse_duration("5fortnights").is_err());
        assert!(parse_duration("12").is_err());
    }
}
