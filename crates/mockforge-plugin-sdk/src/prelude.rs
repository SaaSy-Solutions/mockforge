//! Prelude module for convenient imports
//!
//! Import everything you need with:
//! ```rust
//! use mockforge_plugin_sdk::prelude::*;
//! ```

// Re-export core plugin traits and types
pub use mockforge_plugin_core::{
    // Auth plugin types
    AuthPlugin, AuthPluginConfig, AuthRequest, AuthResponse, UserIdentity,
    // Template plugin types
    TemplatePlugin, TemplatePluginConfig, TemplateFunction, FunctionParameter,
    // Response plugin types
    ResponsePlugin, ResponsePluginConfig, ResponseRequest, ResponseData,
    // DataSource plugin types
    DataSourcePlugin, DataSourcePluginConfig, DataConnection, DataQuery,
    DataResult, DataRow, ColumnInfo, Schema, TableInfo,
    // Common types
    PluginCapabilities, PluginContext, PluginResult,
    PluginError, PluginId, PluginVersion, PluginInfo, PluginManifest,
};

// Re-export async trait
pub use async_trait::async_trait;

// Re-export common types
pub use serde::{Deserialize, Serialize};
pub use serde_json::{json, Value};
pub use std::collections::HashMap;

// Re-export SDK utilities
pub use crate::builders::*;
pub use crate::macros::*;

#[cfg(feature = "testing")]
pub use crate::testing::*;

// Re-export error types
pub use crate::{SdkError, SdkResult};

// Common utilities
pub use anyhow::{anyhow, Context, Result};
pub use uuid::Uuid;
