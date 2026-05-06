//! `mockforge-plugin-egress` binary — the cloud-plugins egress proxy.
//!
//! Reads its allowlist from `MOCKFORGE_PLUGIN_EGRESS_ALLOWLIST`
//! (comma-separated patterns) or the file at
//! `MOCKFORGE_PLUGIN_EGRESS_ALLOWLIST_FILE` (one pattern per line).
//! Bind address comes from `MOCKFORGE_PLUGIN_EGRESS_LISTEN`,
//! defaulting to `127.0.0.1:8125`.
//!
//! In a Phase 2 production deployment, the plugin-host (PR #399/#400)
//! is responsible for keeping this proxy's allowlist in sync with the
//! per-plugin grants. v1 reads a static config; a future PR will
//! add live reload via SIGHUP or an admin socket.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use mockforge_plugin_egress::{run_proxy, HostPolicy, ProxyConfig};

const DEFAULT_LISTEN: &str = "127.0.0.1:8125";

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let listen: SocketAddr = std::env::var("MOCKFORGE_PLUGIN_EGRESS_LISTEN")
        .unwrap_or_else(|_| DEFAULT_LISTEN.to_string())
        .parse()
        .context("parsing MOCKFORGE_PLUGIN_EGRESS_LISTEN")?;

    let patterns = load_allowlist()?;
    let policy = HostPolicy::from_patterns(&patterns).context("compiling egress allowlist")?;
    let config = ProxyConfig::new(listen, Arc::new(policy));

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        listen = %listen,
        patterns = patterns.len(),
        "mockforge-plugin-egress starting"
    );

    tokio::select! {
        result = run_proxy(config) => {
            tracing::error!(?result, "proxy exited unexpectedly");
            result
        }
        _ = shutdown_signal() => {
            tracing::info!("egress proxy received shutdown signal — exiting");
            Ok(())
        }
    }
}

fn load_allowlist() -> Result<Vec<String>> {
    if let Ok(path) = std::env::var("MOCKFORGE_PLUGIN_EGRESS_ALLOWLIST_FILE") {
        return load_allowlist_file(PathBuf::from(path));
    }
    if let Ok(inline) = std::env::var("MOCKFORGE_PLUGIN_EGRESS_ALLOWLIST") {
        return Ok(inline
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect());
    }
    // No allowlist configured — deny-all by default. The proxy
    // will respond 403 to every request, which is the correct
    // behavior when the operator forgets to set the env var.
    tracing::warn!(
        "no allowlist configured (set MOCKFORGE_PLUGIN_EGRESS_ALLOWLIST or _FILE) — deny-all"
    );
    Ok(Vec::new())
}

fn load_allowlist_file(path: PathBuf) -> Result<Vec<String>> {
    let contents = std::fs::read_to_string(&path)
        .with_context(|| format!("reading allowlist file {}", path.display()))?;
    Ok(contents
        .lines()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty() && !s.starts_with('#'))
        .map(|s| s.to_string())
        .collect())
}

fn init_tracing() {
    use tracing_subscriber::fmt::format::FmtSpan;
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_span_events(FmtSpan::CLOSE)
        .try_init();
}

#[cfg(unix)]
async fn shutdown_signal() {
    use tokio::signal::unix::{signal, SignalKind};
    let mut sigterm = signal(SignalKind::terminate()).expect("install SIGTERM handler");
    let mut sigint = signal(SignalKind::interrupt()).expect("install SIGINT handler");
    tokio::select! {
        _ = sigterm.recv() => tracing::info!("received SIGTERM"),
        _ = sigint.recv() => tracing::info!("received SIGINT"),
    }
}

#[cfg(not(unix))]
async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}
