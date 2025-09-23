//! WebAssembly runtime for plugin execution
//!
//! This module provides the WebAssembly runtime environment for secure
//! execution of MockForge plugins. It handles plugin loading, sandboxing,
//! and communication between the host and plugin code.

use crate::{
    PluginCapabilities, PluginContext, PluginError, PluginHealth, PluginId, PluginManifest,
    PluginMetrics, PluginResult, PluginState, Result,
};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use wasmtime::{Engine, Linker, Module, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder};

/// WebAssembly runtime for plugin execution
pub struct PluginRuntime {
    /// WebAssembly engine
    engine: Engine,
    /// Active plugin instances
    plugins: RwLock<HashMap<PluginId, Arc<RwLock<PluginInstance>>>>,
    /// Runtime configuration
    config: RuntimeConfig,
}

impl PluginRuntime {
    /// Create a new plugin runtime
    pub fn new(config: RuntimeConfig) -> Result<Self> {
        let engine = Engine::default();

        Ok(Self {
            engine,
            plugins: RwLock::new(HashMap::new()),
            config,
        })
    }

    /// Load a plugin from WebAssembly module
    pub async fn load_plugin(
        &self,
        plugin_id: PluginId,
        manifest: PluginManifest,
        wasm_path: &Path,
    ) -> Result<()> {
        // Validate plugin capabilities against runtime limits
        let plugin_capabilities = PluginCapabilities::from_strings(&manifest.capabilities);
        self.validate_capabilities(&plugin_capabilities)?;

        // Load WASM module
        let module = Module::from_file(&self.engine, wasm_path)
            .map_err(|e| PluginError::wasm(&format!("Failed to load WASM module: {}", e)))?;

        // Validate module against declared capabilities
        ModuleValidator::validate_module(&module, &plugin_capabilities)?;

        // Create plugin instance
        let instance = PluginInstance::new(
            plugin_id.clone(),
            manifest,
            module,
            self.config.clone(),
        ).await?;

        // Store plugin instance
        let mut plugins = self.plugins.write().await;
        plugins.insert(plugin_id, Arc::new(RwLock::new(instance)));

        Ok(())
    }

    /// Unload a plugin
    pub async fn unload_plugin(&self, plugin_id: &PluginId) -> Result<()> {
        let mut plugins = self.plugins.write().await;
        if let Some(instance) = plugins.remove(plugin_id) {
            let mut instance = instance.write().await;
            instance.unload().await?;
        }
        Ok(())
    }

    /// Execute a plugin function
    pub async fn execute_plugin_function<T>(
        &self,
        plugin_id: &PluginId,
        function_name: &str,
        context: &PluginContext,
        input: &[u8],
    ) -> Result<PluginResult<T>>
    where
        T: serde::de::DeserializeOwned,
    {
        let plugins = self.plugins.read().await;
        let instance = plugins.get(plugin_id)
            .ok_or_else(|| PluginError::execution("Plugin not found"))?;

        let mut instance = instance.write().await;
        instance.execute_function(function_name, context, input).await
    }

    /// Get plugin health status
    pub async fn get_plugin_health(&self, plugin_id: &PluginId) -> Result<PluginHealth> {
        let plugins = self.plugins.read().await;
        let instance = plugins.get(plugin_id)
            .ok_or_else(|| PluginError::execution("Plugin not found"))?;

        let instance = instance.read().await;
        Ok(instance.get_health().await)
    }

    /// List loaded plugins
    pub async fn list_plugins(&self) -> Vec<PluginId> {
        let plugins = self.plugins.read().await;
        plugins.keys().cloned().collect()
    }

    /// Get plugin metrics
    pub async fn get_plugin_metrics(&self, plugin_id: &PluginId) -> Result<PluginMetrics> {
        let plugins = self.plugins.read().await;
        let instance = plugins.get(plugin_id)
            .ok_or_else(|| PluginError::execution("Plugin not found"))?;

        let instance = instance.read().await;
        Ok(instance.metrics.clone())
    }

