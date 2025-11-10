//! Reality Continuum - Dynamic blending of mock and real data sources
//!
//! This module provides functionality to gradually transition from mock to real backend
//! data by intelligently blending responses from both sources. This enables teams to
//! develop and test against a real backend that's still under construction.

pub mod blender;
pub mod config;
pub mod engine;
pub mod merge_strategy;
pub mod schedule;

pub use blender::ResponseBlender;
pub use config::{ContinuumConfig, ContinuumRule, MergeStrategy, TransitionMode};
pub use engine::RealityContinuumEngine;
pub use schedule::{TimeSchedule, TransitionCurve};
