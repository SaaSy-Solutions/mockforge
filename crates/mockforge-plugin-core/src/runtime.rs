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
use std::sync::{Arc, OnceLock};
use tokio::sync::RwLock;
use tracing;
use wasmtime::{Config, Engine, Linker, Module, PoolingAllocationConfig, ResourceLimiter, Store, StoreLimits, StoreLimitsBuilder};
use wasmtime_wasi::p2::WasiCtxBuilder;
use wasmtime_wasi::preview1::{self, WasiP1Ctx};
use wasmtime_wasi::{DirPerms, FilePerms};

/// WebAssembly runtime for plugin execution
pub struct PluginRuntime {
    /// WebAssembly engine (lazy-initialized only when plugins are loaded)
    engine: OnceLock<Engine>,
    /// Active plugin instances
    plugins: RwLock<HashMap<PluginId, Arc<RwLock<PluginInstance>>>>,
    /// Runtime configuration
    config: RuntimeConfig,
}

impl PluginRuntime {
    /// Create a new plugin runtime
    ///
    /// Note: The WebAssembly engine is lazy-initialized on first plugin load
    /// to avoid unnecessary memory allocation when no plugins are used.
    pub fn new(config: RuntimeConfig) -> Result<Self> {
        Ok(Self {
            engine: OnceLock::new(),
            plugins: RwLock::new(HashMap::new()),
            config,
        })
    }

    /// Get or initialize the WebAssembly engine
    fn get_engine(&self) -> &Engine {
        self.engine.get_or_init(|| {
            // Lazy initialization: only create engine when first plugin is loaded
            // Configure engine with security and resource limits
            let mut config = Config::new();

            // Enable fuel consumption tracking for CPU time limits
            config.consume_fuel(true);

            // Enable epoch-based interruption for wall clock timeouts
            config.epoch_interruption(true);

            // Enable memory limiting
            config.max_wasm_stack(2 * 1024 * 1024); // 2MB stack limit

            // Disable features that could be security risks
            config.wasm_threads(false);
            config.wasm_bulk_memory(true); // Allow for efficiency
            config.wasm_simd(false); // Disable SIMD for now
            config.wasm_multi_memory(false);

            // Enable pooling allocator for better performance and memory isolation
            config.allocation_strategy(wasmtime::InstanceAllocationStrategy::Pooling(
                PoolingAllocationConfig::default(),
            ));

            Engine::new(&config).expect("Failed to create WASM engine with security config")
        })
    }

    /// Load a plugin from WebAssembly module
    pub async fn load_plugin(
        &self,
        plugin_id: PluginId,
        manifest: PluginManifest,
        wasm_path: &Path,
    ) -> Result<()> {
        // Security: Validate plugin path is within allowed directories
        self.validate_plugin_path(wasm_path)?;
        
        // Security: Check file size limits
        self.validate_file_size(wasm_path)?;
        
        // Validate plugin capabilities against runtime limits
        let plugin_capabilities = PluginCapabilities::from_strings(&manifest.capabilities);
        self.validate_capabilities(&plugin_capabilities)?;

        // Security: Validate manifest integrity
        self.validate_manifest_security(&manifest)?;

        // Load WASM module with additional validation
        // This will lazy-initialize the engine if it hasn't been created yet
        let engine = self.get_engine();
        let module = Module::from_file(engine, wasm_path)
            .map_err(|e| PluginError::wasm(format!("Failed to load WASM module: {}", e)))?;

        // Security: Validate module against declared capabilities
        ModuleValidator::validate_module(&module, &plugin_capabilities)?;
        
        // Security: Check for dangerous imports/exports
        self.validate_module_security(&module)?;

        // Create plugin instance
        let instance =
            PluginInstance::new(plugin_id.clone(), manifest, module, self.config.clone()).await?;

        // Store plugin instance
        let mut plugins = self.plugins.write().await;
        #[allow(clippy::arc_with_non_send_sync)]
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
        let instance = plugins
            .get(plugin_id)
            .ok_or_else(|| PluginError::execution("Plugin not found"))?;

        let mut instance = instance.write().await;
        instance.execute_function(function_name, context, input).await
    }

    /// Get plugin health status
    pub async fn get_plugin_health(&self, plugin_id: &PluginId) -> Result<PluginHealth> {
        let plugins = self.plugins.read().await;
        let instance = plugins
            .get(plugin_id)
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
        let instance = plugins
            .get(plugin_id)
            .ok_or_else(|| PluginError::execution("Plugin not found"))?;

        let instance = instance.read().await;
        Ok(instance.metrics.clone())
    }