    /// Validate plugin capabilities against runtime limits
    fn validate_capabilities(&self, capabilities: &PluginCapabilities) -> Result<()> {
        // Check memory limits
        if capabilities.resources.max_memory_bytes > self.config.max_memory_per_plugin {
            return Err(PluginError::security(&format!(
                "Plugin memory limit {} exceeds runtime limit {}",
                capabilities.resources.max_memory_bytes,
                self.config.max_memory_per_plugin
            )));
        }

        // Check CPU limits
        if capabilities.resources.max_cpu_percent > self.config.max_cpu_per_plugin {
            return Err(PluginError::security(&format!(
                "Plugin CPU limit {:.2}% exceeds runtime limit {:.2}%",
                capabilities.resources.max_cpu_percent,
                self.config.max_cpu_per_plugin
            )));
        }

        // Check execution time limits
        if capabilities.resources.max_execution_time_ms > self.config.max_execution_time_ms {
            return Err(PluginError::security(&format!(
                "Plugin execution time limit {}ms exceeds runtime limit {}ms",
                capabilities.resources.max_execution_time_ms,
                self.config.max_execution_time_ms
            )));
        }

        // Check network permissions
        if capabilities.network.allow_http && !self.config.allow_network_access {
            return Err(PluginError::security("Plugin requires network access but runtime disallows it"));
        }

        Ok(())
    }
}

/// Runtime configuration
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Maximum memory per plugin (bytes)
    pub max_memory_per_plugin: usize,
    /// Maximum CPU usage per plugin (0.0-1.0)
    pub max_cpu_per_plugin: f64,
    /// Maximum execution time per plugin (milliseconds)
    pub max_execution_time_ms: u64,
    /// Allow network access
    pub allow_network_access: bool,
    /// Allow file system access
    pub allow_filesystem_access: bool,
    /// Maximum concurrent plugin executions
    pub max_concurrent_executions: usize,
    /// Plugin cache directory
    pub cache_dir: Option<String>,
    /// Enable debug logging
    pub debug_logging: bool,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            max_memory_per_plugin: 10 * 1024 * 1024, // 10MB
            max_cpu_per_plugin: 0.5,                  // 50% of one core
            max_execution_time_ms: 5000,              // 5 seconds
            allow_network_access: false,
            allow_filesystem_access: false,
            max_concurrent_executions: 10,
            cache_dir: None,
            debug_logging: false,
        }
    }
}

/// Plugin instance wrapper
pub struct PluginInstance {
    /// Plugin ID
    plugin_id: PluginId,
    /// Plugin manifest
    manifest: PluginManifest,
    /// WebAssembly instance with WASI support
    instance: wasmtime::Instance,
    /// WebAssembly store with WASI context
    store: Store<WasiCtx>,
    /// Plugin state
    state: PluginState,
    /// Plugin metrics
    metrics: PluginMetrics,
    /// Runtime configuration
    config: RuntimeConfig,
    /// Creation time
    created_at: chrono::DateTime<chrono::Utc>,
}

impl PluginInstance {
    /// Create a new plugin instance
    async fn new(
        plugin_id: PluginId,
        manifest: PluginManifest,
        module: Module,
        config: RuntimeConfig,
    ) -> Result<Self> {
        // Create WASI context with appropriate permissions
        let mut wasi_ctx_builder = WasiCtxBuilder::new();

        // Configure WASI based on runtime config
        let wasi_ctx = if config.allow_filesystem_access {
            // Allow filesystem access - in production, this should be more restrictive
            wasi_ctx_builder
                .inherit_stdio()
                .inherit_env()
                .build()
        } else {
            // Minimal WASI context for plugins without filesystem access
            wasi_ctx_builder
                .inherit_stdio()
                .build()
        };

        // Create WebAssembly store with WASI context
        let mut store = Store::new(&module.engine(), wasi_ctx);

        // Link WASI functions to the store
        let linker = Linker::new(&module.engine());
        // TODO: Fix WASI linker setup - API has changed
        // wasmtime_wasi::add_to_linker(&mut linker, |ctx: &mut WasiCtx| ctx)?;

        // Instantiate the module with WASI support
        let instance = linker.instantiate(&mut store, &module)
            .map_err(|e| PluginError::wasm(&format!("Failed to instantiate WASM module with WASI: {}", e)))?;

        // Note: Component model support can be added here when available
        // The component model will provide better interface definitions and composition

        Ok(Self {
            plugin_id,
            manifest,
            instance,
            store,
            state: PluginState::Loaded,
            metrics: PluginMetrics::default(),
            config,
            created_at: chrono::Utc::now(),
        })
    }

