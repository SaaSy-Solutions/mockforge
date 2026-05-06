//! `mockforge-plugin-host` binary — the cloud-plugins sidecar.
//!
//! Reads `MOCKFORGE_PLUGIN_HOST_SOCKET` for the bind path (default
//! `/tmp/plugin-host.sock`) and `MOCKFORGE_PLUGIN_HOST_SOCKET_MODE`
//! for the file mode (default `0o660`). Logs to stdout with the
//! standard `tracing` subscriber so Fly's log aggregation picks it
//! up alongside main mockforge.
//!
//! See the crate-level docs for the design.

use std::path::PathBuf;

use anyhow::{Context, Result};
use mockforge_plugin_host::{run_server, ServerConfig};

const DEFAULT_SOCKET_PATH: &str = "/tmp/plugin-host.sock";
const DEFAULT_SOCKET_MODE: u32 = 0o660;

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let config = ServerConfig {
        socket_path: env_socket_path()?,
        socket_mode: env_socket_mode()?,
    };

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        socket = %config.socket_path.display(),
        "mockforge-plugin-host starting"
    );

    // Run the server until SIGTERM. The server loop returns only on
    // bind error or when the runtime is shut down.
    tokio::select! {
        result = run_server(config) => {
            tracing::error!(?result, "plugin-host server loop exited unexpectedly");
            result
        }
        _ = shutdown_signal() => {
            tracing::info!("plugin-host received shutdown signal — exiting");
            Ok(())
        }
    }
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
