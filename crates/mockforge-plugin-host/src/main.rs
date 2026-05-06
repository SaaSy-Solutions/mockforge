//! `mockforge-plugin-host` binary — the cloud-plugins sidecar.
//!
//! Reads `MOCKFORGE_PLUGIN_HOST_SOCKET` for the bind path (default
//! `/tmp/plugin-host.sock`) and `MOCKFORGE_PLUGIN_HOST_SOCKET_MODE`
//! for the file mode (default `0o660`). Logs to stdout with the
//! standard `tracing` subscriber so Fly's log aggregation picks it
//! up alongside main mockforge.
//!
//! See the crate-level docs for the design.
//!
//! ### Runtime flavor
//!
//! Built on a current-thread Tokio runtime. The plugin sandbox holds
//! a Wasmtime `Store` whose embedded `WasiCtx` is `!Send`, which
//! means the actor task that owns it cannot run on a multi-thread
//! runtime. Every IPC connection from main mockforge is processed
//! on this single thread. At the latency budgets and concurrency
//! we're targeting (≤25 plugins per Team-tier deployment, sub-50 ms
//! invocations) this is plenty.

use std::path::PathBuf;

use anyhow::{Context, Result};
use mockforge_plugin_host::{
    run_exporter, run_poll_loop, run_server, Blocklist, BlocklistConfig, ExporterConfig, Host,
    ServerConfig, SignatureMode, SignatureVerifier, TrustStore,
};
use mockforge_plugin_loader::PluginLoaderConfig;

const DEFAULT_SOCKET_PATH: &str = "/tmp/plugin-host.sock";
const DEFAULT_SOCKET_MODE: u32 = 0o660;

fn main() -> Result<()> {
    init_tracing();

    let config = ServerConfig {
        socket_path: env_socket_path()?,
        socket_mode: env_socket_mode()?,
    };
    let trust_store = env_trust_store()?;
    let signature_mode = env_signature_mode();
    let verifier = SignatureVerifier::new(trust_store, signature_mode);
    let blocklist = Blocklist::new();
    let blocklist_config = env_blocklist_config();
    let exporter_config = env_exporter_config();

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        socket = %config.socket_path.display(),
        signature_mode = ?signature_mode,
        trusted_keys = verifier.trusted_key_count(),
        blocklist_url = blocklist_config.as_ref().map(|c| c.url.as_str()).unwrap_or("(disabled)"),
        exporter_url = exporter_config.as_ref().map(|c| c.url.as_str()).unwrap_or("(disabled)"),
        "mockforge-plugin-host starting"
    );

    // Current-thread runtime — see the module-level note on why
    // this is required (the Wasmtime store inside the actor is
    // !Send, so it can't run on a multi-thread runtime).
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("building Tokio current-thread runtime")?;

    rt.block_on(async move {
        let (host, actor, metrics_bus) =
            Host::new(PluginLoaderConfig::default(), verifier, blocklist.clone());

        // The blocklist poller is Send (just an HTTP client) so it
        // could in principle be tokio::spawn'd, but driving it
        // inline via select! keeps the lifecycle uniform — every
        // long-running task in this binary is on the main task,
        // and shutdown is one drop.
        let poll_future: std::pin::Pin<Box<dyn std::future::Future<Output = ()>>> =
            match blocklist_config {
                Some(cfg) => Box::pin(run_poll_loop(cfg, blocklist)),
                None => Box::pin(std::future::pending()),
            };

        // Same pattern for the metric exporter — Send + 'static
        // (just a reqwest client + a broadcast Receiver) but kept
        // inline for lifecycle uniformity. `(*metrics_bus).clone()`
        // gives the exporter its own subscription handle.
        let exporter_future: std::pin::Pin<Box<dyn std::future::Future<Output = ()>>> =
            match exporter_config {
                Some(cfg) => Box::pin(run_exporter(cfg, (*metrics_bus).clone())),
                None => Box::pin(std::future::pending()),
            };

        tokio::select! {
            _ = actor => {
                tracing::error!("plugin-host actor exited unexpectedly");
                Err(anyhow::anyhow!("plugin-host actor exited"))
            }
            result = run_server(config, host) => {
                tracing::error!(?result, "plugin-host server loop exited unexpectedly");
                result
            }
            _ = poll_future => {
                tracing::error!("blocklist poller exited unexpectedly");
                Err(anyhow::anyhow!("blocklist poller exited"))
            }
            _ = exporter_future => {
                tracing::error!("metric exporter exited unexpectedly");
                Err(anyhow::anyhow!("metric exporter exited"))
            }
            _ = shutdown_signal() => {
                tracing::info!("plugin-host received shutdown signal — exiting");
                Ok(())
            }
        }
    })
}

