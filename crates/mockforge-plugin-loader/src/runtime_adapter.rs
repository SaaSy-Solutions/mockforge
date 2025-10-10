//! Runtime adapter for supporting multiple plugin runtimes
//!
//! This module provides an abstraction layer that allows MockForge to load and execute
//! plugins written in different languages and compiled with different WASM runtimes.
//!
//! Supported runtimes:
//! - Rust (native, via wasmtime)
//! - TinyGo (Go compiled to WASM)
//! - AssemblyScript (TypeScript-like, compiled to WASM)
//! - Remote (HTTP/gRPC-based plugins in any language)

use async_trait::async_trait;
use mockforge_plugin_core::{
    AuthRequest, AuthResponse, DataQuery, DataResult, PluginContext, PluginError, PluginId,
    ResponseData, ResponseRequest, ResolutionContext,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use wasmtime::{Engine, Instance, Linker, Module, Store};
use wasmtime_wasi::p2::{WasiCtx, WasiCtxBuilder};

/// Enum representing different plugin runtime types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeType {
    /// Native Rust plugin compiled to WASM
    Rust,
    /// TinyGo compiled plugin
    TinyGo,
    /// AssemblyScript compiled plugin
    AssemblyScript,
    /// Remote plugin accessed via HTTP/gRPC
    Remote(RemoteRuntimeConfig),
}

