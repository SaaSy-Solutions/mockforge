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

/// Find the MockForge binary
fn find_mockforge_binary(config: &ServerConfig) -> Result<PathBuf> {
    // If binary path is explicitly provided, use it
    if let Some(binary_path) = &config.binary_path {
        if binary_path.exists() {
            return Ok(binary_path.clone());
        }
        return Err(Error::BinaryNotFound);
    }

    // Try to find mockforge in PATH
    which::which("mockforge")
        .map_err(|_| Error::BinaryNotFound)
        .map(|p| p.to_path_buf())
}

/// Check if a port is available
pub fn is_port_available(port: u16) -> bool {
    use std::net::TcpListener;
    TcpListener::bind(("127.0.0.1", port)).is_ok()
}

/// Find an available port starting from a given port
pub fn find_available_port(start_port: u16) -> Result<u16> {
    for port in start_port..start_port + 100 {
        if is_port_available(port) {
            return Ok(port);
        }
    }
    Err(Error::ConfigError(format!(
        "No available ports found in range {}-{}",
        start_port,
        start_port + 100
    )))
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
    fn test_find_available_port() {
        // Should find a port starting from 30000
        let port = find_available_port(30000).expect("Failed to find available port");
        assert!(port >= 30000);
        assert!(port < 30100);
    }
}