/// Build an [`ExporterConfig`] from env vars, or `None` if no
/// `MOCKFORGE_PLUGIN_HOST_METRICS_URL` is set. Same shape as the
/// blocklist config (URL + optional bearer + optional interval).
fn env_exporter_config() -> Option<ExporterConfig> {
    let url = std::env::var("MOCKFORGE_PLUGIN_HOST_METRICS_URL").ok()?;
    let mut cfg = ExporterConfig::new(url);
    if let Ok(secs) = std::env::var("MOCKFORGE_PLUGIN_HOST_METRICS_FLUSH_INTERVAL_SECS") {
        if let Ok(n) = secs.parse::<u64>() {
            cfg.flush_interval = std::time::Duration::from_secs(n.max(1));
        }
    }
    if let Ok(qsize) = std::env::var("MOCKFORGE_PLUGIN_HOST_METRICS_QUEUE_SIZE") {
        if let Ok(n) = qsize.parse::<usize>() {
            cfg.max_queue_size = n.max(1);
        }
    }
    if let Ok(token) = std::env::var("MOCKFORGE_PLUGIN_HOST_METRICS_BEARER") {
        if !token.is_empty() {
            cfg.bearer_token = Some(token);
        }
    }
    Some(cfg)
}

/// Build a [`BlocklistConfig`] from env vars, or `None` if no
/// `MOCKFORGE_PLUGIN_HOST_BLOCKLIST_URL` is set. Optional polling
/// keeps self-hosted / dev simple — only cloud production has a
/// blocklist endpoint to point at.
fn env_blocklist_config() -> Option<BlocklistConfig> {
    let url = std::env::var("MOCKFORGE_PLUGIN_HOST_BLOCKLIST_URL").ok()?;
    let mut cfg = BlocklistConfig::new(url);
    if let Ok(secs) = std::env::var("MOCKFORGE_PLUGIN_HOST_BLOCKLIST_INTERVAL_SECS") {
        if let Ok(n) = secs.parse::<u64>() {
            cfg.interval = std::time::Duration::from_secs(n.max(1));
        }
    }
    if let Ok(token) = std::env::var("MOCKFORGE_PLUGIN_HOST_BLOCKLIST_BEARER") {
        if !token.is_empty() {
            cfg.bearer_token = Some(token);
        }
    }
    Some(cfg)
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

fn env_socket_path() -> Result<PathBuf> {
    Ok(std::env::var("MOCKFORGE_PLUGIN_HOST_SOCKET")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_SOCKET_PATH)))
}

fn env_socket_mode() -> Result<u32> {
    match std::env::var("MOCKFORGE_PLUGIN_HOST_SOCKET_MODE") {
        Ok(s) => {
            // Accept "0o660", "660", "0660", "0x1B0" — be lenient
            // to keep deploy configs readable.
            let trimmed = s.trim();
            let (radix, digits) = if let Some(rest) = trimmed.strip_prefix("0o") {
                (8, rest)
            } else if let Some(rest) = trimmed.strip_prefix("0x") {
                (16, rest)
            } else if trimmed.starts_with('0') && trimmed.len() > 1 {
                (8, &trimmed[1..])
            } else {
                (8, trimmed)
            };
            u32::from_str_radix(digits, radix)
                .with_context(|| format!("parsing MOCKFORGE_PLUGIN_HOST_SOCKET_MODE={}", s))
        }
        Err(_) => Ok(DEFAULT_SOCKET_MODE),
    }
}

/// Read the trust store from one of:
///
/// - `MOCKFORGE_PLUGIN_HOST_TRUSTED_KEYS_FILE` (path to JSON file)
/// - `MOCKFORGE_PLUGIN_HOST_TRUSTED_KEYS` (inline JSON)
///
/// or return an empty store if neither is set. The store is a
/// JSON object `{ "publisher-id": "<base64-pubkey>", ... }`.
fn env_trust_store() -> Result<TrustStore> {
    if let Ok(path) = std::env::var("MOCKFORGE_PLUGIN_HOST_TRUSTED_KEYS_FILE") {
        return TrustStore::from_file(std::path::Path::new(&path)).map_err(anyhow::Error::from);
    }
    if let Ok(inline) = std::env::var("MOCKFORGE_PLUGIN_HOST_TRUSTED_KEYS") {
        return TrustStore::from_json_str(&inline).map_err(anyhow::Error::from);
    }
    Ok(TrustStore::new())
}

/// Read `MOCKFORGE_PLUGIN_HOST_SIGNATURE_MODE` (`required` |
/// `optional`). Cloud production sets this to `required` via the
/// orchestrator's env var; self-hosted leaves it unset and gets
/// the default (`optional`).
fn env_signature_mode() -> SignatureMode {
    match std::env::var("MOCKFORGE_PLUGIN_HOST_SIGNATURE_MODE")
        .ok()
        .map(|s| s.to_ascii_lowercase())
        .as_deref()
    {
        Some("required") => SignatureMode::Required,
        Some("optional") => SignatureMode::Optional,
        Some(other) => {
            tracing::warn!(
                value = other,
                "unrecognized MOCKFORGE_PLUGIN_HOST_SIGNATURE_MODE; falling back to Optional"
            );
            SignatureMode::Optional
        }
        None => SignatureMode::Optional,
    }
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
