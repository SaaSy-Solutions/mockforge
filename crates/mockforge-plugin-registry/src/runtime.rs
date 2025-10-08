//! Multi-language plugin runtime support

use crate::{RegistryError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

/// Plugin runtime language
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum PluginLanguage {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    Ruby,
    Other(String),
}

impl PluginLanguage {
    /// Get runtime executor for this language
    pub fn executor(&self) -> Box<dyn RuntimeExecutor> {
        match self {
            PluginLanguage::Rust => Box::new(RustExecutor),
            PluginLanguage::Python => Box::new(PythonExecutor::default()),
            PluginLanguage::JavaScript | PluginLanguage::TypeScript => {
                Box::new(JavaScriptExecutor::default())
            }
            PluginLanguage::Go => Box::new(GoExecutor),
            PluginLanguage::Ruby => Box::new(RubyExecutor),
            PluginLanguage::Other(_) => Box::new(GenericExecutor),
        }
    }
}

/// Runtime executor trait
pub trait RuntimeExecutor: Send + Sync {
    /// Start the plugin process
    fn start(
        &self,
        plugin_path: &Path,
        config: &RuntimeConfig,
    ) -> Result<Box<dyn RuntimeProcess>>;

    /// Check if runtime is available
    fn is_available(&self) -> bool;

    /// Get runtime version
    fn version(&self) -> Result<String>;

    /// Install plugin dependencies
    fn install_dependencies(&self, plugin_path: &Path) -> Result<()>;
}

/// Running plugin process
pub trait RuntimeProcess: Send + Sync {
    /// Check if process is running
    fn is_running(&mut self) -> bool;

    /// Stop the process
    fn stop(&mut self) -> Result<()>;

    /// Get process ID
    fn pid(&self) -> Option<u32>;

    /// Send message to plugin
    fn send_message(&mut self, message: &[u8]) -> Result<()>;

    /// Receive message from plugin
    fn receive_message(&mut self) -> Result<Vec<u8>>;
}

/// Runtime configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    /// Environment variables
    pub env_vars: HashMap<String, String>,

    /// Working directory
    pub working_dir: Option<PathBuf>,

    /// Arguments to pass to plugin
    pub args: Vec<String>,

    /// Timeout for operations (seconds)
    pub timeout: u64,

    /// Memory limit (MB)
    pub memory_limit: Option<u64>,

    /// CPU limit (cores)
    pub cpu_limit: Option<f32>,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            env_vars: HashMap::new(),
            working_dir: None,
            args: vec![],
            timeout: 30,
            memory_limit: Some(512), // 512MB default
            cpu_limit: None,
        }
    }
}

// ===== Rust Executor =====

struct RustExecutor;

impl RuntimeExecutor for RustExecutor {
    fn start(
        &self,
        plugin_path: &Path,
        config: &RuntimeConfig,
    ) -> Result<Box<dyn RuntimeProcess>> {
        let mut cmd = Command::new(plugin_path);

        cmd.args(&config.args)
            .envs(&config.env_vars)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(dir) = &config.working_dir {
            cmd.current_dir(dir);
        }

        let child = cmd
            .spawn()
            .map_err(|e| RegistryError::Storage(format!("Failed to start Rust plugin: {}", e)))?;

        Ok(Box::new(ProcessWrapper::new(child)))
    }

    fn is_available(&self) -> bool {
        Command::new("rustc")
            .arg("--version")
            .output()
            .is_ok()
    }

