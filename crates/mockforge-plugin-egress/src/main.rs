//! `mockforge-plugin-egress` binary — the cloud-plugins egress proxy.
//!
//! Reads its allowlist from `MOCKFORGE_PLUGIN_EGRESS_ALLOWLIST`
//! (comma-separated patterns) or the file at
//! `MOCKFORGE_PLUGIN_EGRESS_ALLOWLIST_FILE` (one pattern per line).
//! Bind address comes from `MOCKFORGE_PLUGIN_EGRESS_LISTEN`,
//! defaulting to `127.0.0.1:8125`.
//!
//! ## Live reload
//!
//! When `MOCKFORGE_PLUGIN_EGRESS_ALLOWLIST_FILE` is set, sending
//! `SIGHUP` to the process re-reads the file and atomically swaps
//! the active policy. In-flight connections finish under their
//! pre-reload snapshot; new connections see the updated policy
//! immediately. A failed reload (file missing, invalid pattern)
//! is logged and the previous policy stays in place — a typo'd
//! file shouldn't open the gates.

use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::{Context, Result};
use mockforge_plugin_egress::{
    load_policy_from_file, run_proxy, HostPolicy, PolicyHandle, ProxyConfig,
};

const DEFAULT_LISTEN: &str = "127.0.0.1:8125";

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let listen: SocketAddr = std::env::var("MOCKFORGE_PLUGIN_EGRESS_LISTEN")
        .unwrap_or_else(|_| DEFAULT_LISTEN.to_string())
        .parse()
        .context("parsing MOCKFORGE_PLUGIN_EGRESS_LISTEN")?;

    let allowlist_file =
        std::env::var("MOCKFORGE_PLUGIN_EGRESS_ALLOWLIST_FILE").ok().map(PathBuf::from);

    let patterns = load_initial_allowlist(allowlist_file.as_deref())?;
    let policy = HostPolicy::from_patterns(&patterns).context("compiling egress allowlist")?;
    let handle = PolicyHandle::new(policy);
    let config = ProxyConfig::new(listen, handle.clone());

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        listen = %listen,
        patterns = patterns.len(),
        reload_path = allowlist_file.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "(disabled)".into()),
        "mockforge-plugin-egress starting"
    );

    tokio::select! {
        result = run_proxy(config) => {
            tracing::error!(?result, "proxy exited unexpectedly");
            result
        }
        _ = sighup_loop(handle, allowlist_file) => {
            unreachable!("sighup_loop runs forever until shutdown_signal cancels it")
        }
        _ = shutdown_signal() => {
            tracing::info!("egress proxy received shutdown signal — exiting");
            Ok(())
        }
    }
}

/// SIGHUP reload loop. Returns only on cancellation by the
/// outer `select!`. If no allowlist file is configured (so a
/// reload would have nothing to read), `pending()` blocks
/// forever — the `select!` arm is effectively disabled but the
/// shape of the binary stays simple.
#[cfg(unix)]
async fn sighup_loop(handle: PolicyHandle, allowlist_file: Option<PathBuf>) {
    use tokio::signal::unix::{signal, SignalKind};

    let Some(path) = allowlist_file else {
        std::future::pending::<()>().await;
        return;
    };

    let mut sighup = match signal(SignalKind::hangup()) {
        Ok(s) => s,
        Err(err) => {
            tracing::error!(error = %err, "failed to install SIGHUP handler; live reload disabled");
            std::future::pending::<()>().await;
            return;
        }
    };

    while sighup.recv().await.is_some() {
        match load_policy_from_file(&path) {
            Ok(new_policy) => {
                handle.replace(new_policy);
                tracing::info!(path = %path.display(), "egress allowlist reloaded on SIGHUP");
            }
            Err(err) => {
                tracing::error!(
                    error = %err,
                    path = %path.display(),
                    "SIGHUP reload failed; previous policy retained"
                );
            }
        }
    }
}

#[cfg(not(unix))]
async fn sighup_loop(_handle: PolicyHandle, _allowlist_file: Option<PathBuf>) {
    // SIGHUP is Unix-only. Non-Unix builds (developer machines)
    // get no live reload.
    std::future::pending::<()>().await;
}

fn load_initial_allowlist(allowlist_file: Option<&std::path::Path>) -> Result<Vec<String>> {
    if let Some(path) = allowlist_file {
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("reading allowlist file {}", path.display()))?;
        return Ok(contents
            .lines()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty() && !s.starts_with('#'))
            .map(|s| s.to_string())
            .collect());
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