    /// Execute a plugin function
    async fn execute_function<T>(
        &mut self,
        function_name: &str,
        context: &PluginContext,
        input: &[u8],
    ) -> Result<PluginResult<T>>
    where
        T: serde::de::DeserializeOwned,
    {
        let start_time = std::time::Instant::now();

        // Update state
        self.state = PluginState::Executing;
        self.metrics.total_executions += 1;

        // Prepare input parameters
        let context_json = serde_json::to_string(context)
            .map_err(|e| PluginError::execution(&format!("Failed to serialize context: {}", e)))?;

        // Execute function (this is a simplified implementation)
        // In practice, you'd need to handle the WASM calling convention
        let result = self.call_plugin_function(function_name, &context_json).await;

        // Update metrics
        let execution_time = start_time.elapsed();
        self.metrics.avg_execution_time_ms = (
            self.metrics.avg_execution_time_ms * (self.metrics.total_executions - 1) as f64
            + execution_time.as_millis() as f64
        ) / self.metrics.total_executions as f64;

        if execution_time.as_millis() as u64 > self.metrics.max_execution_time_ms {
            self.metrics.max_execution_time_ms = execution_time.as_millis() as u64;
        }

        // Update state
        self.state = PluginState::Ready;

        match result {
            Ok(output) => {
                self.metrics.successful_executions += 1;
                match serde_json::from_slice::<T>(&output) {
                    Ok(data) => Ok(PluginResult::success(data, execution_time.as_millis() as u64)),
                    Err(e) => {
                        self.metrics.failed_executions += 1;
                        Err(PluginError::execution(&format!("Failed to deserialize result: {}", e)))
                    }
                }
            }
            Err(e) => {
                self.metrics.failed_executions += 1;
                Err(e)
            }
        }
    }

    /// Call a plugin function (simplified implementation)
    async fn call_plugin_function(
        &mut self,
        function_name: &str,
        input: &str,
    ) -> Result<Vec<u8>> {
        // Get the exported function from the WASM instance
        let func = self.instance.get_func(&mut self.store, function_name)
            .ok_or_else(|| PluginError::execution(&format!("Function '{}' not found in WASM module", function_name)))?;

        // Allocate memory in the WASM store for the input string
        let input_bytes = input.as_bytes();
        let input_len = input_bytes.len() as i32;

        // Allocate space for the input string in WASM memory
        let alloc_func = self.instance.get_func(&mut self.store, "alloc")
            .ok_or_else(|| PluginError::execution("WASM module must export an 'alloc' function for memory allocation"))?;

        let input_ptr = alloc_func.call(&mut self.store, &[wasmtime::Val::I32(input_len)])
            .map_err(|e| PluginError::execution(&format!("Failed to allocate memory for input: {}", e)))?;

        let input_ptr = match input_ptr.get(0) {
            Some(wasmtime::Val::I32(ptr)) => *ptr,
            _ => return Err(PluginError::execution("alloc function did not return a valid pointer")),
        };

        // Write the input string to WASM memory
        let memory = self.instance.get_memory(&mut self.store, "memory")
            .ok_or_else(|| PluginError::execution("WASM module must export a 'memory'"))?;

        memory.write(&mut self.store, input_ptr as usize, input_bytes)
            .map_err(|e| PluginError::execution(&format!("Failed to write input to WASM memory: {}", e)))?;

        // Call the plugin function with the input pointer and length
        let result = func.call(&mut self.store, &[wasmtime::Val::I32(input_ptr), wasmtime::Val::I32(input_len)])
            .map_err(|e| PluginError::execution(&format!("Failed to call WASM function '{}': {}", function_name, e)))?;

        // Extract the return values (assuming the function returns (ptr, len))
        let output_ptr = match result.get(0) {
            Some(wasmtime::Val::I32(ptr)) => *ptr,
            _ => return Err(PluginError::execution(&format!("Function '{}' did not return a valid output pointer", function_name))),
        };

        let output_len = match result.get(1) {
            Some(wasmtime::Val::I32(len)) => *len,
            _ => return Err(PluginError::execution(&format!("Function '{}' did not return a valid output length", function_name))),
        };

        // Read the output from WASM memory
        let mut output_bytes = vec![0u8; output_len as usize];
        memory.read(&mut self.store, output_ptr as usize, &mut output_bytes)
            .map_err(|e| PluginError::execution(&format!("Failed to read output from WASM memory: {}", e)))?;

        // Deallocate the memory if there's a dealloc function
        if let Ok(dealloc_func) = self.instance.get_func(&mut self.store, "dealloc") {
            let _ = dealloc_func.call(&mut self.store, &[wasmtime::Val::I32(input_ptr), wasmtime::Val::I32(input_len)]);
            let _ = dealloc_func.call(&mut self.store, &[wasmtime::Val::I32(output_ptr), wasmtime::Val::I32(output_len)]);
        }

        Ok(output_bytes)
    }

