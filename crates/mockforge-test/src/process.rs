//! Process management for MockForge servers

use crate::config::ServerConfig;
use crate::error::{Error, Result};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use tracing::{debug, info, warn};

/// Managed MockForge process
pub struct ManagedProcess {
    child: Child,
    http_port: u16,
    pid: u32,
}

impl ManagedProcess {
    /// Spawn a new MockForge server process
    pub fn spawn(config: &ServerConfig) -> Result<Self> {
        let binary_path = find_mockforge_binary(config)?;
        debug!("Using MockForge binary at: {:?}", binary_path);

        let mut cmd = Command::new(&binary_path);
        cmd.arg("serve");

        // Configure ports
        cmd.arg("--http-port").arg(config.http_port.to_string());

        if let Some(ws_port) = config.ws_port {
            cmd.arg("--ws-port").arg(ws_port.to_string());
        }

        if let Some(grpc_port) = config.grpc_port {
            cmd.arg("--grpc-port").arg(grpc_port.to_string());
        }

        if let Some(admin_port) = config.admin_port {
            cmd.arg("--admin-port").arg(admin_port.to_string());
        }

        if let Some(metrics_port) = config.metrics_port {
            cmd.arg("--metrics-port").arg(metrics_port.to_string());
        }

        // Configure admin UI
        if config.enable_admin {
            cmd.arg("--admin");
        }

        // Configure metrics
        if config.enable_metrics {
            cmd.arg("--metrics");
        }

        // Configure spec file
        if let Some(spec_file) = &config.spec_file {
            cmd.arg("--spec").arg(spec_file);
        }

        // Configure workspace
        if let Some(workspace_dir) = &config.workspace_dir {
            cmd.arg("--workspace-dir").arg(workspace_dir);
        }

        // Configure profile
        if let Some(profile) = &config.profile {
            cmd.arg("--profile").arg(profile);
        }

        // Add extra arguments
        for arg in &config.extra_args {
            cmd.arg(arg);
        }

        // Set working directory
        if let Some(working_dir) = &config.working_dir {
            cmd.current_dir(working_dir);
        }

        // Set environment variables
        for (key, value) in &config.env_vars {
            cmd.env(key, value);
        }

        // Configure stdio - use inherit() for testing to see actual output
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());

        debug!("Spawning MockForge process: {:?}", cmd);

        let child = cmd
            .spawn()
            .map_err(|e| Error::ServerStartFailed(format!("Failed to spawn process: {}", e)))?;

        let pid = child.id();
        info!("Spawned MockForge process with PID: {}", pid);

        Ok(Self {
            child,
            http_port: config.http_port,
            pid,
        })
    }

    /// Get the HTTP port the server is running on
    pub fn http_port(&self) -> u16 {
        self.http_port
    }

    /// Get the process ID
    pub fn pid(&self) -> u32 {
        self.pid
    }

    /// Check if the process is still running
    pub fn is_running(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    /// Kill the process
    pub fn kill(&mut self) -> Result<()> {
        if self.is_running() {
            debug!("Killing MockForge process (PID: {})", self.pid);
            self.child
                .kill()
                .map_err(|e| Error::ProcessError(format!("Failed to kill process: {}", e)))?;

            // Wait for the process to exit
            let _ = self.child.wait();
            info!("MockForge process (PID: {}) terminated", self.pid);
        } else {
            debug!("Process (PID: {}) already exited", self.pid);
        }
        Ok(())
    }
}

impl Drop for ManagedProcess {
    fn drop(&mut self) {
        if let Err(e) = self.kill() {
            warn!("Failed to kill process on drop: {}", e);
        }
    }
}

/// Find the MockForge binary.
///
/// Resolution order:
/// 1. Explicit `config.binary_path` (when the test set one).
/// 2. `MOCKFORGE_TEST_BINARY` env var (lets `cargo test` point at the
///    freshly-built `target/debug/mockforge` instead of an older
///    `mockforge` on PATH).
/// 3. The workspace's `target/debug/mockforge` and `target/release/mockforge`,
///    if either exists — auto-detected via `CARGO_TARGET_DIR` or by walking
///    up from `CARGO_MANIFEST_DIR`. This makes `cargo test` "just work" on
///    a fresh checkout without having to `cargo install --path`.
/// 4. `mockforge` on `$PATH` as a last resort.
fn find_mockforge_binary(config: &ServerConfig) -> Result<PathBuf> {
    if let Some(binary_path) = &config.binary_path {
        if binary_path.exists() {
            return Ok(binary_path.clone());
        }
        return Err(Error::BinaryNotFound);
    }

    if let Ok(env_path) = std::env::var("MOCKFORGE_TEST_BINARY") {
        let p = PathBuf::from(env_path);
        if p.exists() {
            return Ok(p);
        }
    }

    if let Some(p) = workspace_target_binary() {
        return Ok(p);
    }

    which::which("mockforge")
        .map_err(|_| Error::BinaryNotFound)
        .map(|p| p.to_path_buf())
}

