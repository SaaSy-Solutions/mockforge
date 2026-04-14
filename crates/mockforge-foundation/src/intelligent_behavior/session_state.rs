//! Session types: `InteractionRecord` and `SessionState`
//!
//! Extracted from `mockforge-core::intelligent_behavior::types` (Phase 6 / A7).
//! Uses `crate::clock::now()` which honors any registered time-travel clock.

use crate::clock::now as clock_now;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single interaction record (request + response pair)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionRecord {
    /// Timestamp of the interaction
    pub timestamp: DateTime<Utc>,

    /// HTTP method (GET, POST, PUT, DELETE, etc.)
    pub method: String,

    /// Request path (e.g., /api/users/123)
    pub path: String,

    /// Query parameters
    #[serde(default)]
    pub query_params: HashMap<String, String>,

    /// Request headers
    #[serde(default)]
    pub headers: HashMap<String, String>,

    /// Request body (if present)
    pub request: Option<serde_json::Value>,

    /// Response status code
    pub status: u16,

    /// Response body (if present)
    pub response: Option<serde_json::Value>,

    /// Vector embedding for semantic search (generated from interaction summary)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,

    /// Metadata about this interaction
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl InteractionRecord {
    /// Create a new interaction record (timestamped at the current wall-clock time).
    pub fn new(
        method: impl Into<String>,
        path: impl Into<String>,
        request: Option<serde_json::Value>,
        status: u16,
        response: Option<serde_json::Value>,
    ) -> Self {
        Self {
            timestamp: Utc::now(),
            method: method.into(),
            path: path.into(),
            query_params: HashMap::new(),
            headers: HashMap::new(),
            request,
            status,
            response,
            embedding: None,
            metadata: HashMap::new(),
        }
    }

    /// Add query parameters.
    #[must_use]
    pub fn with_query_params(mut self, params: HashMap<String, String>) -> Self {
        self.query_params = params;
        self
    }

    /// Add headers.
    #[must_use]
    pub fn with_headers(mut self, headers: HashMap<String, String>) -> Self {
        self.headers = headers;
        self
    }

    /// Set embedding.
    #[must_use]
    pub fn with_embedding(mut self, embedding: Vec<f32>) -> Self {
        self.embedding = Some(embedding);
        self
    }

    /// Add metadata.
    #[must_use]
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Generate a textual summary of this interaction for embedding.
    pub fn summary(&self) -> String {
        let request_body = self
            .request
            .as_ref()
            .map(|r| serde_json::to_string(r).unwrap_or_default())
            .unwrap_or_default();

        let response_body = self
            .response
            .as_ref()
            .map(|r| serde_json::to_string(r).unwrap_or_default())
            .unwrap_or_default();

        format!(
            "{} {} | Request: {} | Status: {} | Response: {}",
            self.method, self.path, request_body, self.status, response_body
        )
    }
}

/// Session state snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    /// Session ID
    pub session_id: String,

    /// Current state data (e.g., logged-in user, cart items, etc.)
    pub state: HashMap<String, serde_json::Value>,

    /// Interaction history for this session
    pub history: Vec<InteractionRecord>,

    /// Session creation time
    pub created_at: DateTime<Utc>,

    /// Last activity time
    pub last_activity: DateTime<Utc>,

    /// Session metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl SessionState {
    /// Create a new session state.
    ///
    /// Uses `crate::clock::now()` which respects any registered virtual clock
    /// (see `mockforge_foundation::clock::set_clock`).
    pub fn new(session_id: impl Into<String>) -> Self {
        let now = clock_now();
        Self {
            session_id: session_id.into(),
            state: HashMap::new(),
            history: Vec::new(),
            created_at: now,
            last_activity: now,
            metadata: HashMap::new(),
        }
    }

    /// Update last activity timestamp.
    ///
    /// Uses `crate::clock::now()` which respects any registered virtual clock.
    pub fn touch(&mut self) {
        self.last_activity = clock_now();
    }

    /// Add an interaction to history.
    pub fn record_interaction(&mut self, interaction: InteractionRecord) {
        self.history.push(interaction);
        self.touch();
    }

    /// Get a value from state.
    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.state.get(key)
    }

    /// Set a value in state.
    pub fn set(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.state.insert(key.into(), value);
        self.touch();
    }

    /// Remove a value from state.
    pub fn remove(&mut self, key: &str) -> Option<serde_json::Value> {
        let result = self.state.remove(key);
        self.touch();
        result
    }

    /// Check if session has been inactive for a duration.
    ///
    /// Uses `crate::clock::now()` which respects any registered virtual clock.
    pub fn is_inactive(&self, duration: chrono::Duration) -> bool {
        clock_now().signed_duration_since(self.last_activity) > duration
    }
}
