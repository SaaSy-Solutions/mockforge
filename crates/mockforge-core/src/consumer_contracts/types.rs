//! Core types for consumer-driven contracts

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Type of consumer identifier
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ConsumerType {
    /// Identified by workspace ID
    Workspace,
    /// Custom consumer ID
    Custom,
    /// Identified by API key
    ApiKey,
    /// Identified by authentication token
    AuthToken,
}

/// Consumer identifier
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ConsumerIdentifier {
    /// Type of identifier
    pub consumer_type: ConsumerType,
    /// Identifier value (workspace_id, custom_id, api_key, or token_hash)
    pub value: String,
}

impl ConsumerIdentifier {
    /// Create a workspace-based identifier
    pub fn workspace(workspace_id: String) -> Self {
        Self {
            consumer_type: ConsumerType::Workspace,
            value: workspace_id,
        }
    }

    /// Create a custom identifier
    pub fn custom(custom_id: String) -> Self {
        Self {
            consumer_type: ConsumerType::Custom,
            value: custom_id,
        }
    }

    /// Create an API key identifier
    pub fn api_key(api_key: String) -> Self {
        Self {
            consumer_type: ConsumerType::ApiKey,
            value: api_key,
        }
    }

    /// Create an auth token identifier (should be hashed)
    pub fn auth_token(token_hash: String) -> Self {
        Self {
            consumer_type: ConsumerType::AuthToken,
            value: token_hash,
        }
    }
}

/// A consumer of the API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Consumer {
    /// Unique identifier
    pub id: String,
    /// Consumer identifier
    pub identifier: ConsumerIdentifier,
    /// Consumer name
    pub name: String,
    /// Workspace ID (for multi-tenant support)
    pub workspace_id: Option<String>,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// When the consumer was created
    pub created_at: i64,
    /// When the consumer was last updated
    pub updated_at: i64,
}

impl Consumer {
    /// Create a new consumer
    pub fn new(
        id: String,
        identifier: ConsumerIdentifier,
        name: String,
        workspace_id: Option<String>,
    ) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id,
            identifier,
            name,
            workspace_id,
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }
}

/// Usage tracking for a consumer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumerUsage {
    /// Consumer ID
    pub consumer_id: String,
    /// Endpoint path
    pub endpoint: String,
    /// HTTP method
    pub method: String,
    /// Fields used (JSON path notation)
    pub fields_used: Vec<String>,
    /// Last time this endpoint was used
    pub last_used_at: i64,
    /// Number of times this endpoint was used
    pub usage_count: u64,
}

/// Consumer contract violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumerViolation {
    /// Violation ID
    pub id: String,
    /// Consumer ID
    pub consumer_id: String,
    /// Associated incident ID (if any)
    pub incident_id: Option<String>,
    /// Endpoint path
    pub endpoint: String,
    /// HTTP method
    pub method: String,
    /// Fields that were violated (removed or changed)
    pub violated_fields: Vec<String>,
    /// When the violation was detected
    pub detected_at: i64,
}
