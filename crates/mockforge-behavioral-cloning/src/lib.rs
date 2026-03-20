//! Behavioral Cloning of Backends
//!
//! This crate provides functionality to learn from recorded traffic and create
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
//! ```rust,ignore
//! use mockforge_behavioral_cloning::{
//!     SequenceLearner, ProbabilisticModel, EdgeAmplifier,
//! };
//!
//! async fn example(database: &impl mockforge_behavioral_cloning::TraceQueryProvider) -> mockforge_behavioral_cloning::Result<()> {
//!     // Learn sequences from recorded traffic
//!     let sequences = SequenceLearner::discover_sequences_from_traces(database, 0.1, None).await?;
//!
//!     // Build probability models for endpoints
//!     let model = ProbabilisticModel::build_probability_model_from_data(
//!         "/api/users",
//!         "GET",
//!         &[200, 200, 404],
//!         &[100, 150, 200],
//!         &[],
//!         &[],
//!         &[],
//!     );
//!
//!     // Sample a status code based on learned distribution
//!     let status_code = ProbabilisticModel::sample_status_code(&model);
//!
//!     // Amplify rare errors for testing
//!     let config = mockforge_behavioral_cloning::EdgeAmplificationConfig {
//!         enabled: true,
//!         amplification_factor: 0.5,
//!         ..Default::default()
//!     };
//!     EdgeAmplifier::apply_amplification(&mut model.clone(), &config)?;
//!     Ok(())
//! }
//! ```

pub mod edge_amplifier;
pub mod error;
pub mod probabilistic_model;
pub mod sequence_learner;
pub mod types;

pub use edge_amplifier::EdgeAmplifier;
pub use error::{Error, Result};
pub use probabilistic_model::ProbabilisticModel;
pub use sequence_learner::{SequenceLearner, TraceQueryProvider, TraceRequest};
pub use types::{
    AmplificationScope, BehavioralSequence, EdgeAmplificationConfig, EndpointProbabilityModel,
    ErrorPattern, LatencyDistribution, PayloadVariation, SequenceStep,
};
