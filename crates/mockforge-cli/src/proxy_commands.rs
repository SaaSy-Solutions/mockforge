//! First-class HTTP proxy and passive contract validation (#864).

use anyhow::{bail, Context, Result};
use axum::{extract::Query, routing::get, Json, Router};
use clap::Args;
use mockforge_foundation::conformance_violations;
use mockforge_proxy::{conformance::ConformanceTap, server::ProxyServer, ProxyConfig};
use serde::Deserialize;
use std::{net::IpAddr, path::PathBuf};

#[derive(Debug, Args)]
pub struct ProxyArgs {
    /// Upstream HTTP(S) URL (also accepts --target)
    #[arg(long, alias = "target")]
    upstream: url::Url,
    /// Address to listen on
    #[arg(long, default_value = "127.0.0.1")]
    host: IpAddr,
    /// Proxy listen port
    #[arg(long, default_value_t = 9081)]
    port: u16,
    /// OpenAPI/Swagger document used for conformance validation
    #[arg(long, env = "MOCKFORGE_PROXY_SPEC")]
    spec: Option<PathBuf>,
    /// Record request and response violations while forwarding traffic
    #[arg(long, env = "MOCKFORGE_PROXY_VALIDATE_CONFORMANCE")]
    validate_conformance: bool,
    /// Reject invalid requests and return 502 for invalid upstream responses
    #[arg(long, env = "MOCKFORGE_PROXY_VALIDATE_CONFORMANCE_STRICT")]
    validate_conformance_strict: bool,
}

fn build_router(args: &ProxyArgs, spec: Option<serde_json::Value>) -> Result<Router> {
    if !matches!(args.upstream.scheme(), "http" | "https")
        || args.upstream.host_str().is_none()
        || !args.upstream.username().is_empty()
        || args.upstream.password().is_some()
        || args.upstream.fragment().is_some()
    {
        bail!("--upstream must be an HTTP(S) URL without embedded credentials or a fragment");
    }
    let enabled = args.validate_conformance || args.validate_conformance_strict;
    let tap = if enabled {
        let spec =
            spec.context("Conformance validation requires --spec or MOCKFORGE_PROXY_SPEC")?;
        Some(
            ConformanceTap::from_spec_value(spec, args.validate_conformance_strict)
                .map_err(anyhow::Error::msg)?,
        )
    } else {
        None
    };
    let mut config = ProxyConfig::new(args.upstream.to_string());
    config.prefix = None;
    // Redirect responses are part of the upstream contract, not another request.
    config.follow_redirects = false;
    let mut router = ProxyServer::with_conformance(config, false, false, tap).router();
    if enabled {
        router = router.route(
            "/__mockforge/api/conformance/violations",
            get(violations).delete(clear_violations),
        );
    }
    Ok(router)
}

#[derive(Default, Deserialize)]
struct ListQuery {
    limit: Option<usize>,
}

async fn violations(Query(query): Query<ListQuery>) -> Json<serde_json::Value> {
    let mut rows = conformance_violations::snapshot();
    let total = rows.len();
    if let Some(limit) = query.limit {
        rows.truncate(limit);
    }
    Json(serde_json::json!({"violations": rows, "total": total}))
}

async fn clear_violations() -> Json<serde_json::Value> {
    let before = conformance_violations::len();
    conformance_violations::clear();
    Json(serde_json::json!({"cleared": before}))
}

pub async fn run(args: ProxyArgs) -> Result<()> {
    let spec = if let Some(path) = &args.spec {
        let raw = tokio::fs::read_to_string(path).await.context("Cannot read proxy spec")?;
        Some(
            serde_json::from_str(&raw)
                .or_else(|_| serde_yaml::from_str(&raw))
                .context("Cannot parse proxy spec as JSON or YAML")?,
        )
    } else {
        None
    };
    let router = build_router(&args, spec)?;
    let listener = tokio::net::TcpListener::bind((args.host, args.port))
        .await
        .context("Cannot bind proxy listener")?;
    println!("Proxy listening on {}", listener.local_addr()?);
    axum::serve(listener, router).with_graceful_shutdown(shutdown()).await?;
    Ok(())
}

async fn shutdown() {
    #[cfg(unix)]
    {
        let terminate = async {
            match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
                Ok(mut signal) => {
                    signal.recv().await;
                }
                Err(error) => {
                    tracing::error!(%error, "Cannot listen for SIGTERM");
                    std::future::pending::<()>().await;
                }
            }
        };
        tokio::select! {
            result = tokio::signal::ctrl_c() => {
                if let Err(error) = result { tracing::error!(%error, "Cannot listen for Ctrl-C"); }
            }
            () = terminate => {}
        }
    }
    #[cfg(not(unix))]
    if let Err(error) = tokio::signal::ctrl_c().await {
        tracing::error!(%error, "Cannot listen for Ctrl-C");
    }
}

#[cfg(test)]
mod tests;