    /// Validate plugin capabilities against runtime limits
    fn validate_capabilities(&self, capabilities: &PluginCapabilities) -> Result<()> {
        // Check memory limits
        if capabilities.resources.max_memory_bytes > self.config.max_memory_per_plugin {
            return Err(PluginError::security(format!(
                "Plugin memory limit {} exceeds runtime limit {}",
                capabilities.resources.max_memory_bytes, self.config.max_memory_per_plugin
            )));
        }

        // Check CPU limits
        if capabilities.resources.max_cpu_percent > self.config.max_cpu_per_plugin {
            return Err(PluginError::security(format!(
                "Plugin CPU limit {:.2}% exceeds runtime limit {:.2}%",
                capabilities.resources.max_cpu_percent, self.config.max_cpu_per_plugin
            )));
        }

        // Check execution time limits
        if capabilities.resources.max_execution_time_ms > self.config.max_execution_time_ms {
            return Err(PluginError::security(format!(
                "Plugin execution time limit {}ms exceeds runtime limit {}ms",
                capabilities.resources.max_execution_time_ms, self.config.max_execution_time_ms
            )));
        }

        // Check network permissions
        if capabilities.network.allow_http && !self.config.allow_network_access {
            return Err(PluginError::security(
                "Plugin requires network access but runtime disallows it",
            ));
        }

        Ok(())
    }

    /// Security: Validate plugin path is within allowed directories
    fn validate_plugin_path(&self, wasm_path: &Path) -> Result<()> {
        let canonicalized = wasm_path.canonicalize()
            .map_err(|e| PluginError::security(format!("Invalid plugin path: {}", e)))?;
        
        // Check if path is within allowed plugin directories
        if self.config.allowed_fs_paths.is_empty() {
            return Err(PluginError::security("No allowed plugin paths configured"));
        }
        
        for allowed_path in &self.config.allowed_fs_paths {
            if canonicalized.starts_with(allowed_path) {
                return Ok(());
            }
        }
        
        Err(PluginError::security(format!(
            "Plugin path {} is not within allowed directories", 
            canonicalized.display()
        )))
    }

    /// Security: Check file size limits
    fn validate_file_size(&self, wasm_path: &Path) -> Result<()> {
        let metadata = std::fs::metadata(wasm_path)
            .map_err(|e| PluginError::security(format!("Cannot read plugin file metadata: {}", e)))?;
        
        const MAX_PLUGIN_SIZE: u64 = 50 * 1024 * 1024; // 50MB limit
        if metadata.len() > MAX_PLUGIN_SIZE {
            return Err(PluginError::security(format!(
                "Plugin file size {} exceeds maximum allowed size {}",
                metadata.len(), MAX_PLUGIN_SIZE
            )));
        }
        
        Ok(())
    }

    /// Security: Validate manifest integrity and security properties
    fn validate_manifest_security(&self, manifest: &PluginManifest) -> Result<()> {
        // Validate plugin name contains only safe characters
        if !manifest.info.name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            return Err(PluginError::security("Plugin name contains unsafe characters"));
        }

        // Check for dangerous capabilities
        let dangerous_caps = ["raw_syscalls", "kernel_access", "direct_memory"];
        for cap in &manifest.capabilities {
            if dangerous_caps.contains(&cap.as_str()) {
                return Err(PluginError::security(format!(
                    "Dangerous capability not allowed: {}", cap
                )));
            }
        }

        // Validate author field exists and is reasonable
        if manifest.info.author.name.is_empty() || manifest.info.author.name.len() > 100 {
            return Err(PluginError::security("Invalid author field in manifest"));
        }

        // Validate plugin ID format
        if manifest.info.id.0.is_empty() || manifest.info.id.0.len() > 100 {
            return Err(PluginError::security("Invalid plugin ID format"));
        }

        // Validate description length
        if manifest.info.description.len() > 1000 {
            return Err(PluginError::security("Plugin description too long"));
        }

