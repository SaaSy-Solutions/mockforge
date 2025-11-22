//! Behavioral Economics Engine
//!
//! This module provides a behavioral economics engine that makes mocks react to
//! pressure, load, pricing, fraud suspicion, and customer segments. Rules can be
//! either declarative (simple YAML/JSON config) or scriptable (JavaScript/WASM)
//! for complex logic.
//!
//! # Features
//!
//! - **Declarative Rules**: Simple if-then rules for 80% of use cases
//! - **Scriptable Rules**: Advanced logic for complex scenarios
//! - **Condition Evaluation**: Latency, load, pricing, fraud, customer segments
//! - **Action Execution**: Modify responses, change behavior, trigger chaos
//!
//! # Example Usage
//!
//! ```rust,no_run
//! use mockforge_core::behavioral_economics::{
//!     BehavioralEconomicsEngine, BehaviorRule, BehaviorCondition, BehaviorAction
//! };
//!
//! // Declarative rule: Cart conversion drops if latency > 400ms
//! let rule = BehaviorRule {
//!     name: "latency-conversion-impact".to_string(),
//!     rule_type: RuleType::Declarative,
//!     condition: BehaviorCondition::LatencyThreshold {
//!         endpoint: "/api/checkout/*".to_string(),
//!         threshold_ms: 400,
//!     },
//!     action: BehaviorAction::ModifyConversionRate {
//!         multiplier: 0.8,
//!     },
//!     priority: 100,
//! };
//!
//! let engine = BehavioralEconomicsEngine::new(vec![rule]);
//! ```

pub mod actions;
pub mod conditions;
pub mod config;
pub mod engine;
pub mod rules;

pub use actions::{BehaviorAction, ActionExecutor};
pub use conditions::{BehaviorCondition, ConditionEvaluator};
pub use config::BehavioralEconomicsConfig;
pub use engine::BehavioralEconomicsEngine;
pub use rules::{BehaviorRule, RuleType};
