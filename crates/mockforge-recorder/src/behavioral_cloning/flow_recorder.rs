//! Flow recorder for capturing sequences of requests/responses
//!
//! This module provides functionality to record multi-step flows by grouping
//! requests by trace_id, session_id, or client IP + time window.

use crate::database::{FlowMetadataRow, FlowStepRow, RecorderDatabase};
use crate::models::RecordedRequest;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Configuration for flow recording
#[derive(Debug, Clone)]
pub struct FlowRecordingConfig {
    /// How to group requests into flows
    pub group_by: FlowGroupingStrategy,
    /// Time window in seconds for IP-based grouping
    pub time_window_seconds: u64,
    /// Whether flow recording is enabled
    pub enabled: bool,
}

/// Strategy for grouping requests into flows
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowGroupingStrategy {
    /// Group by trace_id (preferred, if available)
    TraceId,
    /// Group by session_id (from cookies/headers)
    SessionId,
    /// Group by client IP + time window (fallback)
    IpTimeWindow,
}

impl FlowGroupingStrategy {
    /// Parse from string
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "trace_id" => Self::TraceId,
            "session_id" => Self::SessionId,
            "ip_time_window" => Self::IpTimeWindow,
            _ => {
                warn!("Unknown grouping strategy: {}, defaulting to trace_id", s);
                Self::TraceId
            }
        }
    }
}

impl Default for FlowRecordingConfig {
    fn default() -> Self {
        Self {
            group_by: FlowGroupingStrategy::TraceId,
            time_window_seconds: 300, // 5 minutes
            enabled: true,
        }
    }
}

/// A recorded flow representing a sequence of requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Flow {
    /// Unique identifier for this flow
    pub id: String,
    /// Optional name for this flow
    pub name: Option<String>,
    /// Optional description
    pub description: Option<String>,
    /// Ordered list of steps in this flow
    pub steps: Vec<FlowStep>,
    /// When this flow was created
    pub created_at: DateTime<Utc>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// A single step in a flow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowStep {
    /// Request ID (references RecordedRequest)
    pub request_id: String,
    /// Step index in the flow (0-based)
    pub step_index: usize,
    /// Optional step label (e.g., "login", "checkout")
    pub step_label: Option<String>,
    /// Timing delay from previous step in milliseconds
    pub timing_ms: Option<u64>,
}

/// Flow recorder for capturing and managing flows
pub struct FlowRecorder {
    /// Database connection
    db: RecorderDatabase,
    /// Recording configuration
    config: FlowRecordingConfig,
    /// Active flows being tracked (group_key -> flow_id)
    active_flows: HashMap<String, String>,
    /// Last request timestamp per flow (flow_id -> timestamp)
    last_request_timestamps: HashMap<String, DateTime<Utc>>,
}

impl FlowRecorder {
    /// Create a new flow recorder
    pub fn new(db: RecorderDatabase, config: FlowRecordingConfig) -> Self {
        Self {
            db,
            config,
            active_flows: HashMap::new(),
            last_request_timestamps: HashMap::new(),
        }
    }

    /// Record a request as part of a flow
    ///
    /// This method determines which flow the request belongs to based on the
    /// grouping strategy, creates a new flow if needed, and adds the request
    /// as a step in that flow.
    pub async fn record_request(&mut self, request: &RecordedRequest) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        // Determine group key based on strategy
        let group_key = self.get_group_key(request)?;
        if group_key.is_empty() {
            // Cannot group this request, skip
            debug!("Cannot group request {}: no grouping key available", request.id);
            return Ok(());
        }

        // Get or create flow for this group
        let flow_id = if let Some(flow_id) = self.active_flows.get(&group_key) {
            flow_id.clone()
        } else {
            // Create new flow
            let flow_id = Uuid::new_v4().to_string();
            self.db
                .create_flow(&flow_id, None, None, &[])
                .await
                .map_err(|e| anyhow::anyhow!("Failed to create flow: {}", e))?;
            self.active_flows.insert(group_key.clone(), flow_id.clone());
            flow_id
        };

        // Calculate timing from previous request in this flow
        let timing_ms = if let Some(last_timestamp) = self.last_request_timestamps.get(&flow_id) {
            let delta = request.timestamp.signed_duration_since(*last_timestamp);
            Some(delta.num_milliseconds().max(0) as u64)
        } else {
            None // First step in flow
        };