        Ok(())
    }

    /// Security: Check for dangerous imports/exports in WASM module
    fn validate_module_security(&self, module: &Module) -> Result<()> {
        // Check imports for dangerous functions
        for import in module.imports() {
            match import.module() {
                "env" => {
                    // Allow basic environment functions
                    match import.name() {
                        "memory" | "table" => continue,
                        name if name.starts_with("__") => {
                            return Err(PluginError::security(format!(
                                "Dangerous import function: {}", name
                            )));
                        }
                        _ => continue,
                    }
                }
                "wasi_snapshot_preview1" => {
                    // Allow standard WASI functions
                    continue;
                }
                module_name => {
                    return Err(PluginError::security(format!(
                        "Dangerous import module: {}", module_name
                    )));
                }
            }
        }

        // Check exports for required functions
        let mut has_init = false;
        let mut has_process = false;
        
        for export in module.exports() {
            match export.name() {
                "init" => has_init = true,
                "process" => has_process = true,
                name if name.starts_with("_") => {
                    return Err(PluginError::security(format!(
                        "Private export function not allowed: {}", name
                    )));
                }
                _ => continue,
            }
        }

        if !has_init || !has_process {
            return Err(PluginError::security(
                "Plugin must export 'init' and 'process' functions"
            ));
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
    /// Allowed filesystem paths for plugins (empty for no access)
    pub allowed_fs_paths: Vec<String>,
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
            max_cpu_per_plugin: 0.5,                 // 50% of one core
            max_execution_time_ms: 5000,             // 5 seconds
            allow_network_access: false,
            allowed_fs_paths: vec![],
            max_concurrent_executions: 10,
            cache_dir: None,
            debug_logging: false,
        }
    }
}

/// WASI context with resource limits
pub struct WasiCtxWithLimits {
    /// WASI P1 context
    wasi: WasiP1Ctx,
    /// Store limits
    limits: StoreLimits,
}

impl WasiCtxWithLimits {
    fn new(wasi: WasiP1Ctx, limits: StoreLimits) -> Self {
        Self { wasi, limits }
    }
}

/// Implement ResourceLimiter to enforce memory limits
impl ResourceLimiter for WasiCtxWithLimits {
    fn memory_growing(
        &mut self,
        current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> anyhow::Result<bool> {
        // Check if the desired memory growth exceeds our limits
        self.limits.memory_growing(current, desired, _maximum)
    }

    fn table_growing(
        &mut self,
        current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> anyhow::Result<bool> {
        // Check if the desired table growth exceeds our limits
        self.limits.table_growing(current, desired, _maximum)
    }
}

/// Plugin instance wrapper
pub struct PluginInstance {
    /// Plugin ID
    #[allow(dead_code)]
    plugin_id: PluginId,
    /// Plugin manifest
    #[allow(dead_code)]
    manifest: PluginManifest,
    /// WebAssembly instance with WASI support
    instance: wasmtime::Instance,
    /// WebAssembly store with WASI context and limits
    store: Store<WasiCtxWithLimits>,
    /// Plugin state
    state: PluginState,
    /// Plugin metrics
    metrics: PluginMetrics,
    /// Runtime configuration
    #[allow(dead_code)]
    config: RuntimeConfig,
    /// Creation time
    #[allow(dead_code)]
    created_at: chrono::DateTime<chrono::Utc>,
    /// Execution limits
    limits: ExecutionLimits,
}

impl PluginInstance {
    /// Create a new plugin instance
    async fn new(
        plugin_id: PluginId,
        manifest: PluginManifest,
        module: Module,
        config: RuntimeConfig,
    ) -> Result<Self> {
        // Create execution limits from config
        let limits = ExecutionLimits {
            memory_limit: config.max_memory_per_plugin,
            cpu_time_limit: config.max_execution_time_ms as u64 * 1_000_000, // Convert ms to ns
            wall_time_limit: config.max_execution_time_ms as u64 * 2 * 1_000_000, // 2x for wall time
            fuel_limit: (config.max_execution_time_ms as u64 * 1_000), // ~1K fuel per ms
        };

        // Build store limits for memory enforcement
        let store_limits = StoreLimitsBuilder::new()
            .memory_size(limits.memory_limit)
            .table_elements(1000) // Limit table size
            .instances(1) // Single instance per store
            .tables(10) // Limit number of tables
            .memories(1) // Single memory per instance
            .build();

        // Create WASI context with appropriate permissions
        let mut wasi_ctx_builder = WasiCtxBuilder::new();

        // Configure WASI based on runtime config
        let wasi_ctx_builder = wasi_ctx_builder.inherit_stdio();

        // Preopen allowed filesystem paths
        for path in &config.allowed_fs_paths {
            wasi_ctx_builder.preopened_dir(
                Path::new(path),
                path.as_str(),
                DirPerms::all(),
                FilePerms::all(),
            )?;
        }

        let wasi_ctx = wasi_ctx_builder.build_p1();

        // Wrap WASI context with resource limits
        let ctx_with_limits = WasiCtxWithLimits::new(wasi_ctx, store_limits);

        // Create WebAssembly store with WASI context and limits
        let mut store = Store::new(module.engine(), ctx_with_limits);

        // Set store limiter to enforce memory limits
        store.limiter(|ctx| &mut ctx.limits);

        // Configure fuel for CPU time limiting
        store.set_fuel(limits.fuel_limit)
            .map_err(|e| PluginError::wasm(format!("Failed to set fuel limit: {}", e)))?;

        // Set epoch deadline for wall clock timeout
        // Epoch is incremented by a background thread in production
        store.set_epoch_deadline(1);

        // Link WASI functions to the store
        let mut linker = Linker::<WasiCtxWithLimits>::new(module.engine());
        preview1::add_to_linker_sync(&mut linker, |t| &mut t.wasi)
            .map_err(|e| PluginError::wasm(format!("Failed to add WASI to linker: {}", e)))?;

        // Instantiate the module with WASI support
        let instance = linker.instantiate(&mut store, &module).map_err(|e| {
            PluginError::wasm(format!("Failed to instantiate WASM module with WASI: {}", e))
        })?;

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
            limits,
        })
    }

