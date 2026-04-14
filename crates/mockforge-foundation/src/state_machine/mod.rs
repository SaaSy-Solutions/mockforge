//! State machine types for scenario/behavior modeling
//!
//! These types were extracted from `mockforge-core::intelligent_behavior`
//! as part of the foundation crate split.

pub mod condition_evaluator;
pub mod history;
pub mod rules;
pub mod sub_scenario;
pub mod visual_layout;

pub use condition_evaluator::{ConditionError, ConditionEvaluator, ConditionResult};
pub use history::HistoryManager;
pub use rules::{ConsistencyRule, EvaluationContext, RuleAction, StateMachine, StateTransition};
pub use sub_scenario::SubScenario;
pub use visual_layout::{Viewport, VisualEdge, VisualLayout, VisualNode};
