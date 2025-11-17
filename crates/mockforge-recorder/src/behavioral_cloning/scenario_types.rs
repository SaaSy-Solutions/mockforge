//! Behavioral scenario types for flow recording and replay
//!
//! This module defines types for named behavioral scenarios that can be
//! compiled from recorded flows and replayed deterministically.

use crate::models::{RecordedRequest, RecordedResponse};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A behavioral scenario that can be replayed deterministically
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehavioralScenario {
    /// Unique identifier for this scenario
    pub id: String,
    /// Human-readable name (e.g., "checkout_success")
    pub name: String,
    /// Optional description
    pub description: Option<String>,
    /// Ordered list of steps in this scenario
    pub steps: Vec<BehavioralScenarioStep>,
    /// State variables extracted from responses (user_id, cart_id, etc.)
    pub state_variables: HashMap<String, StateVariable>,
    /// Whether to use strict mode (exact sequence) or flex mode (minor variations allowed)
    pub strict_mode: bool,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Tags for categorization
    pub tags: Vec<String>,
}

/// A single step in a behavioral scenario
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehavioralScenarioStep {
    /// Step identifier (unique within scenario)
    pub step_id: String,
    /// Optional step label (e.g., "login", "checkout")
    pub label: Option<String>,
    /// The recorded request for this step
    pub request: RecordedRequest,
    /// The recorded response for this step
    pub response: RecordedResponse,
    /// Timing delay from previous step in milliseconds
    pub timing_ms: Option<u64>,
    /// Variables to extract from response (variable_name -> json_path)
    pub extracts: HashMap<String, String>,
    /// Step IDs that this step depends on
    pub depends_on: Vec<String>,
}

/// A state variable extracted from a scenario step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateVariable {
    /// Variable name (e.g., "user_id", "cart_id")
    pub name: String,
    /// JSONPath expression to extract the value
    pub json_path: String,
    /// The step ID where this variable is extracted
    pub extracted_from_step: String,
    /// Optional default value if extraction fails
    pub default_value: Option<serde_json::Value>,
}

impl BehavioralScenario {
    /// Create a new behavioral scenario
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: None,
            steps: Vec::new(),
            state_variables: HashMap::new(),
            strict_mode: true,
            metadata: HashMap::new(),
            tags: Vec::new(),
        }
    }

    /// Add a step to the scenario
    pub fn add_step(mut self, step: BehavioralScenarioStep) -> Self {
        self.steps.push(step);
        self
    }

    /// Add a state variable
    pub fn add_state_variable(mut self, variable: StateVariable) -> Self {
        self.state_variables.insert(variable.name.clone(), variable);
        self
    }

    /// Set strict mode
    pub fn with_strict_mode(mut self, strict: bool) -> Self {
        self.strict_mode = strict;
        self
    }

    /// Set description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add tags
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }
}

impl BehavioralScenarioStep {
    /// Create a new scenario step
    pub fn new(
        step_id: impl Into<String>,
        request: RecordedRequest,
        response: RecordedResponse,
    ) -> Self {
        Self {
            step_id: step_id.into(),
            label: None,
            request,
            response,
            timing_ms: None,
            extracts: HashMap::new(),
            depends_on: Vec::new(),
        }
    }

    /// Set step label
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set timing delay
    pub fn with_timing(mut self, timing_ms: u64) -> Self {
        self.timing_ms = Some(timing_ms);
        self
    }

    /// Add a variable extraction
    pub fn add_extract(mut self, variable_name: impl Into<String>, json_path: impl Into<String>) -> Self {
        self.extracts.insert(variable_name.into(), json_path.into());
        self
    }

    /// Add a dependency on another step
    pub fn add_dependency(mut self, step_id: impl Into<String>) -> Self {
        self.depends_on.push(step_id.into());
        self
    }
}

impl StateVariable {
    /// Create a new state variable
    pub fn new(
        name: impl Into<String>,
        json_path: impl Into<String>,
        extracted_from_step: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            json_path: json_path.into(),
            extracted_from_step: extracted_from_step.into(),
            default_value: None,
        }
    }

    /// Set default value
    pub fn with_default(mut self, default: serde_json::Value) -> Self {
        self.default_value = Some(default);
        self
    }
}

