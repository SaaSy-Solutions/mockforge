//! Lifecycle hooks for extensibility
//!
//! This module provides a comprehensive lifecycle hook system that allows
//! extensions to hook into various lifecycle events in MockForge:
//!
//! - Request/Response lifecycle: before_request, after_response
//! - Server lifecycle: on_startup, on_shutdown
//! - Mock lifecycle: on_mock_created, on_mock_updated, on_mock_deleted
//!
//! # Examples
//!
//! ```rust
//! use mockforge_core::lifecycle::{LifecycleHook, RequestContext, ResponseContext};
//! use async_trait::async_trait;
//!
//! struct LoggingHook;
//!
//! #[async_trait]
//! impl LifecycleHook for LoggingHook {
//!     async fn before_request(&self, ctx: &RequestContext) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//!         println!("Request: {} {}", ctx.method, ctx.path);
//!         Ok(())
//!     }
//!
//!     async fn after_response(&self, ctx: &ResponseContext) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//!         println!("Response: {} in {}ms", ctx.status_code, ctx.response_time_ms);
//!         Ok(())
//!     }
//! }
//! ```

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Request context for lifecycle hooks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestContext {
    /// HTTP method
    pub method: String,
    /// Request path
    pub path: String,
    /// Request headers
    pub headers: HashMap<String, String>,
    /// Query parameters
    pub query_params: HashMap<String, String>,
    /// Request body (if available)
    pub body: Option<Vec<u8>>,
    /// Request ID for tracking
    pub request_id: String,
    /// Timestamp when request was received
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Response context for lifecycle hooks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseContext {
    /// Request context
    pub request: RequestContext,
    /// HTTP status code
    pub status_code: u16,
    /// Response headers
    pub headers: HashMap<String, String>,
    /// Response body (if available)
    pub body: Option<Vec<u8>>,
    /// Response time in milliseconds
    pub response_time_ms: u64,
    /// Timestamp when response was sent
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Mock lifecycle event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MockLifecycleEvent {
    /// Mock was created
    Created {
        /// Mock ID
        id: String,
        /// Mock name
        name: String,
        /// Mock configuration (serialized)
        config: serde_json::Value,
    },
    /// Mock was updated
    Updated {
        /// Mock ID
        id: String,
        /// Mock name
        name: String,
        /// Updated mock configuration (serialized)
        config: serde_json::Value,
    },
    /// Mock was deleted
    Deleted {
        /// Mock ID
        id: String,
        /// Mock name
        name: String,
    },
    /// Mock was enabled
    Enabled {
        /// Mock ID
        id: String,
    },
    /// Mock was disabled
    Disabled {
        /// Mock ID
        id: String,
    },
}

/// Server lifecycle event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerLifecycleEvent {
    /// Server is starting up
    Startup {
        /// Server configuration
        config: serde_json::Value,
    },
    /// Server is shutting down
    Shutdown {
        /// Shutdown reason
        reason: String,
    },
}

/// Comprehensive lifecycle hook trait
///
/// Implement this trait to hook into various lifecycle events in MockForge.
/// All methods have default no-op implementations, so you only need to
/// implement the hooks you care about.
#[async_trait]
pub trait LifecycleHook: Send + Sync {
    /// Called before a request is processed
    ///
    /// This hook is called after the request is received but before it's
    /// matched against mocks or processed. You can use this to:
    /// - Log requests
    /// - Modify request headers
    /// - Add request metadata
    /// - Perform authentication checks
    async fn before_request(
        &self,
        _ctx: &RequestContext,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    /// Called after a response is generated
    ///
    /// This hook is called after the response is generated but before it's
    /// sent to the client. You can use this to:
    /// - Log responses
    /// - Modify response headers
    /// - Add response metadata
    /// - Perform response validation
    async fn after_response(
        &self,
        _ctx: &ResponseContext,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    /// Called when a mock is created
    async fn on_mock_created(
        &self,
        _event: &MockLifecycleEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    /// Called when a mock is updated
    async fn on_mock_updated(
        &self,
        _event: &MockLifecycleEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    /// Called when a mock is deleted
    async fn on_mock_deleted(
        &self,
        _event: &MockLifecycleEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    /// Called when a mock is enabled or disabled
    async fn on_mock_state_changed(
        &self,
        _event: &MockLifecycleEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    /// Called when the server starts up
    async fn on_startup(
        &self,
        _event: &ServerLifecycleEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    /// Called when the server shuts down
    async fn on_shutdown(
        &self,
        _event: &ServerLifecycleEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }
}

/// Lifecycle hook registry
///
/// Manages all registered lifecycle hooks and provides methods to invoke them.
pub struct LifecycleHookRegistry {
    hooks: Arc<RwLock<Vec<Arc<dyn LifecycleHook>>>>,
}

impl LifecycleHookRegistry {
    /// Create a new lifecycle hook registry
    pub fn new() -> Self {
        Self {
            hooks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Register a lifecycle hook
    pub async fn register_hook(&self, hook: Arc<dyn LifecycleHook>) {
        let mut hooks = self.hooks.write().await;
        hooks.push(hook);
    }

    /// Invoke all registered before_request hooks
    pub async fn invoke_before_request(&self, ctx: &RequestContext) {
        let hooks = self.hooks.read().await;
        for hook in hooks.iter() {
            if let Err(e) = hook.before_request(ctx).await {
                tracing::error!("Error in before_request hook: {}", e);
            }
        }
    }

    /// Invoke all registered after_response hooks
    pub async fn invoke_after_response(&self, ctx: &ResponseContext) {
        let hooks = self.hooks.read().await;
        for hook in hooks.iter() {
            if let Err(e) = hook.after_response(ctx).await {
                tracing::error!("Error in after_response hook: {}", e);
            }
        }
    }

    /// Invoke all registered on_mock_created hooks
    pub async fn invoke_mock_created(&self, event: &MockLifecycleEvent) {
        let hooks = self.hooks.read().await;
        for hook in hooks.iter() {
            if let Err(e) = hook.on_mock_created(event).await {
                tracing::error!("Error in on_mock_created hook: {}", e);
            }
        }
    }

    /// Invoke all registered on_mock_updated hooks
    pub async fn invoke_mock_updated(&self, event: &MockLifecycleEvent) {
        let hooks = self.hooks.read().await;
        for hook in hooks.iter() {
            if let Err(e) = hook.on_mock_updated(event).await {
                tracing::error!("Error in on_mock_updated hook: {}", e);
            }
        }
    }

    /// Invoke all registered on_mock_deleted hooks
    pub async fn invoke_mock_deleted(&self, event: &MockLifecycleEvent) {
        let hooks = self.hooks.read().await;
        for hook in hooks.iter() {
            if let Err(e) = hook.on_mock_deleted(event).await {
                tracing::error!("Error in on_mock_deleted hook: {}", e);
            }
        }
    }

    /// Invoke all registered on_mock_state_changed hooks
    pub async fn invoke_mock_state_changed(&self, event: &MockLifecycleEvent) {
        let hooks = self.hooks.read().await;
        for hook in hooks.iter() {
            if let Err(e) = hook.on_mock_state_changed(event).await {
                tracing::error!("Error in on_mock_state_changed hook: {}", e);
            }
        }
    }

    /// Invoke all registered on_startup hooks
    pub async fn invoke_startup(&self, event: &ServerLifecycleEvent) {
        let hooks = self.hooks.read().await;
        for hook in hooks.iter() {
            if let Err(e) = hook.on_startup(event).await {
                tracing::error!("Error in on_startup hook: {}", e);
            }
        }
    }

    /// Invoke all registered on_shutdown hooks
    pub async fn invoke_shutdown(&self, event: &ServerLifecycleEvent) {
        let hooks = self.hooks.read().await;
        for hook in hooks.iter() {
            if let Err(e) = hook.on_shutdown(event).await {
                tracing::error!("Error in on_shutdown hook: {}", e);
            }
        }
    }
}

impl Default for LifecycleHookRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn create_request_context() -> RequestContext {
        RequestContext {
            method: "GET".to_string(),
            path: "/test".to_string(),
            headers: {
                let mut h = HashMap::new();
                h.insert("Content-Type".to_string(), "application/json".to_string());
                h
            },
            query_params: {
                let mut q = HashMap::new();
                q.insert("page".to_string(), "1".to_string());
                q
            },
            body: Some(b"test body".to_vec()),
            request_id: "test-123".to_string(),
            timestamp: chrono::Utc::now(),
        }
    }

    fn create_response_context() -> ResponseContext {
        ResponseContext {
            request: create_request_context(),
            status_code: 200,
            headers: {
                let mut h = HashMap::new();
                h.insert("Content-Type".to_string(), "application/json".to_string());
                h
            },
            body: Some(b"response body".to_vec()),
            response_time_ms: 50,
            timestamp: chrono::Utc::now(),
        }
    }

    // RequestContext tests
    #[test]
    fn test_request_context_debug() {
        let ctx = create_request_context();
        let debug = format!("{:?}", ctx);
        assert!(debug.contains("RequestContext"));
        assert!(debug.contains("GET"));
        assert!(debug.contains("/test"));
    }

    #[test]
    fn test_request_context_clone() {
        let ctx = create_request_context();
        let cloned = ctx.clone();
        assert_eq!(cloned.method, ctx.method);
        assert_eq!(cloned.path, ctx.path);
        assert_eq!(cloned.request_id, ctx.request_id);
    }

    #[test]
    fn test_request_context_serialize_deserialize() {
        let ctx = create_request_context();
        let json = serde_json::to_string(&ctx).unwrap();
        assert!(json.contains("GET"));
        assert!(json.contains("/test"));

        let deserialized: RequestContext = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.method, ctx.method);
        assert_eq!(deserialized.path, ctx.path);
    }

    // ResponseContext tests
    #[test]
    fn test_response_context_debug() {
        let ctx = create_response_context();
        let debug = format!("{:?}", ctx);
        assert!(debug.contains("ResponseContext"));
        assert!(debug.contains("200"));
    }

    #[test]
    fn test_response_context_clone() {
        let ctx = create_response_context();
        let cloned = ctx.clone();
        assert_eq!(cloned.status_code, ctx.status_code);
        assert_eq!(cloned.response_time_ms, ctx.response_time_ms);
    }

    #[test]
    fn test_response_context_serialize_deserialize() {
        let ctx = create_response_context();
        let json = serde_json::to_string(&ctx).unwrap();
        assert!(json.contains("200"));

        let deserialized: ResponseContext = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.status_code, ctx.status_code);
    }

    // MockLifecycleEvent tests
    #[test]
    fn test_mock_lifecycle_event_created() {
        let event = MockLifecycleEvent::Created {
            id: "mock-1".to_string(),
            name: "Test Mock".to_string(),
            config: json!({"endpoint": "/api/test"}),
        };

        let debug = format!("{:?}", event);
        assert!(debug.contains("Created"));
        assert!(debug.contains("mock-1"));
    }

    #[test]
    fn test_mock_lifecycle_event_updated() {
        let event = MockLifecycleEvent::Updated {
            id: "mock-1".to_string(),
            name: "Updated Mock".to_string(),
            config: json!({"endpoint": "/api/updated"}),
        };

        let debug = format!("{:?}", event);
        assert!(debug.contains("Updated"));
    }

    #[test]
    fn test_mock_lifecycle_event_deleted() {
        let event = MockLifecycleEvent::Deleted {
            id: "mock-1".to_string(),
            name: "Deleted Mock".to_string(),
        };

        let debug = format!("{:?}", event);
        assert!(debug.contains("Deleted"));
    }

    #[test]
    fn test_mock_lifecycle_event_enabled() {
        let event = MockLifecycleEvent::Enabled {
            id: "mock-1".to_string(),
        };

        let debug = format!("{:?}", event);
        assert!(debug.contains("Enabled"));
    }

    #[test]
    fn test_mock_lifecycle_event_disabled() {
        let event = MockLifecycleEvent::Disabled {
            id: "mock-1".to_string(),
        };

        let debug = format!("{:?}", event);
        assert!(debug.contains("Disabled"));
    }

    #[test]
    fn test_mock_lifecycle_event_clone() {
        let event = MockLifecycleEvent::Created {
            id: "mock-1".to_string(),
            name: "Test Mock".to_string(),
            config: json!({}),
        };
        let cloned = event.clone();
        if let MockLifecycleEvent::Created { id, .. } = cloned {
            assert_eq!(id, "mock-1");
        }
    }

    #[test]
    fn test_mock_lifecycle_event_serialize_deserialize() {
        let event = MockLifecycleEvent::Created {
            id: "mock-1".to_string(),
            name: "Test Mock".to_string(),
            config: json!({"key": "value"}),
        };
        let json = serde_json::to_string(&event).unwrap();
        let deserialized: MockLifecycleEvent = serde_json::from_str(&json).unwrap();
        if let MockLifecycleEvent::Created { id, name, .. } = deserialized {
            assert_eq!(id, "mock-1");
            assert_eq!(name, "Test Mock");
        }
    }

    // ServerLifecycleEvent tests
    #[test]
    fn test_server_lifecycle_event_startup() {
        let event = ServerLifecycleEvent::Startup {
            config: json!({"port": 8080}),
        };
        let debug = format!("{:?}", event);
        assert!(debug.contains("Startup"));
    }

    #[test]
    fn test_server_lifecycle_event_shutdown() {
        let event = ServerLifecycleEvent::Shutdown {
            reason: "User requested".to_string(),
        };
        let debug = format!("{:?}", event);
        assert!(debug.contains("Shutdown"));
        assert!(debug.contains("User requested"));
    }

    #[test]
    fn test_server_lifecycle_event_clone() {
        let event = ServerLifecycleEvent::Startup { config: json!({}) };
        let cloned = event.clone();
        if let ServerLifecycleEvent::Startup { .. } = cloned {
            // Clone successful
        } else {
            panic!("Clone failed");
        }
    }

    #[test]
    fn test_server_lifecycle_event_serialize_deserialize() {
        let event = ServerLifecycleEvent::Shutdown {
            reason: "Test shutdown".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let deserialized: ServerLifecycleEvent = serde_json::from_str(&json).unwrap();
        if let ServerLifecycleEvent::Shutdown { reason } = deserialized {
            assert_eq!(reason, "Test shutdown");
        }
    }

    // LifecycleHookRegistry tests
    #[tokio::test]
    async fn test_registry_new() {
        let registry = LifecycleHookRegistry::new();
        // Registry should be created successfully
        let _ = registry;
    }

    #[tokio::test]
    async fn test_registry_default() {
        let registry = LifecycleHookRegistry::default();
        // Default registry should work the same as new
        let _ = registry;
    }

    struct TestHook {
        before_request_called: Arc<RwLock<bool>>,
        after_response_called: Arc<RwLock<bool>>,
    }

    #[async_trait]
    impl LifecycleHook for TestHook {
        async fn before_request(
            &self,
            _ctx: &RequestContext,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            *self.before_request_called.write().await = true;
            Ok(())
        }

        async fn after_response(
            &self,
            _ctx: &ResponseContext,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            *self.after_response_called.write().await = true;
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_lifecycle_hooks() {
        let registry = LifecycleHookRegistry::new();
        let before_called = Arc::new(RwLock::new(false));
        let after_called = Arc::new(RwLock::new(false));

        let hook = Arc::new(TestHook {
            before_request_called: before_called.clone(),
            after_response_called: after_called.clone(),
        });

        registry.register_hook(hook).await;

        let request_ctx = RequestContext {
            method: "GET".to_string(),
            path: "/test".to_string(),
            headers: HashMap::new(),
            query_params: HashMap::new(),
            body: None,
            request_id: "test-1".to_string(),
            timestamp: chrono::Utc::now(),
        };

        registry.invoke_before_request(&request_ctx).await;
        assert!(*before_called.read().await);

        let response_ctx = ResponseContext {
            request: request_ctx,
            status_code: 200,
            headers: HashMap::new(),
            body: None,
            response_time_ms: 10,
            timestamp: chrono::Utc::now(),
        };

        registry.invoke_after_response(&response_ctx).await;
        assert!(*after_called.read().await);
    }

    // Test multiple hooks
    struct CountingHook {
        call_count: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl LifecycleHook for CountingHook {
        async fn before_request(
            &self,
            _ctx: &RequestContext,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_multiple_hooks() {
        let registry = LifecycleHookRegistry::new();
        let count1 = Arc::new(AtomicUsize::new(0));
        let count2 = Arc::new(AtomicUsize::new(0));

        let hook1 = Arc::new(CountingHook {
            call_count: count1.clone(),
        });
        let hook2 = Arc::new(CountingHook {
            call_count: count2.clone(),
        });

        registry.register_hook(hook1).await;
        registry.register_hook(hook2).await;

        let request_ctx = create_request_context();
        registry.invoke_before_request(&request_ctx).await;

        assert_eq!(count1.load(Ordering::SeqCst), 1);
        assert_eq!(count2.load(Ordering::SeqCst), 1);
    }

    // Test error handling in hooks
    struct ErrorHook;

    #[async_trait]
    impl LifecycleHook for ErrorHook {
        async fn before_request(
            &self,
            _ctx: &RequestContext,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            Err("Test error".into())
        }

        async fn on_mock_created(
            &self,
            _event: &MockLifecycleEvent,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            Err("Mock creation error".into())
        }

        async fn on_startup(
            &self,
            _event: &ServerLifecycleEvent,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            Err("Startup error".into())
        }
    }

    #[tokio::test]
    async fn test_error_handling_in_hooks() {
        let registry = LifecycleHookRegistry::new();
        registry.register_hook(Arc::new(ErrorHook)).await;

        // Should not panic, just log the error
        let request_ctx = create_request_context();
        registry.invoke_before_request(&request_ctx).await;
    }

    // Test mock lifecycle hooks
    struct MockLifecycleTestHook {
        created_called: Arc<RwLock<bool>>,
        updated_called: Arc<RwLock<bool>>,
        deleted_called: Arc<RwLock<bool>>,
        state_changed_called: Arc<RwLock<bool>>,
    }

    #[async_trait]
    impl LifecycleHook for MockLifecycleTestHook {
        async fn on_mock_created(
            &self,
            _event: &MockLifecycleEvent,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            *self.created_called.write().await = true;
            Ok(())
        }

        async fn on_mock_updated(
            &self,
            _event: &MockLifecycleEvent,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            *self.updated_called.write().await = true;
            Ok(())
        }

        async fn on_mock_deleted(
            &self,
            _event: &MockLifecycleEvent,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            *self.deleted_called.write().await = true;
            Ok(())
        }

        async fn on_mock_state_changed(
            &self,
            _event: &MockLifecycleEvent,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            *self.state_changed_called.write().await = true;
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_mock_lifecycle_hooks() {
        let registry = LifecycleHookRegistry::new();
        let hook = Arc::new(MockLifecycleTestHook {
            created_called: Arc::new(RwLock::new(false)),
            updated_called: Arc::new(RwLock::new(false)),
            deleted_called: Arc::new(RwLock::new(false)),
            state_changed_called: Arc::new(RwLock::new(false)),
        });
        let hook_clone = hook.clone();

        registry.register_hook(hook).await;

        // Test mock created
        let created_event = MockLifecycleEvent::Created {
            id: "mock-1".to_string(),
            name: "Test".to_string(),
            config: json!({}),
        };
        registry.invoke_mock_created(&created_event).await;
        assert!(*hook_clone.created_called.read().await);

        // Test mock updated
        let updated_event = MockLifecycleEvent::Updated {
            id: "mock-1".to_string(),
            name: "Test".to_string(),
            config: json!({}),
        };
        registry.invoke_mock_updated(&updated_event).await;
        assert!(*hook_clone.updated_called.read().await);

        // Test mock deleted
        let deleted_event = MockLifecycleEvent::Deleted {
            id: "mock-1".to_string(),
            name: "Test".to_string(),
        };
        registry.invoke_mock_deleted(&deleted_event).await;
        assert!(*hook_clone.deleted_called.read().await);

        // Test mock state changed
        let state_event = MockLifecycleEvent::Enabled {
            id: "mock-1".to_string(),
        };
        registry.invoke_mock_state_changed(&state_event).await;
        assert!(*hook_clone.state_changed_called.read().await);
    }

    // Test server lifecycle hooks
    struct ServerLifecycleTestHook {
        startup_called: Arc<RwLock<bool>>,
        shutdown_called: Arc<RwLock<bool>>,
    }

    #[async_trait]
    impl LifecycleHook for ServerLifecycleTestHook {
        async fn on_startup(
            &self,
            _event: &ServerLifecycleEvent,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            *self.startup_called.write().await = true;
            Ok(())
        }

        async fn on_shutdown(
            &self,
            _event: &ServerLifecycleEvent,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            *self.shutdown_called.write().await = true;
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_server_lifecycle_hooks() {
        let registry = LifecycleHookRegistry::new();
        let hook = Arc::new(ServerLifecycleTestHook {
            startup_called: Arc::new(RwLock::new(false)),
            shutdown_called: Arc::new(RwLock::new(false)),
        });
        let hook_clone = hook.clone();

        registry.register_hook(hook).await;

        // Test startup
        let startup_event = ServerLifecycleEvent::Startup {
            config: json!({"port": 8080}),
        };
        registry.invoke_startup(&startup_event).await;
        assert!(*hook_clone.startup_called.read().await);

        // Test shutdown
        let shutdown_event = ServerLifecycleEvent::Shutdown {
            reason: "Test shutdown".to_string(),
        };
        registry.invoke_shutdown(&shutdown_event).await;
        assert!(*hook_clone.shutdown_called.read().await);
    }

    // Test default trait implementations
    struct EmptyHook;

    #[async_trait]
    impl LifecycleHook for EmptyHook {}

    #[tokio::test]
    async fn test_default_hook_implementations() {
        let hook = EmptyHook;

        // All these should succeed with the default no-op implementations
        let request_ctx = create_request_context();
        assert!(hook.before_request(&request_ctx).await.is_ok());

        let response_ctx = create_response_context();
        assert!(hook.after_response(&response_ctx).await.is_ok());

        let mock_event = MockLifecycleEvent::Created {
            id: "test".to_string(),
            name: "Test".to_string(),
            config: json!({}),
        };
        assert!(hook.on_mock_created(&mock_event).await.is_ok());
        assert!(hook.on_mock_updated(&mock_event).await.is_ok());
        assert!(hook.on_mock_deleted(&mock_event).await.is_ok());
        assert!(hook.on_mock_state_changed(&mock_event).await.is_ok());

        let server_event = ServerLifecycleEvent::Startup { config: json!({}) };
        assert!(hook.on_startup(&server_event).await.is_ok());
        assert!(hook.on_shutdown(&server_event).await.is_ok());
    }

    // Test empty registry
    #[tokio::test]
    async fn test_empty_registry() {
        let registry = LifecycleHookRegistry::new();
        let request_ctx = create_request_context();
        let response_ctx = create_response_context();

        // These should not panic on empty registry
        registry.invoke_before_request(&request_ctx).await;
        registry.invoke_after_response(&response_ctx).await;

        let mock_event = MockLifecycleEvent::Created {
            id: "test".to_string(),
            name: "Test".to_string(),
            config: json!({}),
        };
        registry.invoke_mock_created(&mock_event).await;

        let server_event = ServerLifecycleEvent::Startup { config: json!({}) };
        registry.invoke_startup(&server_event).await;
    }

    // Test hook error doesn't stop other hooks
    #[tokio::test]
    async fn test_error_doesnt_stop_other_hooks() {
        let registry = LifecycleHookRegistry::new();
        let count = Arc::new(AtomicUsize::new(0));

        // Register error hook first
        registry.register_hook(Arc::new(ErrorHook)).await;

        // Register counting hook second
        let counting_hook = Arc::new(CountingHook {
            call_count: count.clone(),
        });
        registry.register_hook(counting_hook).await;

        let request_ctx = create_request_context();
        registry.invoke_before_request(&request_ctx).await;

        // Counting hook should still have been called despite error in first hook
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    // Test request context with all fields populated
    #[test]
    fn test_request_context_full() {
        let ctx = RequestContext {
            method: "POST".to_string(),
            path: "/api/users".to_string(),
            headers: {
                let mut h = HashMap::new();
                h.insert("Content-Type".to_string(), "application/json".to_string());
                h.insert("Authorization".to_string(), "Bearer token".to_string());
                h
            },
            query_params: {
                let mut q = HashMap::new();
                q.insert("page".to_string(), "1".to_string());
                q.insert("limit".to_string(), "10".to_string());
                q
            },
            body: Some(b"{\"name\": \"test\"}".to_vec()),
            request_id: "req-12345".to_string(),
            timestamp: chrono::Utc::now(),
        };

        assert_eq!(ctx.method, "POST");
        assert_eq!(ctx.path, "/api/users");
        assert_eq!(ctx.headers.len(), 2);
        assert_eq!(ctx.query_params.len(), 2);
        assert!(ctx.body.is_some());
        assert_eq!(ctx.request_id, "req-12345");
    }

    // Test response context with error response
    #[test]
    fn test_response_context_error() {
        let ctx = ResponseContext {
            request: create_request_context(),
            status_code: 500,
            headers: HashMap::new(),
            body: Some(b"{\"error\": \"Internal Server Error\"}".to_vec()),
            response_time_ms: 100,
            timestamp: chrono::Utc::now(),
        };

        assert_eq!(ctx.status_code, 500);
        assert!(ctx.body.is_some());
    }
}
