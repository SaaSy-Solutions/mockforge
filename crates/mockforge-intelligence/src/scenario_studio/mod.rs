//! Scenario Studio
//!
//! This module provides the core functionality for the Scenario Studio visual editor,
//! enabling collaborative editing of business flows (happy path, SLA violation, regression).

pub mod flow;
pub mod types;

pub use flow::{FlowExecutionResult, FlowExecutor, FlowStepResult};
pub use types::{
    ConditionOperator, FlowCondition, FlowConnection, FlowDefinition, FlowPosition, FlowStep,
    FlowType, FlowVariant, StepType,
};
