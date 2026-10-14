//! Supervise listener tasks and preserve bounded HTTP shutdown draining.

use std::future::Future;
use std::time::Duration;
use tokio::task::{JoinError, JoinHandle};
use tokio_util::sync::CancellationToken;

type ServerTask = JoinHandle<Result<(), String>>;
type TaskResult = Result<Result<(), String>, JoinError>;

/// Running listeners owned by one invocation of `serve`.
pub(super) struct Listeners {
    pub http: ServerTask,
    pub https: Option<ServerTask>,
    pub websocket: ServerTask,
    pub grpc: ServerTask,
    pub admin: Option<ServerTask>,
    pub metrics: Option<ServerTask>,
    #[cfg(feature = "http")]
    pub health: std::sync::Arc<mockforge_http::HealthManager>,
}

fn listener_result(name: &str, icon: &str, result: TaskResult) -> Option<String> {
    match result {
        Ok(Ok(())) => {
            println!("{icon} {name} stopped gracefully");
            None
        }
        Ok(Err(error)) => {
            eprintln!("❌ {error}");
            Some(error)
        }
        Err(error) => {
            let error = format!("{name} task panicked: {error}");
            eprintln!("❌ {error}");
            Some(error)
        }
    }
}

async fn optional_listener(handle: Option<&mut ServerTask>) -> TaskResult {
    match handle {
        Some(handle) => handle.await,
        None => std::future::pending().await,
    }
}

/// Wait for a listener exit or the signal registered before startup, cancel
/// peers, and give in-flight HTTP responses the existing six-second grace.
pub(super) async fn supervise(
    mut listeners: Listeners,
    shutdown: impl Future<Output = std::io::Result<()>>,
    shutdown_token: CancellationToken,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let result = tokio::select! {
        result = &mut listeners.http => listener_result("HTTP server", "📡", result),
        result = optional_listener(listeners.https.as_mut()) => listener_result("HTTPS server", "🔒", result),
        result = &mut listeners.websocket => listener_result("WebSocket server", "🔌", result),
        result = &mut listeners.grpc => listener_result("gRPC server", "⚡", result),
        result = optional_listener(listeners.admin.as_mut()) => listener_result("Admin UI", "🎛️", result),
        result = optional_listener(listeners.metrics.as_mut()) => listener_result("Metrics server", "📊", result),
        signal = shutdown => {
            match signal {
                Ok(()) => {
                    println!("🛑 Received shutdown signal");
                    #[cfg(feature = "http")]
                    listeners.health.trigger_shutdown().await;
                    None
                }
                Err(error) => Some(format!("Failed to listen for shutdown signals: {error}")),
            }
        }
    };

    println!("👋 Shutting down remaining servers...");
    shutdown_token.cancel();
    let drained = tokio::time::timeout(Duration::from_secs(6), async {
        if !listeners.http.is_finished() {
            let _ = (&mut listeners.http).await;
        }
        if let Some(handle) = listeners.https {
            if !handle.is_finished() {
                let _ = handle.await;
            }
        }
    })
    .await;
    if drained.is_err() {
        tracing::warn!("HTTP shutdown drain timed out after six seconds");
    }
    match result {
        Some(error) => Err(error.into()),
        None => Ok(()),
    }
}
