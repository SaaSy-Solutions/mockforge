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
}