/// Look for a freshly-built `mockforge` binary under the workspace's
/// `target/{debug,release}` directory, preferring debug. Returns `None`
/// when neither candidate exists.
///
/// Resolution order for the target dir:
/// 1. `CARGO_TARGET_DIR` env var
/// 2. Walk up from `CARGO_MANIFEST_DIR` looking for a `target/` sibling
///    (set by cargo when the test runs under `cargo test`)
/// 3. Walk up from `std::env::current_exe()` — this lets us locate the
///    workspace target even when the test binary is invoked directly
///    (e.g. by a debugger), so we don't silently fall through to a
///    stale `mockforge` on `$PATH` whose schema may differ from the
///    workspace.
fn workspace_target_binary() -> Option<PathBuf> {
    let target_dir = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .or_else(target_dir_from_manifest)
        .or_else(target_dir_from_current_exe)?;

    let debug = target_dir.join("debug").join("mockforge");
    if debug.exists() {
        return Some(debug);
    }
    let release = target_dir.join("release").join("mockforge");
    if release.exists() {
        return Some(release);
    }
    None
}

fn target_dir_from_manifest() -> Option<PathBuf> {
    let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR").map(PathBuf::from)?;
    let mut dir: &std::path::Path = &manifest_dir;
    loop {
        let candidate = dir.join("target");
        if candidate.is_dir() {
            return Some(candidate);
        }
        dir = dir.parent()?;
    }
}

fn target_dir_from_current_exe() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let mut dir = exe.parent()?;
    loop {
        if dir.file_name() == Some(std::ffi::OsStr::new("target")) {
            return Some(dir.to_path_buf());
        }
        dir = dir.parent()?;
    }
}

/// Check if a port is available
pub fn is_port_available(port: u16) -> bool {
    use std::net::TcpListener;
    TcpListener::bind(("127.0.0.1", port)).is_ok()
}

/// Find an available port.
///
/// Always asks the OS for an ephemeral port (binds to `127.0.0.1:0` and
/// reads back the assigned port). The `start_port` argument is accepted
/// for API compatibility but ignored — the old "sequentially probe
/// `start_port..start_port+100`" strategy raced badly under parallel
/// nextest runs, where two tests would both see the same port as free
/// and both subprocesses would try to bind it. On macOS the loser's
/// `bind` returns `EADDRINUSE` (errno 48) and the losing subprocess's
/// `tokio::select!` over its admin handle would then short-circuit the
/// entire mock server, taking HTTP + WS down with it. The test's
/// `connect_async` saw `Connection refused` and failed.
///
/// The OS-assigned ephemeral range is ~30k ports wide, so the same race
/// is still technically possible but astronomically unlikely in practice.
///
/// A tiny TOCTOU window remains — the listener drops before the caller's
/// subprocess binds the same port — but in practice this hasn't been
/// observed because the ephemeral range is large and no other process is
/// typically aggressively snapping ports in that window.
pub fn find_available_port(_start_port: u16) -> Result<u16> {
    use std::net::TcpListener;
    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|e| Error::ConfigError(format!("OS-assigned port bind failed: {}", e)))?;
    let port = listener
        .local_addr()
        .map_err(|e| Error::ConfigError(format!("Failed to read OS-assigned port: {}", e)))?
        .port();
    drop(listener);
    Ok(port)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_port_available() {
        // Port 0 should always be available (it means "assign any port")
        assert!(is_port_available(0));
    }

    #[test]
    fn test_find_available_port_returns_nonzero() {
        // `find_available_port` always asks the OS for an ephemeral port.
        // The `start_port` hint is ignored (see the function's doc comment
        // for why), so the assigned port is from the OS ephemeral range
        // rather than the argument's range.
        let port = find_available_port(30000).expect("Failed to find available port");
        assert!(port > 0);
    }

    #[test]
    fn test_find_available_port_ignores_hint() {
        // Two calls with the same hint should hand back distinct ports
        // (ephemeral allocation is effectively never reused back-to-back).
        let port1 = find_available_port(30000).expect("Failed to find port 1");
        let port2 = find_available_port(30000).expect("Failed to find port 2");
        assert_ne!(port1, port2, "ephemeral allocator handed back the same port twice");
    }

    #[test]
    fn test_is_port_available_high_port() {
        // High ports are usually available
        let available = is_port_available(59999);
        // This might be true or false depending on system state
        // Just ensure it doesn't panic
        let _ = available;
    }

    #[test]
    fn test_multiple_port_allocations() {
        // Get three ports; they should all be nonzero and distinct.
        let port1 = find_available_port(31000).expect("Failed to find port 1");
        let port2 = find_available_port(32000).expect("Failed to find port 2");
        let port3 = find_available_port(33000).expect("Failed to find port 3");

        assert!(port1 > 0);
        assert!(port2 > 0);
        assert!(port3 > 0);
        assert_ne!(port1, port2);
        assert_ne!(port1, port3);
        assert_ne!(port2, port3);
    }
}
