//! Prelude module for convenient imports
//!
//! Import everything you need with:
//! ```rust
//! use mockforge_plugin_sdk::prelude::*;
//! ```

// Re-export core plugin traits and types
pub use mockforge_plugin_core::{
    // Auth plugin types
    AuthPlugin,
    AuthPluginConfig,
    AuthRequest,
    AuthResponse,
    FunctionParameter,
    // Common types
    PluginCapabilities,
    PluginContext,
    PluginError,
    PluginId,
    PluginInfo,
    PluginManifest,
    PluginResult,
    PluginVersion,
    TemplateFunction,
    // Template plugin types
    TemplatePlugin,
    TemplatePluginConfig,
    UserIdentity,
};

// Re-export response plugin types from their module
pub use mockforge_plugin_core::response::{
    ResponseData, ResponsePlugin, ResponsePluginConfig, ResponseRequest,
};

// Re-export datasource plugin types from their module
pub use mockforge_plugin_core::datasource::{
    ColumnInfo, DataConnection, DataQuery, DataResult, DataRow, DataSourcePlugin,
    DataSourcePluginConfig, Schema, TableInfo,
};

// Re-export async trait
pub use async_trait::async_trait;

// Re-export common types
pub use serde::{Deserialize, Serialize};
pub use serde_json::{json, Value};
pub use std::collections::HashMap;

// Re-export SDK utilities
pub use crate::builders::*;

#[cfg(feature = "testing")]
pub use crate::testing::*;

// Re-export error types
pub use crate::{SdkError, SdkResult};

// Common utilities
pub use anyhow::{anyhow, Context, Result};
pub use uuid::Uuid;
