//! WebAssembly sandbox for secure plugin execution
//!
//! This module provides the sandboxed execution environment for plugins,
//! including resource limits, security boundaries, and isolation.

use super::*;
use mockforge_plugin_core::{
    PluginCapabilities, PluginContext, PluginHealth, PluginId, PluginMetrics, PluginResult,
    PluginState,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use wasmtime::{Engine, Linker, Module, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder};

/// Plugin sandbox for secure execution
pub struct PluginSandbox {
    /// WebAssembly engine (optional)
    engine: Option<Arc<Engine>>,
    /// Sandbox configuration
    _config: PluginLoaderConfig,
    /// Active sandboxes
    active_sandboxes: RwLock<HashMap<PluginId, SandboxInstance>>,
}

impl PluginSandbox {
    /// Create a new plugin sandbox
    pub fn new(config: PluginLoaderConfig) -> Self {
        // Create WebAssembly engine for plugin execution
        let engine = Some(Arc::new(Engine::default()));

        Self {
            engine,
            _config: config,
            active_sandboxes: RwLock::new(HashMap::new()),
        }
    }

    /// Create a plugin instance in the sandbox
    pub async fn create_plugin_instance(
        &self,
        context: &PluginLoadContext,
    ) -> LoaderResult<PluginInstance> {
        let plugin_id = &context.plugin_id;

        // Check if sandbox already exists
        {
            let sandboxes = self.active_sandboxes.read().await;
            if sandboxes.contains_key(plugin_id) {
                return Err(PluginLoaderError::already_loaded(plugin_id.clone()));
            }
        }

        // Create sandbox instance
        let sandbox = if let Some(ref engine) = self.engine {
            SandboxInstance::new(engine, context).await?
        } else {
            // Create a stub sandbox instance when WebAssembly is disabled
            SandboxInstance::stub_new(context).await?
        };

        // Store sandbox instance
        let mut sandboxes = self.active_sandboxes.write().await;
        sandboxes.insert(plugin_id.clone(), sandbox);

        // Create core plugin instance
        let mut core_instance =
            mockforge_plugin_core::PluginInstance::new(plugin_id.clone(), context.manifest.clone());
        core_instance.set_state(PluginState::Ready);

        Ok(core_instance)
    }

    /// Execute a plugin function in the sandbox
    pub async fn execute_plugin_function(
        &self,
        plugin_id: &PluginId,
        function_name: &str,
        context: &PluginContext,
        input: &[u8],
    ) -> LoaderResult<PluginResult<serde_json::Value>> {
        let mut sandboxes = self.active_sandboxes.write().await;
        let sandbox = sandboxes
            .get_mut(plugin_id)
            .ok_or_else(|| PluginLoaderError::not_found(plugin_id.clone()))?;

        sandbox.execute_function(function_name, context, input).await
    }

    /// Get plugin health from sandbox
    pub async fn get_plugin_health(&self, plugin_id: &PluginId) -> LoaderResult<PluginHealth> {
        let sandboxes = self.active_sandboxes.read().await;
        let sandbox = sandboxes
            .get(plugin_id)
            .ok_or_else(|| PluginLoaderError::not_found(plugin_id.clone()))?;

        Ok(sandbox.get_health().await)
    }

    /// Destroy a plugin sandbox
    pub async fn destroy_sandbox(&self, plugin_id: &PluginId) -> LoaderResult<()> {
        let mut sandboxes = self.active_sandboxes.write().await;
        if let Some(mut sandbox) = sandboxes.remove(plugin_id) {
            sandbox.destroy().await?;
        }
        Ok(())
    }

    /// List active sandboxes
    pub async fn list_active_sandboxes(&self) -> Vec<PluginId> {
        let sandboxes = self.active_sandboxes.read().await;
        sandboxes.keys().cloned().collect()
    }

    /// Get sandbox resource usage
    pub async fn get_sandbox_resources(
        &self,
        plugin_id: &PluginId,
    ) -> LoaderResult<SandboxResources> {
        let sandboxes = self.active_sandboxes.read().await;
        let sandbox = sandboxes
            .get(plugin_id)
            .ok_or_else(|| PluginLoaderError::not_found(plugin_id.clone()))?;

        Ok(sandbox.get_resources().await)
    }

    /// Check sandbox health
    pub async fn check_sandbox_health(&self, plugin_id: &PluginId) -> LoaderResult<SandboxHealth> {
        let sandboxes = self.active_sandboxes.read().await;
        let sandbox = sandboxes
            .get(plugin_id)
            .ok_or_else(|| PluginLoaderError::not_found(plugin_id.clone()))?;

        Ok(sandbox.check_health().await)
    }
}

/// Individual sandbox instance
pub struct SandboxInstance {
    /// Plugin ID
    _plugin_id: PluginId,
    /// WebAssembly module
    _module: Module,
    /// WebAssembly store
    store: Store<WasiCtx>,
    /// Linker for the instance
    linker: Linker<WasiCtx>,
    /// Sandbox resources
    resources: SandboxResources,
    /// Health monitor
    health: SandboxHealth,
    /// Execution limits
    limits: ExecutionLimits,
}

impl SandboxInstance {
    /// Create a new sandbox instance
    async fn new(engine: &Engine, context: &PluginLoadContext) -> LoaderResult<Self> {
        let plugin_id = &context.plugin_id;

        // Load WASM module
        let module = Module::from_file(engine, &context.plugin_path)
            .map_err(|e| PluginLoaderError::wasm(format!("Failed to load WASM module: {}", e)))?;

        // Create WASI context
        let wasi_ctx = WasiCtxBuilder::new().inherit_stderr().inherit_stdout().build();

        // Create WebAssembly store
        let mut store = Store::new(engine, wasi_ctx);

        // Create linker
        let linker = Linker::new(engine);

        // Add WASI functions using the updated API
        // For now, skip WASI integration until proper wasmtime-wasi version is resolved
        // This is a non-critical feature for the main MockForge functionality

        // Instantiate the module
        linker
            .instantiate(&mut store, &module)
            .map_err(|e| PluginLoaderError::wasm(format!("Failed to instantiate module: {}", e)))?;

        // Set up execution limits
        let plugin_capabilities = PluginCapabilities::default();
        let limits = ExecutionLimits::from_capabilities(&plugin_capabilities);

        Ok(Self {
            _plugin_id: plugin_id.clone(),
            _module: module,
            store,
            linker,
            resources: SandboxResources::default(),
            health: SandboxHealth::healthy(),
            limits,
        })
    }

    /// Create a stub sandbox instance (when WebAssembly is disabled)
    async fn stub_new(context: &PluginLoadContext) -> LoaderResult<Self> {
        let plugin_id = &context.plugin_id;

        // Create dummy values for when WebAssembly is disabled
        let module = Module::new(&Engine::default(), [])
            .map_err(|e| PluginLoaderError::wasm(format!("Failed to create stub module: {}", e)))?;

        let wasi_ctx = WasiCtxBuilder::new().inherit_stderr().inherit_stdout().build();

        let store = Store::new(&Engine::default(), wasi_ctx);
        let linker = Linker::new(&Engine::default());

        let plugin_capabilities = PluginCapabilities::default();
        let limits = ExecutionLimits::from_capabilities(&plugin_capabilities);

        Ok(Self {
            _plugin_id: plugin_id.clone(),
            _module: module,
            store,
            linker,
            resources: SandboxResources::default(),
            health: SandboxHealth::healthy(),
            limits,
        })
    }

    /// Execute a function in the sandbox
    async fn execute_function(
        &mut self,
        function_name: &str,
        context: &PluginContext,
        input: &[u8],
    ) -> LoaderResult<PluginResult<serde_json::Value>> {
        // Update resource tracking
        self.resources.execution_count += 1;
        self.resources.last_execution = chrono::Utc::now();

        // Check execution limits
        if self.resources.execution_count > self.limits.max_executions {
            return Err(PluginLoaderError::resource_limit(format!(
                "Maximum executions exceeded: {} allowed, {} used",
                self.limits.max_executions, self.resources.execution_count
            )));
        }

        // Check time limits
        let time_since_last = chrono::Utc::now().signed_duration_since(self.resources.created_at);
        let time_since_last_std =
            std::time::Duration::from_secs(time_since_last.num_seconds() as u64);
        if time_since_last_std > self.limits.max_lifetime {
            return Err(PluginLoaderError::resource_limit(format!(
                "Maximum lifetime exceeded: {}s allowed, {}s used",
                self.limits.max_lifetime.as_secs(),
                time_since_last_std.as_secs()
            )));
        }

        // Prepare function call
        let start_time = std::time::Instant::now();

        // Get function from linker
        let _func = self.linker.get(&mut self.store, "", function_name).ok_or_else(|| {
            PluginLoaderError::execution(format!("Function '{}' not found", function_name))
        })?;

        // Execute function (simplified - real implementation would handle WASM calling conventions)
        let result = self.call_wasm_function(function_name, context, input).await;

        // Update resource tracking
        let execution_time = start_time.elapsed();
        self.resources.total_execution_time += execution_time;
        self.resources.last_execution_time = execution_time;

        if execution_time > self.resources.max_execution_time {
            self.resources.max_execution_time = execution_time;
        }

        match result {
            Ok(data) => {
                self.resources.success_count += 1;
                Ok(PluginResult::success(data, execution_time.as_millis() as u64))
            }
            Err(e) => {
                self.resources.error_count += 1;
                Ok(PluginResult::failure(e.to_string(), execution_time.as_millis() as u64))
            }
        }
    }

    /// Call WebAssembly function
    async fn call_wasm_function(
        &mut self,
        function_name: &str,
        context: &PluginContext,
        input: &[u8],
    ) -> Result<serde_json::Value, String> {
        // Serialize context and input for WASM
        let context_json = serde_json::to_string(context)
            .map_err(|e| format!("Failed to serialize context: {}", e))?;
        let combined_input = format!("{}\n{}", context_json, String::from_utf8_lossy(input));

        // Get the exported function from the linker
        let func_extern = self
            .linker
            .get(&mut self.store, "", function_name)
            .ok_or_else(|| format!("Function '{}' not found in WASM module", function_name))?;
        let func = func_extern
            .into_func()
            .ok_or_else(|| format!("Export '{}' is not a function", function_name))?;

        // Allocate memory in WASM for the input string
        let input_bytes = combined_input.as_bytes();
        let input_len = input_bytes.len() as i32;

        // Get alloc function
        let alloc_extern = self.linker.get(&mut self.store, "", "alloc").ok_or_else(|| {
            "WASM module must export an 'alloc' function for memory allocation".to_string()
        })?;
        let alloc_func = alloc_extern
            .into_func()
            .ok_or_else(|| "Export 'alloc' is not a function".to_string())?;

        let mut alloc_result = [wasmtime::Val::I32(0)];
        alloc_func
            .call(&mut self.store, &[wasmtime::Val::I32(input_len)], &mut alloc_result)
            .map_err(|e| format!("Failed to allocate memory for input: {}", e))?;

        let input_ptr = match alloc_result[0] {
            wasmtime::Val::I32(ptr) => ptr,
            _ => return Err("alloc function did not return a valid pointer".to_string()),
        };

        // Write the input string to WASM memory
        let memory_extern = self
            .linker
            .get(&mut self.store, "", "memory")
            .ok_or_else(|| "WASM module must export a 'memory'".to_string())?;
        let memory = memory_extern
            .into_memory()
            .ok_or_else(|| "Export 'memory' is not a memory".to_string())?;

        memory
            .write(&mut self.store, input_ptr as usize, input_bytes)
            .map_err(|e| format!("Failed to write input to WASM memory: {}", e))?;

        // Call the plugin function with the input pointer and length
        let mut func_result = [wasmtime::Val::I32(0), wasmtime::Val::I32(0)];
        func.call(
            &mut self.store,
            &[wasmtime::Val::I32(input_ptr), wasmtime::Val::I32(input_len)],
            &mut func_result,
        )
        .map_err(|e| format!("Failed to call WASM function '{}': {}", function_name, e))?;

        // Extract the return values (assuming the function returns (ptr, len))
        let output_ptr = match func_result[0] {
            wasmtime::Val::I32(ptr) => ptr,
            _ => {
                return Err(format!(
                    "Function '{}' did not return a valid output pointer",
                    function_name
                ))
            }
        };

        let output_len = match func_result[1] {
            wasmtime::Val::I32(len) => len,
            _ => {
                return Err(format!(
                    "Function '{}' did not return a valid output length",
                    function_name
                ))
            }
        };

        // Read the output from WASM memory
        let mut output_bytes = vec![0u8; output_len as usize];
        memory
            .read(&mut self.store, output_ptr as usize, &mut output_bytes)
            .map_err(|e| format!("Failed to read output from WASM memory: {}", e))?;

        // Deallocate the memory if there's a dealloc function
        if let Some(dealloc_extern) = self.linker.get(&mut self.store, "", "dealloc") {
            if let Some(dealloc_func) = dealloc_extern.into_func() {
                let _ = dealloc_func.call(
                    &mut self.store,
                    &[wasmtime::Val::I32(input_ptr), wasmtime::Val::I32(input_len)],
                    &mut [],
                );
                let _ = dealloc_func.call(
                    &mut self.store,
                    &[
                        wasmtime::Val::I32(output_ptr),
                        wasmtime::Val::I32(output_len),
                    ],
                    &mut [],
                );
            }
        }

        // Parse the output as JSON
        let output_str = String::from_utf8(output_bytes)
            .map_err(|e| format!("Failed to convert output to string: {}", e))?;

        serde_json::from_str(&output_str)
            .map_err(|e| format!("Failed to parse WASM output as JSON: {}", e))
    }

    /// Get sandbox health
    async fn get_health(&self) -> PluginHealth {
        if self.health.is_healthy {
            PluginHealth::healthy(
                "Sandbox is healthy".to_string(),
                PluginMetrics {
                    total_executions: self.resources.execution_count,
                    successful_executions: self.resources.success_count,
                    failed_executions: self.resources.error_count,
                    avg_execution_time_ms: self.resources.avg_execution_time_ms(),
                    max_execution_time_ms: self.resources.max_execution_time.as_millis() as u64,
                    memory_usage_bytes: self.resources.memory_usage,
                    peak_memory_usage_bytes: self.resources.peak_memory_usage,
                },
            )
        } else {
            PluginHealth::unhealthy(
                PluginState::Error,
                self.health.last_error.clone(),
                PluginMetrics::default(),
            )
        }
    }

    /// Get sandbox resources
    async fn get_resources(&self) -> SandboxResources {
        self.resources.clone()
    }

    /// Check sandbox health
    async fn check_health(&self) -> SandboxHealth {
        self.health.clone()
    }

    /// Destroy the sandbox
    async fn destroy(&mut self) -> LoaderResult<()> {
        // Cleanup resources
        self.health.is_healthy = false;
        self.health.last_error = "Sandbox destroyed".to_string();
        Ok(())
    }
}

/// Sandbox resource tracking
#[derive(Debug, Clone, Default)]
pub struct SandboxResources {
    /// Total execution count
    pub execution_count: u64,
    /// Successful execution count
    pub success_count: u64,
    /// Error execution count
    pub error_count: u64,
    /// Total execution time
    pub total_execution_time: std::time::Duration,
    /// Last execution time
    pub last_execution_time: std::time::Duration,
    /// Maximum execution time
    pub max_execution_time: std::time::Duration,
    /// Current memory usage
    pub memory_usage: usize,
    /// Peak memory usage
    pub peak_memory_usage: usize,
    /// Creation time
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last execution time
    pub last_execution: chrono::DateTime<chrono::Utc>,
}

impl SandboxResources {
    /// Get average execution time in milliseconds
    pub fn avg_execution_time_ms(&self) -> f64 {
        if self.execution_count == 0 {
            0.0
        } else {
            self.total_execution_time.as_millis() as f64 / self.execution_count as f64
        }
    }

    /// Get success rate as percentage
    pub fn success_rate(&self) -> f64 {
        if self.execution_count == 0 {
            0.0
        } else {
            (self.success_count as f64 / self.execution_count as f64) * 100.0
        }
    }

    /// Check if resource limits are exceeded
    pub fn check_limits(&self, limits: &ExecutionLimits) -> bool {
        self.execution_count <= limits.max_executions
            && self.memory_usage <= limits.max_memory_bytes
            && self.total_execution_time <= limits.max_total_time
    }
}

/// Sandbox health status
#[derive(Debug, Clone)]
pub struct SandboxHealth {
    /// Whether sandbox is healthy
    pub is_healthy: bool,
    /// Last health check time
    pub last_check: chrono::DateTime<chrono::Utc>,
    /// Last error message
    pub last_error: String,
    /// Health check results
    pub checks: Vec<HealthCheck>,
}

impl SandboxHealth {
    /// Create healthy status
    pub fn healthy() -> Self {
        Self {
            is_healthy: true,
            last_check: chrono::Utc::now(),
            last_error: String::new(),
            checks: Vec::new(),
        }
    }

    /// Create unhealthy status
    pub fn unhealthy<S: Into<String>>(error: S) -> Self {
        Self {
            is_healthy: false,
            last_check: chrono::Utc::now(),
            last_error: error.into(),
            checks: Vec::new(),
        }
    }

    /// Add health check result
    pub fn add_check(&mut self, check: HealthCheck) {
        let failed = !check.passed;
        let error_message = if failed {
            Some(check.message.clone())
        } else {
            None
        };

        self.checks.push(check);
        self.last_check = chrono::Utc::now();

        // Update overall health
        if failed {
            self.is_healthy = false;
            if let Some(msg) = error_message {
                self.last_error = msg;
            }
        }
    }

    /// Run health checks
    pub async fn run_checks(&mut self, resources: &SandboxResources, limits: &ExecutionLimits) {
        self.checks.clear();

        // Memory usage check
        let memory_check = if resources.memory_usage <= limits.max_memory_bytes {
            HealthCheck::pass("Memory usage within limits")
        } else {
            HealthCheck::fail(format!(
                "Memory usage {} exceeds limit {}",
                resources.memory_usage, limits.max_memory_bytes
            ))
        };
        self.add_check(memory_check);

        // Execution count check
        let execution_check = if resources.execution_count <= limits.max_executions {
            HealthCheck::pass("Execution count within limits")
        } else {
            HealthCheck::fail(format!(
                "Execution count {} exceeds limit {}",
                resources.execution_count, limits.max_executions
            ))
        };
        self.add_check(execution_check);

        // Success rate check
        let success_rate = resources.success_rate();
        let success_check = if success_rate >= 90.0 {
            HealthCheck::pass(format!("Success rate: {:.1}%", success_rate))
        } else {
            HealthCheck::fail(format!("Low success rate: {:.1}%", success_rate))
        };
        self.add_check(success_check);
    }
}

/// Health check result
#[derive(Debug, Clone)]
pub struct HealthCheck {
    /// Check name
    pub name: String,
    /// Whether check passed
    pub passed: bool,
    /// Check message
    pub message: String,
    /// Check timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl HealthCheck {
    /// Create passing check
    pub fn pass<S: Into<String>>(message: S) -> Self {
        Self {
            name: "health_check".to_string(),
            passed: true,
            message: message.into(),
            timestamp: chrono::Utc::now(),
        }
    }

    /// Create failing check
    pub fn fail<S: Into<String>>(message: S) -> Self {
        Self {
            name: "health_check".to_string(),
            passed: false,
            message: message.into(),
            timestamp: chrono::Utc::now(),
        }
    }
}

/// Execution limits for sandbox
#[derive(Debug, Clone)]
pub struct ExecutionLimits {
    /// Maximum number of executions
    pub max_executions: u64,
    /// Maximum total execution time
    pub max_total_time: std::time::Duration,
    /// Maximum lifetime
    pub max_lifetime: std::time::Duration,
    /// Maximum memory usage
    pub max_memory_bytes: usize,
    /// Maximum CPU time per execution
    pub max_cpu_time_per_execution: std::time::Duration,
}

impl Default for ExecutionLimits {
    fn default() -> Self {
        Self {
            max_executions: 1000,
            max_total_time: std::time::Duration::from_secs(300), // 5 minutes
            max_lifetime: std::time::Duration::from_secs(3600),  // 1 hour
            max_memory_bytes: 10 * 1024 * 1024,                  // 10MB
            max_cpu_time_per_execution: std::time::Duration::from_secs(5),
        }
    }
}

impl ExecutionLimits {
    /// Create limits from plugin capabilities
    pub fn from_capabilities(capabilities: &PluginCapabilities) -> Self {
        Self {
            max_executions: 10000, // Override with capability-based limits
            max_total_time: std::time::Duration::from_secs(600), // 10 minutes
            max_lifetime: std::time::Duration::from_secs(86400), // 24 hours
            max_memory_bytes: capabilities.resources.max_memory_bytes,
            max_cpu_time_per_execution: std::time::Duration::from_millis(
                (capabilities.resources.max_cpu_percent * 1000.0) as u64,
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sandbox_resources() {
        let resources = SandboxResources {
            execution_count: 10,
            success_count: 8,
            error_count: 2,
            total_execution_time: std::time::Duration::from_millis(1000),
            ..Default::default()
        };

        assert_eq!(resources.avg_execution_time_ms(), 100.0);
        assert_eq!(resources.success_rate(), 80.0);
    }

    #[tokio::test]
    async fn test_execution_limits() {
        let limits = ExecutionLimits::default();
        assert_eq!(limits.max_executions, 1000);
        assert_eq!(limits.max_memory_bytes, 10 * 1024 * 1024);
    }

    #[tokio::test]
    async fn test_health_checks() {
        let mut health = SandboxHealth::healthy();
        assert!(health.is_healthy);

        health.add_check(HealthCheck::fail("Test failure"));
        assert!(!health.is_healthy);
        assert_eq!(health.last_error, "Test failure");
    }

    #[tokio::test]
    async fn test_plugin_sandbox_creation() {
        let config = PluginLoaderConfig::default();
        let sandbox = PluginSandbox::new(config);

        let active = sandbox.list_active_sandboxes().await;
        assert!(active.is_empty());
    }
}