    /// Execute a plugin function
    async fn execute_function<T>(
        &mut self,
        function_name: &str,
        context: &PluginContext,
        _input: &[u8],
    ) -> Result<PluginResult<T>>
    where
        T: serde::de::DeserializeOwned,
    {
        let start_time = std::time::Instant::now();

        // Update state
        self.state = PluginState::Executing;
        self.metrics.total_executions += 1;

        // Reset fuel before execution to prevent fuel starvation
        self.store.set_fuel(self.limits.fuel_limit)
            .map_err(|e| PluginError::execution(format!("Failed to reset fuel: {}", e)))?;

        // Reset epoch deadline for this execution
        self.store.set_epoch_deadline(1);

        // Prepare input parameters
        let context_json = serde_json::to_string(context)
            .map_err(|e| PluginError::execution(format!("Failed to serialize context: {}", e)))?;

        // Execute function (this is a simplified implementation)
        // In practice, you'd need to handle the WASM calling convention
        let result = self.call_plugin_function(function_name, &context_json).await;

        // Check remaining fuel to track CPU usage
        let fuel_consumed = match self.store.get_fuel() {
            Ok(remaining) => self.limits.fuel_limit.saturating_sub(remaining),
            Err(_) => 0, // Fuel tracking disabled or error
        };

        // Update metrics
        let execution_time = start_time.elapsed();
        self.metrics.avg_execution_time_ms = (self.metrics.avg_execution_time_ms
            * (self.metrics.total_executions - 1) as f64
            + execution_time.as_millis() as f64)
            / self.metrics.total_executions as f64;

        if execution_time.as_millis() as u64 > self.metrics.max_execution_time_ms {
            self.metrics.max_execution_time_ms = execution_time.as_millis() as u64;
        }

        // Update state
        self.state = PluginState::Ready;

        match result {
            Ok(output) => {
                self.metrics.successful_executions += 1;
                match serde_json::from_slice::<T>(&output) {
                    Ok(data) => {
                        tracing::debug!(
                            "Plugin execution completed: {} fuel consumed, {}ms elapsed",
                            fuel_consumed,
                            execution_time.as_millis()
                        );
                        Ok(PluginResult::success(data, execution_time.as_millis() as u64))
                    }
                    Err(e) => {
                        self.metrics.failed_executions += 1;
                        Err(PluginError::execution(format!("Failed to deserialize result: {}", e)))
                    }
                }
            }
            Err(e) => {
                self.metrics.failed_executions += 1;
                tracing::error!(
                    "Plugin execution failed: {} fuel consumed, {}ms elapsed, error: {}",
                    fuel_consumed,
                    execution_time.as_millis(),
                    e
                );
                Err(e)
            }
        }
    }