/// Configuration for remote plugin runtime
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteRuntimeConfig {
    /// Protocol to use (http or grpc)
    pub protocol: RemoteProtocol,
    /// Endpoint URL
    pub endpoint: String,
    /// Request timeout in milliseconds
    pub timeout_ms: u64,
    /// Maximum number of retries
    pub max_retries: u32,
    /// Authentication configuration
    pub auth: Option<RemoteAuthConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteProtocol {
    Http,
    Grpc,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteAuthConfig {
    /// Authentication type (bearer, api_key, etc.)
    pub auth_type: String,
    /// Authentication value
    pub value: String,
}

/// Trait that all runtime adapters must implement
///
/// This allows different WASM runtimes and remote plugins to be used interchangeably
#[async_trait]
pub trait RuntimeAdapter: Send + Sync {
    /// Get the runtime type
    fn runtime_type(&self) -> RuntimeType;

    /// Initialize the runtime (called once during plugin load)
    async fn initialize(&mut self) -> Result<(), PluginError>;

    /// Call authentication plugin
    async fn call_auth(
        &self,
        context: &PluginContext,
        request: &AuthRequest,
    ) -> Result<AuthResponse, PluginError>;

    /// Call template function plugin
    async fn call_template_function(
        &self,
        function_name: &str,
        args: &[serde_json::Value],
        context: &ResolutionContext,
    ) -> Result<serde_json::Value, PluginError>;

    /// Call response generator plugin
    async fn call_response_generator(
        &self,
        context: &PluginContext,
        request: &ResponseRequest,
    ) -> Result<ResponseData, PluginError>;

    /// Call data source query plugin
    async fn call_datasource_query(
        &self,
        query: &DataQuery,
        context: &PluginContext,
    ) -> Result<DataResult, PluginError>;

    /// Health check - returns true if the plugin is healthy
    async fn health_check(&self) -> Result<bool, PluginError>;

    /// Cleanup resources (called during plugin unload)
    async fn cleanup(&mut self) -> Result<(), PluginError>;

    /// Get runtime-specific metrics
    fn get_metrics(&self) -> HashMap<String, serde_json::Value> {
        HashMap::new()
    }
}

/// Detect runtime type from WASM binary
pub fn detect_runtime_type(wasm_bytes: &[u8]) -> Result<RuntimeType, PluginError> {
    // Parse WASM module to detect runtime type
    // This is a simplified version - real implementation would parse WASM sections

    // Check for TinyGo signatures
    if has_tinygo_signature(wasm_bytes) {
        return Ok(RuntimeType::TinyGo);
    }

    // Check for AssemblyScript signatures
    if has_assemblyscript_signature(wasm_bytes) {
        return Ok(RuntimeType::AssemblyScript);
    }

    // Default to Rust
    Ok(RuntimeType::Rust)
}

/// Check if WASM binary has TinyGo signature
fn has_tinygo_signature(wasm_bytes: &[u8]) -> bool {
    // Look for TinyGo-specific exports or custom sections
    // TinyGo typically exports functions like "resume" and "getsp"
    // This is a placeholder - real implementation would use wasmparser

    // Simple heuristic: check for "tinygo" in custom sections
    String::from_utf8_lossy(wasm_bytes).contains("tinygo")
}

/// Check if WASM binary has AssemblyScript signature
fn has_assemblyscript_signature(wasm_bytes: &[u8]) -> bool {
    // Look for AssemblyScript-specific exports or custom sections
    // AS typically has "__new", "__pin", "__unpin" exports

    // Simple heuristic: check for "assemblyscript" in custom sections
    String::from_utf8_lossy(wasm_bytes).contains("assemblyscript")
}

/// Factory for creating runtime adapters
pub struct RuntimeAdapterFactory;

impl RuntimeAdapterFactory {
    /// Create a runtime adapter for the given runtime type
    pub fn create(
        runtime_type: RuntimeType,
        plugin_id: PluginId,
        wasm_bytes: Vec<u8>,
    ) -> Result<Box<dyn RuntimeAdapter>, PluginError> {
        match runtime_type {
            RuntimeType::Rust => {
                Ok(Box::new(RustAdapter::new(plugin_id, wasm_bytes)?))
            }
            RuntimeType::TinyGo => {
                Ok(Box::new(TinyGoAdapter::new(plugin_id, wasm_bytes)?))
            }
            RuntimeType::AssemblyScript => {
                Ok(Box::new(AssemblyScriptAdapter::new(plugin_id, wasm_bytes)?))
            }
            RuntimeType::Remote(config) => {
                Ok(Box::new(RemoteAdapter::new(plugin_id, config)?))
            }
        }
    }
}

// ============================================================================
// Rust Runtime Adapter (Existing Implementation)
// ============================================================================

pub struct RustAdapter {
    plugin_id: PluginId,
    engine: Arc<Engine>,
    module: Module,
    // Store and Instance need to be behind a Mutex since they're not Send/Sync
    runtime: Mutex<Option<WasmRuntime>>,
}

struct WasmRuntime {
    store: Store<WasiCtx>,
    instance: Instance,
}

impl RustAdapter {
    pub fn new(plugin_id: PluginId, wasm_bytes: Vec<u8>) -> Result<Self, PluginError> {
        let engine = Arc::new(Engine::default());
        let module = Module::from_binary(&engine, &wasm_bytes)
            .map_err(|e| PluginError::execution(format!("Failed to load WASM module: {}", e)))?;

        Ok(Self {
            plugin_id,
            engine,
            module,
            runtime: Mutex::new(None),
        })
    }

    /// Helper to call a WASM function with JSON input/output
    fn call_wasm_json(
        &self,
        function_name: &str,
        input_data: serde_json::Value,
    ) -> Result<serde_json::Value, PluginError> {
        let mut runtime_guard = self.runtime.lock().unwrap();
        let runtime = runtime_guard.as_mut().ok_or_else(|| {
            PluginError::execution("Runtime not initialized. Call initialize() first.".to_string())
        })?;

        let input_json = serde_json::to_string(&input_data)
            .map_err(|e| PluginError::execution(format!("Failed to serialize input: {}", e)))?;

        let input_bytes = input_json.as_bytes();
        let input_len = input_bytes.len() as i32;

        // Get memory and alloc function
        let memory = runtime
            .instance
            .get_memory(&mut runtime.store, "memory")
            .ok_or_else(|| PluginError::execution("WASM module must export 'memory'".to_string()))?;

        let alloc_func = runtime
            .instance
            .get_typed_func::<i32, i32>(&mut runtime.store, "alloc")
            .map_err(|e| PluginError::execution(format!("Failed to get alloc function: {}", e)))?;

        // Allocate memory for input
        let input_ptr = alloc_func
            .call(&mut runtime.store, input_len)
            .map_err(|e| PluginError::execution(format!("Failed to allocate memory: {}", e)))?;

        // Write input to WASM memory
        memory
            .write(&mut runtime.store, input_ptr as usize, input_bytes)
            .map_err(|e| PluginError::execution(format!("Failed to write input: {}", e)))?;

        // Call the plugin function
        let plugin_func = runtime
            .instance
            .get_typed_func::<(i32, i32), (i32, i32)>(&mut runtime.store, function_name)
            .map_err(|e| {
                PluginError::execution(format!("Function '{}' not found: {}", function_name, e))
            })?;

        let (output_ptr, output_len) = plugin_func
            .call(&mut runtime.store, (input_ptr, input_len))
            .map_err(|e| {
                PluginError::execution(format!("Failed to call function '{}': {}", function_name, e))
            })?;

        // Read output from WASM memory
        let mut output_bytes = vec![0u8; output_len as usize];
        memory
            .read(&runtime.store, output_ptr as usize, &mut output_bytes)
            .map_err(|e| PluginError::execution(format!("Failed to read output: {}", e)))?;

        // Deallocate memory if dealloc function exists
        if let Ok(dealloc_func) = runtime
            .instance
            .get_typed_func::<(i32, i32), ()>(&mut runtime.store, "dealloc")
        {
            let _ = dealloc_func.call(&mut runtime.store, (input_ptr, input_len));
            let _ = dealloc_func.call(&mut runtime.store, (output_ptr, output_len));
        }

        // Parse output as JSON
        let output_str = String::from_utf8(output_bytes)
            .map_err(|e| PluginError::execution(format!("Failed to decode output: {}", e)))?;

        serde_json::from_str(&output_str)
            .map_err(|e| PluginError::execution(format!("Failed to parse output JSON: {}", e)))
    }
}

#[async_trait]
impl RuntimeAdapter for RustAdapter {
    fn runtime_type(&self) -> RuntimeType {
        RuntimeType::Rust
    }

    async fn initialize(&mut self) -> Result<(), PluginError> {
        // Initialize wasmtime instance with the Rust WASM module
        tracing::info!("Initializing Rust plugin: {}", self.plugin_id);

        // Create WASI context
        let wasi_ctx = WasiCtxBuilder::new()
            .inherit_stderr()
            .inherit_stdout()
            .build();

        // Create store
        let mut store = Store::new(&self.engine, wasi_ctx);

        // Create linker
        let linker = Linker::new(&self.engine);

        // Instantiate the module
        let instance = linker
            .instantiate(&mut store, &self.module)
            .map_err(|e| PluginError::execution(format!("Failed to instantiate module: {}", e)))?;

        // Store the runtime
        let mut runtime_guard = self.runtime.lock().unwrap();
        *runtime_guard = Some(WasmRuntime { store, instance });

        tracing::info!("Successfully initialized Rust plugin: {}", self.plugin_id);
        Ok(())
    }

    async fn call_auth(
        &self,
        context: &PluginContext,
        request: &AuthRequest,
    ) -> Result<AuthResponse, PluginError> {
        // Call Rust WASM plugin's auth function
        // Note: AuthRequest contains axum::http types which might not serialize well
        // So we create a simplified version for WASM
        let input = serde_json::json!({
            "context": context,
            "method": request.method.to_string(),
            "uri": request.uri.to_string(),
            "query_params": request.query_params,
            "client_ip": request.client_ip,
            "user_agent": request.user_agent,
        });

        let result = self.call_wasm_json("authenticate", input)?;
        serde_json::from_value(result)
            .map_err(|e| PluginError::execution(format!("Failed to parse AuthResponse: {}", e)))
    }

    async fn call_template_function(
        &self,
        function_name: &str,
        args: &[serde_json::Value],
        context: &ResolutionContext,
    ) -> Result<serde_json::Value, PluginError> {
        let input = serde_json::json!({
            "function_name": function_name,
            "args": args,
            "context": context,
        });

        self.call_wasm_json("template_function", input)
    }

    async fn call_response_generator(
        &self,
        context: &PluginContext,
        request: &ResponseRequest,
    ) -> Result<ResponseData, PluginError> {
        // Create simplified request for WASM (avoid non-serializable types)
        let input = serde_json::json!({
            "context": context,
            "method": request.method.to_string(),
            "uri": request.uri,
            "path": request.path,
            "query_params": request.query_params,
            "path_params": request.path_params,
            "client_ip": request.client_ip,
            "user_agent": request.user_agent,
        });

        let result = self.call_wasm_json("generate_response", input)?;
        serde_json::from_value(result)
            .map_err(|e| PluginError::execution(format!("Failed to parse ResponseData: {}", e)))
    }

    async fn call_datasource_query(
        &self,
        query: &DataQuery,
        context: &PluginContext,
    ) -> Result<DataResult, PluginError> {
        // DataQuery should be serializable, but create simplified version to be safe
        let input = serde_json::json!({
            "query_type": format!("{:?}", query.query_type),
            "query": query.query,
            "parameters": query.parameters,
            "limit": query.limit,
            "offset": query.offset,
            "context": context,
        });

        let result = self.call_wasm_json("query_datasource", input)?;
        serde_json::from_value(result)
            .map_err(|e| PluginError::execution(format!("Failed to parse DataResult: {}", e)))
    }

    async fn health_check(&self) -> Result<bool, PluginError> {
        Ok(true)
    }

    async fn cleanup(&mut self) -> Result<(), PluginError> {
        Ok(())
    }
}

// ============================================================================
// TinyGo Runtime Adapter
// ============================================================================

pub struct TinyGoAdapter {
    plugin_id: PluginId,
    engine: Arc<Engine>,
    module: Module,
    runtime: Mutex<Option<WasmRuntime>>,
}

impl TinyGoAdapter {
    pub fn new(plugin_id: PluginId, wasm_bytes: Vec<u8>) -> Result<Self, PluginError> {
        let engine = Arc::new(Engine::default());
        let module = Module::from_binary(&engine, &wasm_bytes)
            .map_err(|e| PluginError::execution(format!("Failed to load TinyGo WASM module: {}", e)))?;

        Ok(Self {
            plugin_id,
            engine,
            module,
            runtime: Mutex::new(None),
        })
    }

    /// Helper to call a TinyGo WASM function with JSON input/output
    /// TinyGo uses specific memory management and calling conventions
    fn call_wasm_json(
        &self,
        function_name: &str,
        input_data: serde_json::Value,
    ) -> Result<serde_json::Value, PluginError> {
        let mut runtime_guard = self.runtime.lock().unwrap();
        let runtime = runtime_guard.as_mut().ok_or_else(|| {
            PluginError::execution("Runtime not initialized. Call initialize() first.".to_string())
        })?;

        let input_json = serde_json::to_string(&input_data)
            .map_err(|e| PluginError::execution(format!("Failed to serialize input: {}", e)))?;

        let input_bytes = input_json.as_bytes();
        let input_len = input_bytes.len() as i32;

        // Get memory (TinyGo always exports memory)
        let memory = runtime
            .instance
            .get_memory(&mut runtime.store, "memory")
            .ok_or_else(|| PluginError::execution("TinyGo WASM module must export 'memory'".to_string()))?;

        // TinyGo uses malloc instead of alloc
        let malloc_func = runtime
            .instance
            .get_typed_func::<i32, i32>(&mut runtime.store, "malloc")
            .map_err(|e| PluginError::execution(format!("Failed to get malloc function (TinyGo specific): {}", e)))?;

        // Allocate memory for input
        let input_ptr = malloc_func
            .call(&mut runtime.store, input_len)
            .map_err(|e| PluginError::execution(format!("Failed to allocate memory: {}", e)))?;

        // Write input to WASM memory
        memory
            .write(&mut runtime.store, input_ptr as usize, input_bytes)
            .map_err(|e| PluginError::execution(format!("Failed to write input: {}", e)))?;

        // Call the plugin function
        let plugin_func = runtime
            .instance
            .get_typed_func::<(i32, i32), (i32, i32)>(&mut runtime.store, function_name)
            .map_err(|e| {
                PluginError::execution(format!("Function '{}' not found: {}", function_name, e))
            })?;

        let (output_ptr, output_len) = plugin_func
            .call(&mut runtime.store, (input_ptr, input_len))
            .map_err(|e| {
                PluginError::execution(format!("Failed to call function '{}': {}", function_name, e))
            })?;

        // Read output from WASM memory
        let mut output_bytes = vec![0u8; output_len as usize];
        memory
            .read(&runtime.store, output_ptr as usize, &mut output_bytes)
            .map_err(|e| PluginError::execution(format!("Failed to read output: {}", e)))?;

        // TinyGo uses free instead of dealloc
        if let Ok(free_func) = runtime
            .instance
            .get_typed_func::<i32, ()>(&mut runtime.store, "free")
        {
            let _ = free_func.call(&mut runtime.store, input_ptr);
            let _ = free_func.call(&mut runtime.store, output_ptr);
        }

        // Parse output as JSON
        let output_str = String::from_utf8(output_bytes)
            .map_err(|e| PluginError::execution(format!("Failed to decode output: {}", e)))?;

        serde_json::from_str(&output_str)
            .map_err(|e| PluginError::execution(format!("Failed to parse output JSON: {}", e)))
    }
}

#[async_trait]
impl RuntimeAdapter for TinyGoAdapter {
    fn runtime_type(&self) -> RuntimeType {
        RuntimeType::TinyGo
    }

    async fn initialize(&mut self) -> Result<(), PluginError> {
        // Initialize wasmtime instance with TinyGo-specific configuration
        // TinyGo requires special memory management and import handling
        tracing::info!("Initializing TinyGo plugin: {}", self.plugin_id);

        // Create WASI context (TinyGo supports WASI)
        let wasi_ctx = WasiCtxBuilder::new()
            .inherit_stderr()
            .inherit_stdout()
            .build();

        // Create store
        let mut store = Store::new(&self.engine, wasi_ctx);

        // Create linker with TinyGo-specific imports
        let linker = Linker::new(&self.engine);

        // TinyGo may require additional imports like syscall/js
        // For now, we'll use the basic linker

        // Instantiate the module
        let instance = linker
            .instantiate(&mut store, &self.module)
            .map_err(|e| PluginError::execution(format!("Failed to instantiate TinyGo module: {}", e)))?;

        // Store the runtime
        let mut runtime_guard = self.runtime.lock().unwrap();
        *runtime_guard = Some(WasmRuntime { store, instance });

        tracing::info!("Successfully initialized TinyGo plugin: {}", self.plugin_id);
        Ok(())
    }

    async fn call_auth(
        &self,
        context: &PluginContext,
        request: &AuthRequest,
    ) -> Result<AuthResponse, PluginError> {
        // Call TinyGo WASM plugin's auth function
        let input = serde_json::json!({
            "context": context,
            "method": request.method.to_string(),
            "uri": request.uri.to_string(),
            "query_params": request.query_params,
            "client_ip": request.client_ip,
            "user_agent": request.user_agent,
        });

        let result = self.call_wasm_json("authenticate", input)?;
        serde_json::from_value(result)
            .map_err(|e| PluginError::execution(format!("Failed to parse AuthResponse: {}", e)))
    }

    async fn call_template_function(
        &self,
        function_name: &str,
        args: &[serde_json::Value],
        context: &ResolutionContext,
    ) -> Result<serde_json::Value, PluginError> {
        let input = serde_json::json!({
            "function_name": function_name,
            "args": args,
            "context": context,
        });

        self.call_wasm_json("template_function", input)
    }

    async fn call_response_generator(
        &self,
        context: &PluginContext,
        request: &ResponseRequest,
    ) -> Result<ResponseData, PluginError> {
        // Create simplified request for WASM (avoid non-serializable types)
        let input = serde_json::json!({
            "context": context,
            "method": request.method.to_string(),
            "uri": request.uri,
            "path": request.path,
            "query_params": request.query_params,
            "path_params": request.path_params,
            "client_ip": request.client_ip,
            "user_agent": request.user_agent,
        });

        let result = self.call_wasm_json("generate_response", input)?;
        serde_json::from_value(result)
            .map_err(|e| PluginError::execution(format!("Failed to parse ResponseData: {}", e)))
    }

    async fn call_datasource_query(
        &self,
        query: &DataQuery,
        context: &PluginContext,
    ) -> Result<DataResult, PluginError> {
        // DataQuery should be serializable, but create simplified version to be safe
        let input = serde_json::json!({
            "query_type": format!("{:?}", query.query_type),
            "query": query.query,
            "parameters": query.parameters,
            "limit": query.limit,
            "offset": query.offset,
            "context": context,
        });

        let result = self.call_wasm_json("query_datasource", input)?;
        serde_json::from_value(result)
            .map_err(|e| PluginError::execution(format!("Failed to parse DataResult: {}", e)))
    }

    async fn health_check(&self) -> Result<bool, PluginError> {
        Ok(true)
    }

    async fn cleanup(&mut self) -> Result<(), PluginError> {
        Ok(())
    }
}

// ============================================================================
// AssemblyScript Runtime Adapter
// ============================================================================

pub struct AssemblyScriptAdapter {
    plugin_id: PluginId,
    engine: Arc<Engine>,
    module: Module,
    runtime: Mutex<Option<WasmRuntime>>,
}

impl AssemblyScriptAdapter {
    pub fn new(plugin_id: PluginId, wasm_bytes: Vec<u8>) -> Result<Self, PluginError> {
        let engine = Arc::new(Engine::default());
        let module = Module::from_binary(&engine, &wasm_bytes)
            .map_err(|e| PluginError::execution(format!("Failed to load AssemblyScript WASM module: {}", e)))?;

        Ok(Self {
            plugin_id,
            engine,
            module,
            runtime: Mutex::new(None),
        })
    }

    /// Helper to call an AssemblyScript WASM function with JSON input/output
    /// AssemblyScript uses __new, __pin, __unpin for memory management
    fn call_wasm_json(
        &self,
        function_name: &str,
        input_data: serde_json::Value,
    ) -> Result<serde_json::Value, PluginError> {
        let mut runtime_guard = self.runtime.lock().unwrap();
        let runtime = runtime_guard.as_mut().ok_or_else(|| {
            PluginError::execution("Runtime not initialized. Call initialize() first.".to_string())
        })?;

        let input_json = serde_json::to_string(&input_data)
            .map_err(|e| PluginError::execution(format!("Failed to serialize input: {}", e)))?;

        let input_bytes = input_json.as_bytes();
        let input_len = input_bytes.len() as i32;

        // Get memory
        let memory = runtime
            .instance
            .get_memory(&mut runtime.store, "memory")
            .ok_or_else(|| PluginError::execution("AssemblyScript WASM module must export 'memory'".to_string()))?;

        // AssemblyScript uses __new for allocation
        // Signature: __new(size: usize, id: u32) -> usize
        // For strings, id is typically 1
        let new_func = runtime
            .instance
            .get_typed_func::<(i32, i32), i32>(&mut runtime.store, "__new")
            .map_err(|e| PluginError::execution(format!("Failed to get __new function (AssemblyScript specific): {}", e)))?;

        // Allocate memory for input (id=1 for string type)
        let input_ptr = new_func
            .call(&mut runtime.store, (input_len, 1))
            .map_err(|e| PluginError::execution(format!("Failed to allocate memory: {}", e)))?;

        // Pin the allocated memory to prevent GC
        if let Ok(pin_func) = runtime
            .instance
            .get_typed_func::<i32, i32>(&mut runtime.store, "__pin")
        {
            let _ = pin_func.call(&mut runtime.store, input_ptr);
        }

        // Write input to WASM memory
        memory
            .write(&mut runtime.store, input_ptr as usize, input_bytes)
            .map_err(|e| PluginError::execution(format!("Failed to write input: {}", e)))?;

        // Call the plugin function
        let plugin_func = runtime
            .instance
            .get_typed_func::<(i32, i32), (i32, i32)>(&mut runtime.store, function_name)
            .map_err(|e| {
                PluginError::execution(format!("Function '{}' not found: {}", function_name, e))
            })?;

        let (output_ptr, output_len) = plugin_func
            .call(&mut runtime.store, (input_ptr, input_len))
            .map_err(|e| {
                PluginError::execution(format!("Failed to call function '{}': {}", function_name, e))
            })?;

        // Read output from WASM memory
        let mut output_bytes = vec![0u8; output_len as usize];
        memory
            .read(&runtime.store, output_ptr as usize, &mut output_bytes)
            .map_err(|e| PluginError::execution(format!("Failed to read output: {}", e)))?;

        // Unpin the allocated memory
        if let Ok(unpin_func) = runtime
            .instance
            .get_typed_func::<i32, ()>(&mut runtime.store, "__unpin")
        {
            let _ = unpin_func.call(&mut runtime.store, input_ptr);
            let _ = unpin_func.call(&mut runtime.store, output_ptr);
        }

        // Parse output as JSON
        let output_str = String::from_utf8(output_bytes)
            .map_err(|e| PluginError::execution(format!("Failed to decode output: {}", e)))?;

        serde_json::from_str(&output_str)
            .map_err(|e| PluginError::execution(format!("Failed to parse output JSON: {}", e)))
    }
}

#[async_trait]
impl RuntimeAdapter for AssemblyScriptAdapter {
    fn runtime_type(&self) -> RuntimeType {
        RuntimeType::AssemblyScript
    }

    async fn initialize(&mut self) -> Result<(), PluginError> {
        tracing::info!("Initializing AssemblyScript plugin: {}", self.plugin_id);

        // Create WASI context (AssemblyScript may use WASI features)
        let wasi_ctx = WasiCtxBuilder::new()
            .inherit_stderr()
            .inherit_stdout()
            .build();

        // Create store
        let mut store = Store::new(&self.engine, wasi_ctx);

        // Create linker
        let linker = Linker::new(&self.engine);

        // AssemblyScript modules typically don't require special imports
        // They use standard WASM with memory management functions

        // Instantiate the module
        let instance = linker
            .instantiate(&mut store, &self.module)
            .map_err(|e| PluginError::execution(format!("Failed to instantiate AssemblyScript module: {}", e)))?;

        // Store the runtime
        let mut runtime_guard = self.runtime.lock().unwrap();
        *runtime_guard = Some(WasmRuntime { store, instance });

        tracing::info!("Successfully initialized AssemblyScript plugin: {}", self.plugin_id);
        Ok(())
    }

    async fn call_auth(
        &self,
        context: &PluginContext,
        request: &AuthRequest,
    ) -> Result<AuthResponse, PluginError> {
        let input = serde_json::json!({
            "context": context,
            "method": request.method.to_string(),
            "uri": request.uri.to_string(),
            "query_params": request.query_params,
            "client_ip": request.client_ip,
            "user_agent": request.user_agent,
        });

        let result = self.call_wasm_json("authenticate", input)?;
        serde_json::from_value(result)
            .map_err(|e| PluginError::execution(format!("Failed to parse AuthResponse: {}", e)))
    }

    async fn call_template_function(
        &self,
        function_name: &str,
        args: &[serde_json::Value],
        context: &ResolutionContext,
    ) -> Result<serde_json::Value, PluginError> {
        let input = serde_json::json!({
            "function_name": function_name,
            "args": args,
            "context": context,
        });

        self.call_wasm_json("template_function", input)
    }

    async fn call_response_generator(
        &self,
        context: &PluginContext,
        request: &ResponseRequest,
    ) -> Result<ResponseData, PluginError> {
        // Create simplified request for WASM (avoid non-serializable types)
        let input = serde_json::json!({
            "context": context,
            "method": request.method.to_string(),
            "uri": request.uri,
            "path": request.path,
            "query_params": request.query_params,
            "path_params": request.path_params,
            "client_ip": request.client_ip,
            "user_agent": request.user_agent,
        });

        let result = self.call_wasm_json("generate_response", input)?;
        serde_json::from_value(result)
            .map_err(|e| PluginError::execution(format!("Failed to parse ResponseData: {}", e)))
    }

    async fn call_datasource_query(
        &self,
        query: &DataQuery,
        context: &PluginContext,
    ) -> Result<DataResult, PluginError> {
        // DataQuery should be serializable, but create simplified version to be safe
        let input = serde_json::json!({
            "query_type": format!("{:?}", query.query_type),
            "query": query.query,
            "parameters": query.parameters,
            "limit": query.limit,
            "offset": query.offset,
            "context": context,
        });

        let result = self.call_wasm_json("query_datasource", input)?;
        serde_json::from_value(result)
            .map_err(|e| PluginError::execution(format!("Failed to parse DataResult: {}", e)))
    }

    async fn health_check(&self) -> Result<bool, PluginError> {
        Ok(true)
    }

    async fn cleanup(&mut self) -> Result<(), PluginError> {
        Ok(())
    }
}

// ============================================================================
// Remote Runtime Adapter (HTTP/gRPC)
// ============================================================================

pub struct RemoteAdapter {
    plugin_id: PluginId,
    config: RemoteRuntimeConfig,
    client: reqwest::Client,
}

impl RemoteAdapter {
    pub fn new(plugin_id: PluginId, config: RemoteRuntimeConfig) -> Result<Self, PluginError> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(config.timeout_ms))
            .build()
            .map_err(|e| PluginError::execution(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            plugin_id,
            config,
            client,
        })
    }

    async fn call_remote_plugin(
        &self,
        endpoint: &str,
        body: serde_json::Value,
    ) -> Result<serde_json::Value, PluginError> {
        let url = format!("{}{}", self.config.endpoint, endpoint);

        let mut request = self.client.post(&url).json(&body);

        // Add authentication if configured
        if let Some(auth) = &self.config.auth {
            request = match auth.auth_type.as_str() {
                "bearer" => request.bearer_auth(&auth.value),
                "api_key" => request.header("X-API-Key", &auth.value),
                _ => request,
            };
        }

        let response = request
            .send()
            .await
            .map_err(|e| PluginError::execution(format!("Remote plugin call failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(PluginError::execution(format!(
                "Remote plugin returned error status: {}",
                response.status()
            )));
        }

        let result: serde_json::Value = response
            .json()
            .await
            .map_err(|e| PluginError::execution(format!("Failed to parse response: {}", e)))?;

        Ok(result)
    }
}

