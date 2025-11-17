//! Scenario Studio
//!
//! This module provides the core functionality for the Scenario Studio visual editor,
//! enabling collaborative editing of business flows (happy path, SLA violation, regression).

pub mod types;
pub mod flow;

pub use types::{
    FlowCondition, FlowConnection, FlowDefinition, FlowPosition, FlowType, FlowVariant,
    FlowStep, StepType, ConditionOperator,
};
pub use flow::{FlowExecutor, FlowExecutionResult, FlowStepResult};