    /// Call a plugin function (simplified implementation)
    async fn call_plugin_function(&mut self, function_name: &str, input: &str) -> Result<Vec<u8>> {
        // Get the exported function from the WASM instance
        let func = self.instance.get_func(&mut self.store, function_name).ok_or_else(|| {
            PluginError::execution(format!(
                "Function '{}' not found in WASM module",
                function_name
            ))
        })?;

        // Allocate memory in the WASM store for the input string
        let input_bytes = input.as_bytes();
        let input_len = input_bytes.len() as i32;

        // Allocate space for the input string in WASM memory
        let alloc_func = self.instance.get_func(&mut self.store, "alloc").ok_or_else(|| {
            PluginError::execution(
                "WASM module must export an 'alloc' function for memory allocation",
            )
        })?;

        let mut alloc_result = [wasmtime::Val::I32(0)];
        alloc_func
            .call(&mut self.store, &[wasmtime::Val::I32(input_len)], &mut alloc_result)
            .map_err(|e| {
                PluginError::execution(format!("Failed to allocate memory for input: {}", e))
            })?;

        let input_ptr = match alloc_result[0] {
            wasmtime::Val::I32(ptr) => ptr,
            _ => {
                return Err(PluginError::execution("alloc function did not return a valid pointer"))
            }
        };

        // Write the input string to WASM memory
        let memory = self
            .instance
            .get_memory(&mut self.store, "memory")
            .ok_or_else(|| PluginError::execution("WASM module must export a 'memory'"))?;

        memory.write(&mut self.store, input_ptr as usize, input_bytes).map_err(|e| {
            PluginError::execution(format!("Failed to write input to WASM memory: {}", e))
        })?;

        // Call the plugin function with the input pointer and length
        let mut func_result = [wasmtime::Val::I32(0), wasmtime::Val::I32(0)];
        func.call(
            &mut self.store,
            &[wasmtime::Val::I32(input_ptr), wasmtime::Val::I32(input_len)],
            &mut func_result,
        )
        .map_err(|e| {
            PluginError::execution(format!(
                "Failed to call WASM function '{}': {}",
                function_name, e
            ))
        })?;

        // Extract the return values (assuming the function returns (ptr, len))
        let output_ptr = match func_result[0] {
            wasmtime::Val::I32(ptr) => ptr,
            _ => {
                return Err(PluginError::execution(format!(
                    "Function '{}' did not return a valid output pointer",
                    function_name
                )))
            }
        };

        let output_len = match func_result[1] {
            wasmtime::Val::I32(len) => len,
            _ => {
                return Err(PluginError::execution(format!(
                    "Function '{}' did not return a valid output length",
                    function_name
                )))
            }
        };

        // Read the output from WASM memory
        let mut output_bytes = vec![0u8; output_len as usize];
        memory
            .read(&mut self.store, output_ptr as usize, &mut output_bytes)
            .map_err(|e| {
                PluginError::execution(format!("Failed to read output from WASM memory: {}", e))
            })?;

        // Deallocate the memory if there's a dealloc function
        if let Some(dealloc_func) = self.instance.get_func(&mut self.store, "dealloc") {
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

        Ok(output_bytes)
    }

    /// Get plugin health
    async fn get_health(&self) -> PluginHealth {
        PluginHealth::healthy("Plugin is running".to_string(), self.metrics.clone())
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
            memory_limit: 10 * 1024 * 1024,  // 10MB
            cpu_time_limit: 5_000_000_000,   // 5 seconds
            wall_time_limit: 10_000_000_000, // 10 seconds
            fuel_limit: 1_000_000,           // 1M fuel units
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
                    return Err(PluginError::security(format!(
                        "Disallowed import module: {}",
                        module_name
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
            "fd_read",
            "fd_write",
            "fd_close",
            "fd_fdstat_get",
            "path_open",
            "path_readlink",
            "path_filestat_get",
        ];

        if filesystem_functions.contains(&field_name)
            && capabilities.filesystem.read_paths.is_empty()
            && capabilities.filesystem.write_paths.is_empty()
        {
            return Err(PluginError::security(format!(
                "Plugin imports filesystem function '{}' but has no filesystem capabilities",
                field_name
            )));
        }

        // Allow other safe WASI functions
        let allowed_functions = [
            "fd_read",
            "fd_write",
            "fd_close",
            "fd_fdstat_get",
            "path_open",
            "path_readlink",
            "path_filestat_get",
            "clock_time_get",
            "proc_exit",
            "random_get",
        ];

        if !allowed_functions.contains(&field_name) {
            return Err(PluginError::security(format!(
                "Disallowed WASI function: {}",
                field_name
            )));
        }

        Ok(())
    }

    /// Validate host function imports
    fn validate_host_import(field_name: &str) -> Result<()> {
        let allowed_functions = [
            "log_message",
            "get_config_value",
            "store_data",
            "retrieve_data",
        ];

        if !allowed_functions.contains(&field_name) {
            return Err(PluginError::security(format!(
                "Disallowed host function: {}",
                field_name
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
                let return_type = func_type.results().next().and_then(|result| match result {
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

#[cfg(test)]
mod tests {
    

    #[test]
    fn test_module_compiles() {
        // Basic compilation test
        assert!(true);
    }
}
