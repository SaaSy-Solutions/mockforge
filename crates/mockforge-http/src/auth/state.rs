//! Authentication state management
//!
//! This module handles the authentication state containing configuration
//! and runtime data needed for authentication operations.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use mockforge_core::{config::AuthConfig, OpenApiSpec};
use super::types::AuthResult;

/// Cached OAuth2 introspection result
#[derive(Debug, Clone)]
pub struct CachedIntrospection {
    /// The introspection result
    pub result: AuthResult,
    /// When this cache entry expires (Unix timestamp)
    pub expires_at: i64,
}

/// Authentication middleware state
#[derive(Clone)]
pub struct AuthState {
    pub config: AuthConfig,
    pub spec: Option<Arc<OpenApiSpec>>,
    pub oauth2_client: Option<oauth2::basic::BasicClient>,
    /// Cache for OAuth2 token introspection results
    pub introspection_cache: Arc<RwLock<HashMap<String, CachedIntrospection>>>,
}