    fn version(&self) -> Result<String> {
        let output = Command::new("rustc")
            .arg("--version")
            .output()
            .map_err(|e| RegistryError::Storage(format!("Failed to get rustc version: {}", e)))?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn install_dependencies(&self, plugin_path: &Path) -> Result<()> {
        let output = Command::new("cargo")
            .args(["build", "--release"])
            .current_dir(plugin_path)
            .output()
            .map_err(|e| RegistryError::Storage(format!("Failed to build Rust plugin: {}", e)))?;

        if !output.status.success() {
            return Err(RegistryError::Storage(format!(
                "Rust plugin build failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        Ok(())
    }
}

// ===== Python Executor =====

#[derive(Default)]
struct PythonExecutor {
    python_cmd: String,
}

impl PythonExecutor {
    fn new(python_cmd: String) -> Self {
        Self { python_cmd }
    }
}

impl RuntimeExecutor for PythonExecutor {
    fn start(
        &self,
        plugin_path: &Path,
        config: &RuntimeConfig,
    ) -> Result<Box<dyn RuntimeProcess>> {
        let python_cmd = if self.python_cmd.is_empty() {
            "python3"
        } else {
            &self.python_cmd
        };

        let mut cmd = Command::new(python_cmd);

        cmd.arg(plugin_path)
            .args(&config.args)
            .envs(&config.env_vars)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(dir) = &config.working_dir {
            cmd.current_dir(dir);
        }

        let child = cmd.spawn().map_err(|e| {
            RegistryError::Storage(format!("Failed to start Python plugin: {}", e))
        })?;

        Ok(Box::new(ProcessWrapper::new(child)))
    }

    fn is_available(&self) -> bool {
        Command::new("python3")
            .arg("--version")
            .output()
            .is_ok()
    }

    fn version(&self) -> Result<String> {
        let output = Command::new("python3")
            .arg("--version")
            .output()
            .map_err(|e| RegistryError::Storage(format!("Failed to get Python version: {}", e)))?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn install_dependencies(&self, plugin_path: &Path) -> Result<()> {
        let requirements = plugin_path.join("requirements.txt");

        if requirements.exists() {
            let output = Command::new("pip3")
                .args(["install", "-r"])
                .arg(&requirements)
                .output()
                .map_err(|e| {
                    RegistryError::Storage(format!("Failed to install Python dependencies: {}", e))
                })?;

            if !output.status.success() {
                return Err(RegistryError::Storage(format!(
                    "Python dependency installation failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                )));
            }
        }

        Ok(())
    }
}

// ===== JavaScript/TypeScript Executor =====

#[derive(Default)]
struct JavaScriptExecutor {
    runtime: String, // "node" or "deno" or "bun"
}

impl JavaScriptExecutor {
    fn new(runtime: String) -> Self {
        Self { runtime }
    }
}

impl RuntimeExecutor for JavaScriptExecutor {
    fn start(
        &self,
        plugin_path: &Path,
        config: &RuntimeConfig,
    ) -> Result<Box<dyn RuntimeProcess>> {
        let runtime = if self.runtime.is_empty() {
            "node"
        } else {
            &self.runtime
        };

        let mut cmd = Command::new(runtime);

        cmd.arg(plugin_path)
            .args(&config.args)
            .envs(&config.env_vars)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(dir) = &config.working_dir {
            cmd.current_dir(dir);
        }

        let child = cmd.spawn().map_err(|e| {
            RegistryError::Storage(format!("Failed to start JavaScript plugin: {}", e))
        })?;

        Ok(Box::new(ProcessWrapper::new(child)))
    }

    fn is_available(&self) -> bool {
        Command::new("node")
            .arg("--version")
            .output()
            .is_ok()
    }

    fn version(&self) -> Result<String> {
        let output = Command::new("node")
            .arg("--version")
            .output()
            .map_err(|e| RegistryError::Storage(format!("Failed to get Node.js version: {}", e)))?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn install_dependencies(&self, plugin_path: &Path) -> Result<()> {
        let package_json = plugin_path.join("package.json");

        if package_json.exists() {
            let output = Command::new("npm")
                .arg("install")
                .current_dir(plugin_path)
                .output()
                .map_err(|e| {
                    RegistryError::Storage(format!("Failed to install npm dependencies: {}", e))
                })?;

            if !output.status.success() {
                return Err(RegistryError::Storage(format!(
                    "npm install failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                )));
            }
        }

        Ok(())
    }
}

// ===== Go Executor =====

struct GoExecutor;

impl RuntimeExecutor for GoExecutor {
    fn start(
        &self,
        plugin_path: &Path,
        config: &RuntimeConfig,
    ) -> Result<Box<dyn RuntimeProcess>> {
        let mut cmd = Command::new(plugin_path);

        cmd.args(&config.args)
            .envs(&config.env_vars)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(dir) = &config.working_dir {
            cmd.current_dir(dir);
        }

        let child = cmd
            .spawn()
            .map_err(|e| RegistryError::Storage(format!("Failed to start Go plugin: {}", e)))?;

        Ok(Box::new(ProcessWrapper::new(child)))
    }

    fn is_available(&self) -> bool {
        Command::new("go").arg("version").output().is_ok()
    }

    fn version(&self) -> Result<String> {
        let output = Command::new("go")
            .arg("version")
            .output()
            .map_err(|e| RegistryError::Storage(format!("Failed to get Go version: {}", e)))?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn install_dependencies(&self, plugin_path: &Path) -> Result<()> {
        let output = Command::new("go")
            .args(["build", "-o", "plugin"])
            .current_dir(plugin_path)
            .output()
            .map_err(|e| RegistryError::Storage(format!("Failed to build Go plugin: {}", e)))?;

        if !output.status.success() {
            return Err(RegistryError::Storage(format!(
                "Go plugin build failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        Ok(())
    }
}

// ===== Ruby Executor =====

struct RubyExecutor;

impl RuntimeExecutor for RubyExecutor {
    fn start(
        &self,
        plugin_path: &Path,
        config: &RuntimeConfig,
    ) -> Result<Box<dyn RuntimeProcess>> {
        let mut cmd = Command::new("ruby");

        cmd.arg(plugin_path)
            .args(&config.args)
            .envs(&config.env_vars)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(dir) = &config.working_dir {
            cmd.current_dir(dir);
        }

        let child = cmd
            .spawn()
            .map_err(|e| RegistryError::Storage(format!("Failed to start Ruby plugin: {}", e)))?;

        Ok(Box::new(ProcessWrapper::new(child)))
    }

    fn is_available(&self) -> bool {
        Command::new("ruby")
            .arg("--version")
            .output()
            .is_ok()
    }

    fn version(&self) -> Result<String> {
        let output = Command::new("ruby")
            .arg("--version")
            .output()
            .map_err(|e| RegistryError::Storage(format!("Failed to get Ruby version: {}", e)))?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn install_dependencies(&self, plugin_path: &Path) -> Result<()> {
        let gemfile = plugin_path.join("Gemfile");

        if gemfile.exists() {
            let output = Command::new("bundle")
                .arg("install")
                .current_dir(plugin_path)
                .output()
                .map_err(|e| {
                    RegistryError::Storage(format!("Failed to install Ruby gems: {}", e))
                })?;

            if !output.status.success() {
                return Err(RegistryError::Storage(format!(
                    "bundle install failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                )));
            }
        }

        Ok(())
    }
}

// ===== Generic Executor =====

struct GenericExecutor;

impl RuntimeExecutor for GenericExecutor {
    fn start(
        &self,
        plugin_path: &Path,
        config: &RuntimeConfig,
    ) -> Result<Box<dyn RuntimeProcess>> {
        let mut cmd = Command::new(plugin_path);

        cmd.args(&config.args)
            .envs(&config.env_vars)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(dir) = &config.working_dir {
            cmd.current_dir(dir);
        }

        let child = cmd.spawn().map_err(|e| {
            RegistryError::Storage(format!("Failed to start generic plugin: {}", e))
        })?;

        Ok(Box::new(ProcessWrapper::new(child)))
    }

    fn is_available(&self) -> bool {
        true
    }

    fn version(&self) -> Result<String> {
        Ok("unknown".to_string())
    }

    fn install_dependencies(&self, _plugin_path: &Path) -> Result<()> {
        Ok(())
    }
}

// ===== Process Wrapper =====

struct ProcessWrapper {
    child: Child,
}

impl ProcessWrapper {
    fn new(child: Child) -> Self {
        Self { child }
    }
}

impl RuntimeProcess for ProcessWrapper {
    fn is_running(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    fn stop(&mut self) -> Result<()> {
        self.child
            .kill()
            .map_err(|e| RegistryError::Storage(format!("Failed to kill process: {}", e)))
    }

    fn pid(&self) -> Option<u32> {
        Some(self.child.id())
    }

    fn send_message(&mut self, message: &[u8]) -> Result<()> {
        use std::io::Write;

        if let Some(stdin) = self.child.stdin.as_mut() {
            stdin
                .write_all(message)
                .map_err(|e| RegistryError::Network(format!("Failed to send message: {}", e)))?;
            stdin
                .flush()
                .map_err(|e| RegistryError::Network(format!("Failed to flush stdin: {}", e)))?;
        }

        Ok(())
    }

    fn receive_message(&mut self) -> Result<Vec<u8>> {
        use std::io::Read;

        if let Some(stdout) = self.child.stdout.as_mut() {
            let mut buffer = Vec::new();
            stdout
                .read_to_end(&mut buffer)
                .map_err(|e| RegistryError::Network(format!("Failed to read message: {}", e)))?;
            return Ok(buffer);
        }

        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_executor_available() {
        let executor = RustExecutor;
        // This may fail in environments without Rust
        let _ = executor.is_available();
    }

    #[test]
    fn test_runtime_config_default() {
        let config = RuntimeConfig::default();
        assert_eq!(config.timeout, 30);
        assert_eq!(config.memory_limit, Some(512));
    }
}
