//! Behavioral cloning modules for flow recording and scenario management
//!
//! This module provides functionality to record multi-step flows and compile
//! them into behavioral scenarios that can be replayed.

pub mod flow_compiler;
pub mod flow_recorder;
pub mod replay_engine;
pub mod scenario_types;
pub mod storage;

pub use flow_compiler::FlowCompiler;
pub use flow_recorder::{Flow, FlowRecorder, FlowRecordingConfig, FlowStep, FlowGroupingStrategy};
pub use replay_engine::{BehavioralScenarioReplayEngine, ReplayResponse};
pub use scenario_types::{BehavioralScenario, BehavioralScenarioStep, StateVariable};
pub use storage::{ScenarioInfo, ScenarioStorage};

