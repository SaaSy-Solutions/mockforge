//! Scenario Studio types
//!
//! This module defines the core types for the Scenario Studio visual editor,
//! which enables collaborative editing of business flows (happy path, SLA violation, regression).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

/// Represents a business flow definition (happy path, SLA violation, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowDefinition {
    /// Unique identifier for the flow
    pub id: String,
    /// Flow name
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// Type of flow (happy path, SLA violation, regression, etc.)
    pub flow_type: FlowType,
    /// Steps in the flow
    pub steps: Vec<FlowStep>,
    /// Connections between steps (from_step_id -> to_step_id)
    pub connections: Vec<FlowConnection>,
    /// Variables available in the flow context
    #[serde(default)]
    pub variables: HashMap<String, Value>,
    /// Tags for categorization
    #[serde(default)]
    pub tags: Vec<String>,
    /// Metadata
    #[serde(default)]
    pub metadata: HashMap<String, Value>,
    /// Timestamp when the flow was created
    pub created_at: DateTime<Utc>,
    /// Timestamp when the flow was last updated
    pub updated_at: DateTime<Utc>,
    /// ID of the user who created the flow
    pub created_by: Option<String>,
    /// ID of the user who last updated the flow
    pub updated_by: Option<String>,
}

impl FlowDefinition {
    /// Create a new flow definition
    pub fn new(name: String, flow_type: FlowType) -> Self {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        Self {
            id,
            name,
            description: None,
            flow_type,
            steps: Vec::new(),
            connections: Vec::new(),
            variables: HashMap::new(),
            tags: Vec::new(),
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
            created_by: None,
            updated_by: None,
        }
    }

    /// Add a step to the flow
    pub fn add_step(&mut self, step: FlowStep) {
        self.steps.push(step);
        self.updated_at = Utc::now();
    }

    /// Remove a step from the flow
    pub fn remove_step(&mut self, step_id: &str) {
        self.steps.retain(|s| s.id != step_id);
        // Remove connections involving this step
        self.connections.retain(|c| c.from_step_id != step_id && c.to_step_id != step_id);
        self.updated_at = Utc::now();
    }

    /// Add a connection between steps
    pub fn add_connection(&mut self, connection: FlowConnection) {
        self.connections.push(connection);
        self.updated_at = Utc::now();
    }

    /// Remove a connection between steps
    pub fn remove_connection(&mut self, from_step_id: &str, to_step_id: &str) {
        self.connections.retain(|c| {
            !(c.from_step_id == from_step_id && c.to_step_id == to_step_id)
        });
        self.updated_at = Utc::now();
    }
}

/// Type of business flow
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FlowType {
    /// Happy path - normal successful execution
    HappyPath,
    /// SLA violation - simulates service level agreement violations
    SLAViolation,
    /// Regression - tests for regressions in API behavior
    Regression,
    /// Custom - user-defined flow type
    Custom,
}

impl FlowType {
    /// Get a human-readable name for the flow type
    pub fn display_name(&self) -> &'static str {
        match self {
            FlowType::HappyPath => "Happy Path",
            FlowType::SLAViolation => "SLA Violation",
            FlowType::Regression => "Regression",
            FlowType::Custom => "Custom",
        }
    }
}

/// Individual step in a flow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowStep {
    /// Unique identifier for the step
    pub id: String,
    /// Step name
    pub name: String,
    /// Step type (API call, condition, delay, etc.)
    pub step_type: StepType,
    /// HTTP method (if step_type is ApiCall)
    pub method: Option<String>,
    /// Endpoint URL (if step_type is ApiCall)
    pub endpoint: Option<String>,
    /// Request headers
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Request body
    pub body: Option<Value>,
    /// Conditions for executing this step
    pub condition: Option<FlowCondition>,
    /// Expected response status code
    pub expected_status: Option<u16>,
    /// Variables to extract from the response
    #[serde(default)]
    pub extract: HashMap<String, String>,
    /// Delay before executing this step (in milliseconds)
    pub delay_ms: Option<u64>,
    /// Position in the visual editor (x, y coordinates)
    pub position: Option<FlowPosition>,
    /// Metadata
    #[serde(default)]
    pub metadata: HashMap<String, Value>,
}