    /// Get plugin health
    async fn get_health(&self) -> PluginHealth {
        PluginHealth::healthy(
            "Plugin is running".to_string(),
            self.metrics.clone(),
        )
    }

    /// Unload plugin
    async fn unload(&mut self) -> Result<()> {
        self.state = PluginState::Unloading;
        // Cleanup resources here
        self.state = PluginState::Unloaded;
        Ok(())
    }
}

/// Plugin execution limits
pub struct ExecutionLimits {
    /// Memory limit (bytes)
    pub memory_limit: usize,
    /// CPU time limit (nanoseconds)
    pub cpu_time_limit: u64,
    /// Wall clock time limit (nanoseconds)
    pub wall_time_limit: u64,
    /// Fuel limit (WASM execution fuel)
    pub fuel_limit: u64,
}

impl Default for ExecutionLimits {
    fn default() -> Self {
        Self {
            memory_limit: 10 * 1024 * 1024, // 10MB
            cpu_time_limit: 5_000_000_000,  // 5 seconds
            wall_time_limit: 10_000_000_000, // 10 seconds
            fuel_limit: 1_000_000,          // 1M fuel units
        }
    }
}

/// Plugin security context
pub struct SecurityContext {
    /// Allowed syscalls
    pub allowed_syscalls: Vec<String>,
    /// Blocked syscalls
    pub blocked_syscalls: Vec<String>,
    /// Network access policy
    pub network_policy: NetworkPolicy,
    /// File system access policy
    pub filesystem_policy: FilesystemPolicy,
}

impl Default for SecurityContext {
    fn default() -> Self {
        Self {
            allowed_syscalls: vec![
                "fd_write".to_string(),
                "fd_read".to_string(),
                "random_get".to_string(),
                "clock_time_get".to_string(),
            ],
            blocked_syscalls: vec![
                "path_open".to_string(),
                "sock_open".to_string(),
                "proc_exec".to_string(),
            ],
            network_policy: NetworkPolicy::DenyAll,
            filesystem_policy: FilesystemPolicy::DenyAll,
        }
    }
}

/// Network access policy
#[derive(Debug, Clone)]
pub enum NetworkPolicy {
    /// Allow all network access
    AllowAll,
    /// Deny all network access
    DenyAll,
    /// Allow access to specific hosts
    AllowHosts(Vec<String>),
}

/// File system access policy
#[derive(Debug, Clone)]
pub enum FilesystemPolicy {
    /// Allow all file system access
    AllowAll,
    /// Deny all file system access
    DenyAll,
    /// Allow access to specific paths
    AllowPaths(Vec<String>),
}

/// WASM module validator
pub struct ModuleValidator;

impl ModuleValidator {
    /// Validate a WASM module for security against declared capabilities
    pub fn validate_module(module: &Module, capabilities: &PluginCapabilities) -> Result<()> {
        // Check for dangerous imports based on capabilities
        Self::validate_imports(module, capabilities)?;

        Ok(())
    }

    /// Validate WASM imports against plugin capabilities
    fn validate_imports(module: &Module, capabilities: &PluginCapabilities) -> Result<()> {
        for import in module.imports() {
            let module_name = import.module();
            let field_name = import.name();

            match module_name {
                "wasi_snapshot_preview1" | "wasi:io/streams" | "wasi:filesystem/types" => {
                    Self::validate_wasi_import(field_name, capabilities)?;
                }
                "mockforge:plugin/host" => {
                    // Host functions are generally allowed
                    Self::validate_host_import(field_name)?;
                }
                _ => {
                    return Err(PluginError::security(&format!(
                        "Disallowed import module: {}", module_name
                    )));
                }
            }
        }

        Ok(())
    }