        // Add step to flow
        let step_index = self.db
            .get_flow_step_count(&flow_id)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get flow step count: {}", e))?;
        self.db
            .add_flow_step(&flow_id, &request.id, step_index, None, timing_ms)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to add step to flow: {}", e))?;

        // Update last request timestamp
        self.last_request_timestamps
            .insert(flow_id, request.timestamp);

        Ok(())
    }

    /// Get the group key for a request based on the grouping strategy
    fn get_group_key(&self, request: &RecordedRequest) -> Result<String> {
        match self.config.group_by {
            FlowGroupingStrategy::TraceId => {
                Ok(request.trace_id.clone().unwrap_or_default())
            }
            FlowGroupingStrategy::SessionId => {
                // Try to extract session_id from headers or cookies
                let headers: HashMap<String, String> =
                    serde_json::from_str(&request.headers).unwrap_or_default();

                // Check for common session ID headers
                if let Some(session_id) = headers.get("x-session-id")
                    .or_else(|| headers.get("session-id"))
                    .or_else(|| headers.get("authorization")) // Could use auth token as session
                {
                    Ok(session_id.clone())
                } else {
                    // Try to extract from Cookie header
                    if let Some(cookie) = headers.get("cookie") {
                        // Simple extraction - look for session_id cookie
                        for part in cookie.split(';') {
                            let part = part.trim();
                            if part.starts_with("session_id=") {
                                return Ok(part[11..].to_string());
                            }
                            if part.starts_with("session=") {
                                return Ok(part[8..].to_string());
                            }
                        }
                    }
                    Ok(String::new())
                }
            }
            FlowGroupingStrategy::IpTimeWindow => {
                // Group by IP + time window
                if let Some(ip) = &request.client_ip {
                    // Round timestamp to time window
                    let window_seconds = self.config.time_window_seconds as i64;
                    let timestamp_seconds = request.timestamp.timestamp();
                    let window_start = (timestamp_seconds / window_seconds) * window_seconds;
                    Ok(format!("{}:{}", ip, window_start))
                } else {
                    Ok(String::new())
                }
            }
        }
    }


    /// Get a flow by ID
    pub async fn get_flow(&self, flow_id: &str) -> Result<Option<Flow>> {
        // Get flow metadata
        let flow_meta = self
            .db
            .get_flow_metadata(flow_id)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get flow: {}", e))?;

        if let Some(meta) = flow_meta {
            // Get flow steps
            let step_rows = self
                .db
                .get_flow_steps(flow_id)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to get flow steps: {}", e))?;

            let mut steps = Vec::new();
            for step_row in step_rows {
                steps.push(FlowStep {
                    request_id: step_row.request_id,
                    step_index: step_row.step_index as usize,
                    step_label: step_row.step_label,
                    timing_ms: step_row.timing_ms.map(|t| t as u64),
                });
            }

            Ok(Some(Flow {
                id: meta.id,
                name: meta.name,
                description: meta.description,
                steps,
                created_at: DateTime::parse_from_rfc3339(&meta.created_at)
                    .map_err(|e| anyhow::anyhow!("Invalid timestamp: {}", e))?
                    .with_timezone(&Utc),
                tags: serde_json::from_str(&meta.tags).unwrap_or_default(),
                metadata: serde_json::from_str(&meta.metadata).unwrap_or_default(),
            }))
        } else {
            Ok(None)
        }
    }

    /// List all flows
    pub async fn list_flows(&self, limit: Option<usize>) -> Result<Vec<Flow>> {
        let limit = limit.map(|l| l as i64);
        let flow_rows = self
            .db
            .list_flows(limit)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list flows: {}", e))?;

        let mut flows = Vec::new();
        for meta in flow_rows {
            if let Some(flow) = self.get_flow(&meta.id).await? {
                flows.push(flow);
            }
        }

        Ok(flows)
    }

    /// Update flow metadata (name, description, tags)
    pub async fn update_flow_metadata(
        &self,
        flow_id: &str,
        name: Option<&str>,
        description: Option<&str>,
        tags: Option<Vec<String>>,
    ) -> Result<()> {
        let tags_ref = tags.as_deref();
        self.db
            .update_flow_metadata(flow_id, name, description, tags_ref)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to update flow: {}", e))?;

        info!("Updated flow metadata: {}", flow_id);
        Ok(())
    }

    /// Delete a flow
    pub async fn delete_flow(&self, flow_id: &str) -> Result<()> {
        self.db
            .delete_flow(flow_id)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to delete flow: {}", e))?;

        info!("Deleted flow: {}", flow_id);
        Ok(())
    }
}

