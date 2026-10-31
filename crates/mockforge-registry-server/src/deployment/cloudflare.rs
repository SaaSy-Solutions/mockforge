//! Cloudflare Containers backend for hosted mocks.
//!
//! HTTP, WebSocket and GraphQL mocks run as Cloudflare Containers behind the
//! `mockforge-hosted-mocks` Worker (`deploy/cloudflare/hosted-mocks`). The
//! Worker serves `<public base>/<org_id>/<slug>/...`, starts the mock's
//! container on the first request, and lets it sleep when idle, so an unused
//! mock costs nothing. On start the Worker asks the registry for the mock's
//! environment (`GET /api/v1/internal/hosted-mocks/runtime/{org}/{slug}`) and
//! reports request counts back (`POST .../usage`).
//!
//! Other protocols need raw TCP, which Cloudflare Containers cannot accept
//! from the internet, so those mocks stay on Fly.

use anyhow::{Context, Result};
use std::time::Duration;

use crate::models::{HostedMock, Protocol};

/// `metadata_json` key recording which backend serves a deployment.
pub const BACKEND_KEY: &str = "backend";
/// `metadata_json[BACKEND_KEY]` value for Cloudflare-served deployments.
pub const BACKEND_CLOUDFLARE: &str = "cloudflare";
/// Header carrying the shared secret on registry-to-Worker admin calls.
pub const EDGE_TOKEN_HEADER: &str = "x-mockforge-edge-token";

/// Edge requests cannot participate in the registry's atomic Free allowance.
/// Re-check effective subscription state even for an already deployed container.
pub(crate) async fn require_paid_plan(
    pool: &sqlx::PgPool,
    org_id: uuid::Uuid,
) -> crate::error::ApiResult<()> {
    let org = crate::models::Organization::find_by_id(pool, org_id)
        .await?
        .ok_or(crate::error::ApiError::PermissionDenied)?;
    let subscription = crate::models::Subscription::find_by_org(pool, org_id).await?;
    if crate::handlers::entitlements::resolve_effective_plan(
        org.plan(),
        subscription.as_ref(),
        chrono::Utc::now(),
    ) == crate::models::Plan::Free
    {
        return Err(crate::error::ApiError::ResourceLimitExceeded("External hosting requires a paid plan; create a local Free mock or restore the subscription".into()));
    }
    Ok(())
}

/// Client for the hosted-mocks Worker's admin endpoints.
#[derive(Clone)]
pub struct CloudflareMocks {
    public_base: String,
    edge_token: String,
    http: reqwest::Client,
}

impl CloudflareMocks {
    /// Configured from `MOCKFORGE_CF_MOCKS_URL` (the Worker's public base,
    /// e.g. `https://mocks.mockforge.dev`) and `MOCKFORGE_CF_MOCKS_EDGE_TOKEN`.
    /// Returns `None` unless both are set, which leaves every mock on Fly.
    pub fn from_env() -> Option<Self> {
        let base = std::env::var("MOCKFORGE_CF_MOCKS_URL").ok()?;
        let token = std::env::var("MOCKFORGE_CF_MOCKS_EDGE_TOKEN").ok()?;
        Self::new(&base, &token)
    }

    pub fn new(public_base: &str, edge_token: &str) -> Option<Self> {
        let public_base = public_base.trim().trim_end_matches('/');
        if public_base.is_empty() || edge_token.trim().is_empty() {
            return None;
        }
        Some(Self {
            public_base: public_base.to_string(),
            edge_token: edge_token.to_string(),
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .ok()?,
        })
    }

    /// Public URL a deployment is served from.
    pub fn public_url(&self, deployment: &HostedMock) -> String {
        format!("{}/{}/{}", self.public_base, deployment.org_id, deployment.slug)
    }

    /// Stop the deployment's running container, if any, so the next request
    /// starts it with the current configuration.
    pub async fn restart(&self, deployment: &HostedMock) -> Result<()> {
        self.admin(deployment, "restart").await
    }

    /// Stop the deployment's container and drop its cached configuration.
    pub async fn destroy(&self, deployment: &HostedMock) -> Result<()> {
        self.admin(deployment, "destroy").await
    }

    async fn admin(&self, deployment: &HostedMock, action: &str) -> Result<()> {
        let url = format!(
            "{}/__mockforge-edge/{}/{}/{}",
            self.public_base, deployment.org_id, deployment.slug, action
        );
        let response = self
            .http
            .post(&url)
            .header(EDGE_TOKEN_HEADER, &self.edge_token)
            .send()
            .await
            .with_context(|| format!("Cloudflare {action} request failed"))?;
        if !response.status().is_success() {
            anyhow::bail!("Cloudflare {action} returned {}", response.status());
        }
        Ok(())
    }
}

/// Whether a deployment can be served from Cloudflare Containers: only
/// protocols that arrive as HTTP (plain HTTP, WebSocket upgrades, GraphQL) and
/// no plugins, which need the separate cloud-plugins image.
pub fn runs_on_cloudflare(deployment: &HostedMock, plugins_attached: bool) -> bool {
    !plugins_attached
        && deployment
            .enabled_protocols()
            .iter()
            .all(|p| matches!(p, Protocol::Http | Protocol::WebSocket | Protocol::GraphQL))
}