#[async_trait]
impl RuntimeAdapter for RemoteAdapter {
    fn runtime_type(&self) -> RuntimeType {
        RuntimeType::Remote(self.config.clone())
    }

    async fn initialize(&mut self) -> Result<(), PluginError> {
        tracing::info!("Initializing remote plugin: {}", self.plugin_id);

        // Perform health check during initialization
        self.health_check().await?;

        Ok(())
    }

    async fn call_auth(
        &self,
        context: &PluginContext,
        request: &AuthRequest,
    ) -> Result<AuthResponse, PluginError> {
        let body = serde_json::json!({
            "context": context,
            "method": request.method.to_string(),
            "uri": request.uri.to_string(),
            "query_params": request.query_params,
            "client_ip": request.client_ip,
            "user_agent": request.user_agent,
        });

        let result = self.call_remote_plugin("/plugin/authenticate", body).await?;

        // Parse the AuthResponse from the response
        serde_json::from_value(result)
            .map_err(|e| PluginError::execution(format!("Failed to parse AuthResponse: {}", e)))
    }

    async fn call_template_function(
        &self,
        function_name: &str,
        args: &[serde_json::Value],
        context: &ResolutionContext,
    ) -> Result<serde_json::Value, PluginError> {
        let body = serde_json::json!({
            "function_name": function_name,
            "args": args,
            "context": context,
        });

        self.call_remote_plugin("/plugin/template/execute", body).await
    }

