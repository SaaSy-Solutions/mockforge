//! Behavioral Cloning of Backends
//!
//! This module provides functionality to learn from recorded traffic and create
//! realistic mock behavior that captures the "personality" of the real backend.
//!
//! # Features
//!
//! - **Sequence Learning**: Discover and model multi-step flows from real traffic
//! - **Probabilistic Outcomes**: Model probabilities of errors, latency, and edge cases per endpoint
//! - **Rare Edge Amplification**: Option to increase rare error frequency for testing
//!
//! # Example Usage
//!
//! ```rust,no_run
//! use mockforge_core::behavioral_cloning::{
//!     SequenceLearner, ProbabilisticModel, EdgeAmplifier,
//! };
//!
//! # async fn example() -> mockforge_core::Result<()> {
//! // Learn sequences from recorded traffic
//! let sequences = SequenceLearner::discover_sequences_from_traces(&database).await?;
//!
//! // Build probability models for endpoints
//! let model = ProbabilisticModel::build_probability_model(
//!     &database,
//!     "/api/users",
//!     "GET"
//! ).await?;
//!
//! // Sample a status code based on learned distribution
//! let status_code = model.sample_status_code();
//!
//! // Amplify rare errors for testing
//! let amplifier = EdgeAmplifier::new();
//! amplifier.apply_amplification(&mut model, 0.5)?; // 50% frequency
//! # Ok(())
//! # }
//! ```

pub mod edge_amplifier;
pub mod probabilistic_model;
pub mod sequence_learner;
pub mod types;

pub use edge_amplifier::EdgeAmplifier;
pub use probabilistic_model::ProbabilisticModel;
pub use sequence_learner::SequenceLearner;
pub use types::{
    AmplificationScope, BehavioralSequence, EdgeAmplificationConfig, EndpointProbabilityModel,
    ErrorPattern, LatencyDistribution, PayloadVariation, SequenceStep,
};