/// Whether a deployment is currently served from Cloudflare.
pub fn is_cloudflare(deployment: &HostedMock) -> bool {
    deployment.metadata_json.get(BACKEND_KEY).and_then(|v| v.as_str()) == Some(BACKEND_CLOUDFLARE)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use serde_json::json;
    use uuid::Uuid;

    pub(crate) fn deployment(protocols: &[&str], metadata: serde_json::Value) -> HostedMock {
        let now = chrono::Utc::now();
        HostedMock {
            id: Uuid::new_v4(),
            org_id: Uuid::new_v4(),
            project_id: None,
            name: "demo".into(),
            slug: "shop".into(),
            description: None,
            config_json: json!({ "enabled_protocols": protocols }),
            openapi_spec_url: None,
            status: "active".into(),
            deployment_url: None,
            internal_url: None,
            region: "iad".into(),
            instance_type: "shared-cpu-1x".into(),
            health_check_url: None,
            last_health_check: None,
            health_status: "unknown".into(),
            error_message: None,
            metadata_json: metadata,
            created_at: now,
            updated_at: now,
            deleted_at: None,
        }
    }

    #[test]
    fn http_websocket_and_graphql_mocks_run_on_cloudflare() {
        for protocols in [&[][..], &["http"], &["http", "ws", "gql"]] {
            assert!(runs_on_cloudflare(&deployment(protocols, json!({})), false), "{protocols:?}");
        }
    }

    #[test]
    fn raw_tcp_protocols_and_plugins_stay_on_fly() {
        for protocol in ["grpc", "kafka", "mqtt", "amqp", "smtp", "tcp"] {
            assert!(
                !runs_on_cloudflare(&deployment(&["http", protocol], json!({})), false),
                "{protocol}"
            );
        }
        assert!(!runs_on_cloudflare(&deployment(&["http"], json!({})), true));
    }

    #[test]
    fn backend_is_read_from_metadata() {
        assert!(is_cloudflare(&deployment(&[], json!({ "backend": "cloudflare" }))));
        assert!(!is_cloudflare(&deployment(&[], json!({ "flyio_machine_id": "m1" }))));
        assert!(!is_cloudflare(&deployment(&[], json!(null))));
    }

    #[test]
    fn public_url_is_org_then_slug_under_the_worker_base() {
        let cf = CloudflareMocks::new("https://mocks.mockforge.dev/", "edge-secret").unwrap();
        let d = deployment(&[], json!({}));
        assert_eq!(cf.public_url(&d), format!("https://mocks.mockforge.dev/{}/shop", d.org_id));
    }

    #[tokio::test]
    async fn admin_redirects_never_forward_the_edge_secret() {
        use axum::{http::StatusCode, routing::any, Router};
        use std::sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        };
        for status in [301, 302, 307, 308] {
            let hits = Arc::new(AtomicUsize::new(0));
            let seen = hits.clone();
            let sink = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let target = format!("http://{}/stolen", sink.local_addr().unwrap());
            let sink_task = tokio::spawn(async move {
                axum::serve(
                    sink,
                    Router::new().fallback(any(move || {
                        let seen = seen.clone();
                        async move {
                            seen.fetch_add(1, Ordering::SeqCst);
                            StatusCode::OK
                        }
                    })),
                )
                .await
                .unwrap();
            });
            let origin_hits = Arc::new(AtomicUsize::new(0));
            let origin_seen = origin_hits.clone();
            let origin = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let base = format!("http://{}", origin.local_addr().unwrap());
            let origin_task = tokio::spawn(async move {
                axum::serve(
                    origin,
                    Router::new().fallback(any(move |headers: axum::http::HeaderMap| {
                        let target = target.clone();
                        let origin_seen = origin_seen.clone();
                        async move {
                            assert_eq!(headers.get(EDGE_TOKEN_HEADER).unwrap(), "edge-secret");
                            origin_seen.fetch_add(1, Ordering::SeqCst);
                            (StatusCode::from_u16(status).unwrap(), [("location", target)])
                        }
                    })),
                )
                .await
                .unwrap();
            });
            let result = CloudflareMocks::new(&base, "edge-secret")
                .unwrap()
                .restart(&deployment(&[], json!({})))
                .await;
            origin_task.abort();
            sink_task.abort();
            assert_eq!(
                origin_hits.load(Ordering::SeqCst),
                1,
                "admin request did not reach its origin"
            );
            assert_eq!(hits.load(Ordering::SeqCst), 0, "secret forwarded for {status}");
            assert!(result.is_err(), "redirect {status} was accepted");
        }
    }

    #[test]
    fn both_settings_are_required() {
        assert!(CloudflareMocks::new("", "edge-secret").is_none());
        assert!(CloudflareMocks::new("https://mocks.mockforge.dev", " ").is_none());
    }
}