impl FlowStep {
    /// Create a new API call step
    pub fn new_api_call(
        name: String,
        method: String,
        endpoint: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            step_type: StepType::ApiCall,
            method: Some(method),
            endpoint: Some(endpoint),
            headers: HashMap::new(),
            body: None,
            condition: None,
            expected_status: None,
            extract: HashMap::new(),
            delay_ms: None,
            position: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a new condition step
    pub fn new_condition(name: String, condition: FlowCondition) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            step_type: StepType::Condition,
            method: None,
            endpoint: None,
            headers: HashMap::new(),
            body: None,
            condition: Some(condition),
            expected_status: None,
            extract: HashMap::new(),
            delay_ms: None,
            position: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a new delay step
    pub fn new_delay(name: String, delay_ms: u64) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            step_type: StepType::Delay,
            method: None,
            endpoint: None,
            headers: HashMap::new(),
            body: None,
            condition: None,
            expected_status: None,
            extract: HashMap::new(),
            delay_ms: Some(delay_ms),
            position: None,
            metadata: HashMap::new(),
        }
    }
}

/// Type of step in a flow
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StepType {
    /// API call step
    ApiCall,
    /// Conditional branching step
    Condition,
    /// Delay step
    Delay,
    /// Loop step
    Loop,
    /// Parallel execution step
    Parallel,
}

impl StepType {
    /// Get a human-readable name for the step type
    pub fn display_name(&self) -> &'static str {
        match self {
            StepType::ApiCall => "API Call",
            StepType::Condition => "Condition",
            StepType::Delay => "Delay",
            StepType::Loop => "Loop",
            StepType::Parallel => "Parallel",
        }
    }
}

/// Condition for executing a step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowCondition {
    /// Condition expression (e.g., "{{response.status}} == 200")
    pub expression: String,
    /// Operator (eq, ne, gt, lt, contains, etc.)
    pub operator: ConditionOperator,
    /// Value to compare against
    pub value: Value,
}

/// Condition operator
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConditionOperator {
    /// Equals
    Eq,
    /// Not equals
    Ne,
    /// Greater than
    Gt,
    /// Greater than or equal
    Gte,
    /// Less than
    Lt,
    /// Less than or equal
    Lte,
    /// Contains
    Contains,
    /// Not contains
    NotContains,
    /// Matches regex
    Matches,
    /// Exists
    Exists,
}

/// Connection between two steps in a flow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowConnection {
    /// ID of the source step
    pub from_step_id: String,
    /// ID of the target step
    pub to_step_id: String,
    /// Label for the connection (e.g., "success", "error")
    pub label: Option<String>,
    /// Condition for taking this connection
    pub condition: Option<FlowCondition>,
}

impl FlowConnection {
    /// Create a new connection
    pub fn new(from_step_id: String, to_step_id: String) -> Self {
        Self {
            from_step_id,
            to_step_id,
            label: None,
            condition: None,
        }
    }
}

/// Position in the visual editor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowPosition {
    /// X coordinate
    pub x: f64,
    /// Y coordinate
    pub y: f64,
}

impl FlowPosition {
    /// Create a new position
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// Different versions of the same flow (e.g., happy path vs error path)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowVariant {
    /// Unique identifier for the variant
    pub id: String,
    /// Variant name
    pub name: String,
    /// Description of what this variant represents
    pub description: Option<String>,
    /// ID of the base flow
    pub flow_id: String,
    /// Modified steps (step_id -> modified step)
    #[serde(default)]
    pub modified_steps: HashMap<String, FlowStep>,
    /// Additional connections
    #[serde(default)]
    pub additional_connections: Vec<FlowConnection>,
    /// Removed step IDs
    #[serde(default)]
    pub removed_step_ids: Vec<String>,
    /// Timestamp when the variant was created
    pub created_at: DateTime<Utc>,
    /// Timestamp when the variant was last updated
    pub updated_at: DateTime<Utc>,
}

impl FlowVariant {
    /// Create a new flow variant
    pub fn new(name: String, flow_id: String) -> Self {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        Self {
            id,
            name,
            description: None,
            flow_id,
            modified_steps: HashMap::new(),
            additional_connections: Vec::new(),
            removed_step_ids: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }
}