    async fn call_response_generator(
        &self,
        context: &PluginContext,
        request: &ResponseRequest,
    ) -> Result<ResponseData, PluginError> {
        // Create simplified request
        let body = serde_json::json!({
            "context": context,
            "method": request.method.to_string(),
            "uri": request.uri,
            "path": request.path,
            "query_params": request.query_params,
            "path_params": request.path_params,
            "client_ip": request.client_ip,
            "user_agent": request.user_agent,
        });

        let result = self.call_remote_plugin("/plugin/response/generate", body).await?;

        serde_json::from_value(result)
            .map_err(|e| PluginError::execution(format!("Failed to parse ResponseData: {}", e)))
    }

    async fn call_datasource_query(
        &self,
        query: &DataQuery,
        context: &PluginContext,
    ) -> Result<DataResult, PluginError> {
        let body = serde_json::json!({
            "query_type": format!("{:?}", query.query_type),
            "query": query.query,
            "parameters": query.parameters,
            "limit": query.limit,
            "offset": query.offset,
            "context": context,
        });

        let result = self.call_remote_plugin("/plugin/datasource/query", body).await?;

        serde_json::from_value(result)
            .map_err(|e| PluginError::execution(format!("Failed to parse DataResult: {}", e)))
    }

    async fn health_check(&self) -> Result<bool, PluginError> {
        // Try to call health endpoint
        let url = format!("{}/health", self.config.endpoint);

        match self.client.get(&url).send().await {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    async fn cleanup(&mut self) -> Result<(), PluginError> {
        tracing::info!("Cleaning up remote plugin: {}", self.plugin_id);
        Ok(())
    }

    fn get_metrics(&self) -> HashMap<String, serde_json::Value> {
        let mut metrics = HashMap::new();
        metrics.insert("plugin_id".to_string(), serde_json::json!(self.plugin_id.as_str()));
        metrics.insert("endpoint".to_string(), serde_json::json!(self.config.endpoint));
        metrics.insert("protocol".to_string(), serde_json::json!(format!("{:?}", self.config.protocol)));
        metrics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_type_detection() {
        // Test with empty bytes (should default to Rust)
        let empty_bytes = vec![];
        let runtime = detect_runtime_type(&empty_bytes).unwrap();
        assert_eq!(runtime, RuntimeType::Rust);
    }

    #[test]
    fn test_remote_runtime_config() {
        let config = RemoteRuntimeConfig {
            protocol: RemoteProtocol::Http,
            endpoint: "http://localhost:8080".to_string(),
            timeout_ms: 5000,
            max_retries: 3,
            auth: Some(RemoteAuthConfig {
                auth_type: "bearer".to_string(),
                value: "secret-token".to_string(),
            }),
        };

        assert_eq!(config.endpoint, "http://localhost:8080");
        assert_eq!(config.timeout_ms, 5000);
    }
}