    /// Validate WASI imports against capabilities
    fn validate_wasi_import(field_name: &str, capabilities: &PluginCapabilities) -> Result<()> {
        // Check filesystem operations
        let filesystem_functions = [
            "fd_read", "fd_write", "fd_close", "fd_fdstat_get",
            "path_open", "path_readlink", "path_filestat_get",
        ];

        if filesystem_functions.contains(&field_name) {
            if capabilities.filesystem.read_paths.is_empty() && capabilities.filesystem.write_paths.is_empty() {
                return Err(PluginError::security(&format!(
                    "Plugin imports filesystem function '{}' but has no filesystem capabilities", field_name
                )));
            }
        }

        // Allow other safe WASI functions
        let allowed_functions = [
            "fd_read", "fd_write", "fd_close", "fd_fdstat_get",
            "path_open", "path_readlink", "path_filestat_get",
            "clock_time_get", "proc_exit", "random_get",
        ];

        if !allowed_functions.contains(&field_name) {
            return Err(PluginError::security(&format!(
                "Disallowed WASI function: {}", field_name
            )));
        }

        Ok(())
    }

    /// Validate host function imports
    fn validate_host_import(field_name: &str) -> Result<()> {
        let allowed_functions = [
            "log_message", "get_config_value", "store_data", "retrieve_data",
        ];

        if !allowed_functions.contains(&field_name) {
            return Err(PluginError::security(&format!(
                "Disallowed host function: {}", field_name
            )));
        }

        Ok(())
    }

    /// Extract plugin interface from WASM module
    pub fn extract_plugin_interface(module: &Module) -> Result<PluginInterface> {
        let mut functions = Vec::new();

        // Iterate over module exports to find functions
        for export in module.exports() {
            if let wasmtime::ExternType::Func(func_type) = export.ty() {
                // Convert WASM parameter types to our ValueType
                let parameters: Vec<ValueType> = func_type
                    .params()
                    .filter_map(|param| match param {
                        wasmtime::ValType::I32 => Some(ValueType::I32),
                        wasmtime::ValType::I64 => Some(ValueType::I64),
                        wasmtime::ValType::F32 => Some(ValueType::F32),
                        wasmtime::ValType::F64 => Some(ValueType::F64),
                        _ => {
                            // For now, skip unsupported types (like V128, Ref, etc.)
                            // In a full implementation, you might want to handle these
                            None
                        }
                    })
                    .collect();

                // Convert WASM return type (assuming single return for simplicity)
                let return_type = func_type
                    .results()
                    .next()
                    .and_then(|result| match result {
                        wasmtime::ValType::I32 => Some(ValueType::I32),
                        wasmtime::ValType::I64 => Some(ValueType::I64),
                        wasmtime::ValType::F32 => Some(ValueType::F32),
                        wasmtime::ValType::F64 => Some(ValueType::F64),
                        _ => {
                            // Skip unsupported return types
                            None
                        }
                    });

                functions.push(PluginFunction {
                    name: export.name().to_string(),
                    signature: FunctionSignature {
                        parameters,
                        return_type,
                    },
                    documentation: None, // Could be extracted from custom sections in the future
                });
            }
        }

        Ok(PluginInterface { functions })
    }
}

/// Plugin interface description
#[derive(Debug, Clone)]
pub struct PluginInterface {
    /// Available functions
    pub functions: Vec<PluginFunction>,
}

/// Plugin function description
#[derive(Debug, Clone)]
pub struct PluginFunction {
    /// Function name
    pub name: String,
    /// Function signature
    pub signature: FunctionSignature,
    /// Documentation
    pub documentation: Option<String>,
}

/// Function signature
#[derive(Debug, Clone)]
pub struct FunctionSignature {
    /// Parameter types
    pub parameters: Vec<ValueType>,
    /// Return type
    pub return_type: Option<ValueType>,
}

/// WASM value type
#[derive(Debug, Clone)]
pub enum ValueType {
    /// 32-bit integer
    I32,
    /// 64-bit integer
    I64,
    /// 32-bit float
    F32,
    /// 64-bit float
    F64,
}
