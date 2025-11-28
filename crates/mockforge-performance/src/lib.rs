//! MockForge Performance Mode
//!
//! Lightweight load simulation mode for running scenarios at N RPS,
//! simulating bottlenecks, recording latencies, and observing response
//! changes under load.
//!
//! This is NOT true load testing - it's realistic behavior simulation
//! under stress testing conditions.

pub mod bottleneck;
pub mod controller;
pub mod latency;
pub mod metrics;
pub mod simulator;

pub use bottleneck::{BottleneckConfig, BottleneckSimulator, BottleneckType};
pub use controller::{RpsController, RpsProfile};
pub use latency::{LatencyAnalyzer, LatencyRecorder, LatencySample};
pub use metrics::{PerformanceMetrics, PerformanceSnapshot};
pub use simulator::{PerformanceSimulator, SimulatorConfig};
