//! Unix-socket server loop. Accepts connections from main mockforge,
//! reads newline-delimited JSON requests, dispatches via
//! [`handlers::handle`], writes back the JSON response.
//!
//! [`handlers::handle`]: crate::handlers::handle

use std::path::Path;
use std::path::PathBuf;

use anyhow::{Context, Result};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

use crate::handlers::{handle, HandlerContext};
use crate::protocol::Request;

/// Configuration for the host server.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Filesystem path the Unix socket is bound to. Main mockforge
    /// connects here.
    pub socket_path: PathBuf,
    /// Permissions to set on the socket file after bind. The
    /// production deployment uses `0o660` + a shared group between
    /// the mockforge user and the plugin-host user; the spike uses
    /// `0o666` for simplicity.
    pub socket_mode: u32,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            socket_path: PathBuf::from("/tmp/plugin-host.sock"),
            socket_mode: 0o660,
        }
    }
}

/// Bind the socket, accept connections forever. Returns only on
/// fatal error (the bind itself failing, or shutdown signal).
///
/// Each accepted connection runs in its own Tokio task so a slow
/// client doesn't head-of-line-block other callers.
pub async fn run_server(config: ServerConfig) -> Result<()> {
    // Best-effort cleanup of a stale socket from a previous run.
    // If the previous host crashed the file is still on disk and a
    // fresh bind would fail with EADDRINUSE.
    if let Err(err) = tokio::fs::remove_file(&config.socket_path).await {
        if err.kind() != std::io::ErrorKind::NotFound {
            tracing::warn!(
                path = %config.socket_path.display(),
                error = %err,
                "could not remove stale socket file; bind may fail"
            );
        }
    }

    let listener = UnixListener::bind(&config.socket_path)
        .with_context(|| format!("binding Unix socket at {}", config.socket_path.display()))?;
    set_socket_permissions(&config.socket_path, config.socket_mode)?;

    tracing::info!(
        path = %config.socket_path.display(),
        mode = format!("0o{:o}", config.socket_mode),
        "plugin-host listening"
    );

    let ctx = HandlerContext::new();

    loop {
        let (stream, _addr) = match listener.accept().await {
            Ok(pair) => pair,
            Err(err) => {
                // Transient accept failures (EMFILE, etc.) are
                // worth surfacing but not fatal — keep the loop
                // running so the next client can connect.
                tracing::error!(error = %err, "accept failed; retrying");
                continue;
            }
        };

        let ctx = ctx.clone();
        tokio::spawn(async move {
            if let Err(err) = handle_connection(&ctx, stream).await {
                tracing::warn!(error = %err, "client connection terminated with error");
            }
        });
    }
}

#[cfg(unix)]
fn set_socket_permissions(path: &Path, mode: u32) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let perms = std::fs::Permissions::from_mode(mode);
    std::fs::set_permissions(path, perms)
        .with_context(|| format!("setting {:o} on {}", mode, path.display()))
}

#[cfg(not(unix))]
fn set_socket_permissions(_path: &Path, _mode: u32) -> Result<()> {
    // Unix sockets are Unix-only; this branch is unreachable in
    // practice but keeps the crate buildable on non-Unix targets
    // (e.g., for `cargo check` on a developer's Windows machine).
    Ok(())
}

async fn handle_connection(ctx: &HandlerContext, stream: UnixStream) -> Result<()> {
    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);
    let mut line = String::new();

    loop {
        line.clear();
        let bytes = reader.read_line(&mut line).await.context("reading IPC frame")?;
        if bytes == 0 {
            // Clean EOF — client closed the connection.
            return Ok(());
        }

        let request: Request = match serde_json::from_str(line.trim_end()) {
            Ok(req) => req,
            Err(err) => {
                tracing::warn!(error = %err, raw = %line, "malformed request — closing");
                // We don't have a request id to echo, so we close
                // the connection rather than send a tagged error.
                // Clients should treat dropped connections as a
                // signal to retry with fresh state.
                return Ok(());
            }
        };

        let response = handle(ctx, request).await;
        let mut bytes = serde_json::to_vec(&response).context("serializing response")?;
        bytes.push(b'\n');
        write_half.write_all(&bytes).await.context("writing response")?;
        write_half.flush().await.context("flushing response")?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Response;
    use tokio::io::BufReader as TokioBufReader;
    use uuid::Uuid;

    /// End-to-end: spawn the server in the background, connect over
    /// the Unix socket, send a Health request, get a HealthOk back.
    /// Validates the protocol round-trips through real socket I/O,
    /// which the unit tests in `handlers` and `protocol` don't.
    #[tokio::test]
    async fn server_round_trips_a_health_request() {
        let dir = tempfile::tempdir().unwrap();
        let socket_path = dir.path().join("test.sock");

        let config = ServerConfig {
            socket_path: socket_path.clone(),
            // Tests run under the user's account; 0o660 needs a
            // matching group. 0o600 keeps the test self-contained.
            socket_mode: 0o600,
        };

        let server_handle = tokio::spawn(async move { run_server(config).await });

        // Wait for the socket file to appear; the server binds on
        // its own task so there's a small window where it's not
        // ready yet.
        let mut attempts = 0;
        while !socket_path.exists() && attempts < 50 {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            attempts += 1;
        }
        assert!(socket_path.exists(), "server didn't bind socket");

        let stream = UnixStream::connect(&socket_path).await.unwrap();
        let (read_half, mut write_half) = stream.into_split();
        let mut reader = TokioBufReader::new(read_half);

        let id = Uuid::new_v4();
        let req = Request::Health { id };
        let mut bytes = serde_json::to_vec(&req).unwrap();
        bytes.push(b'\n');
        write_half.write_all(&bytes).await.unwrap();
        write_half.flush().await.unwrap();

        let mut response_line = String::new();
        reader.read_line(&mut response_line).await.unwrap();
        let response: Response = serde_json::from_str(response_line.trim_end()).unwrap();

        match response {
            Response::HealthOk { id: echoed, .. } => assert_eq!(echoed, id),
            other => panic!("expected HealthOk, got {:?}", other),
        }

        // Cleanup: drop the writer to send EOF, then abort the
        // server so the test exits.
        drop(write_half);
        server_handle.abort();
    }

    #[tokio::test]
    async fn malformed_request_closes_connection_without_panic() {
        let dir = tempfile::tempdir().unwrap();
        let socket_path = dir.path().join("test.sock");
        let config = ServerConfig {
            socket_path: socket_path.clone(),
            socket_mode: 0o600,
        };

        let server_handle = tokio::spawn(async move { run_server(config).await });
        let mut attempts = 0;
        while !socket_path.exists() && attempts < 50 {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            attempts += 1;
        }

        let stream = UnixStream::connect(&socket_path).await.unwrap();
        let (read_half, mut write_half) = stream.into_split();
        let mut reader = TokioBufReader::new(read_half);
        write_half.write_all(b"not valid json\n").await.unwrap();
        write_half.flush().await.unwrap();

        // Server should close the connection. A clean EOF from
        // `read_line` returns Ok(0); we'd time out if the server
        // got stuck in a parse loop.
        let mut buf = String::new();
        let bytes =
            tokio::time::timeout(std::time::Duration::from_secs(2), reader.read_line(&mut buf))
                .await
                .unwrap()
                .unwrap();
        assert_eq!(bytes, 0, "expected EOF, got payload {:?}", buf);

        server_handle.abort();
    }
}
