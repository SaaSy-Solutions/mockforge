//! Behavioral cloning HTTP handlers, backed by an injected recording store.
//!
//! Probability models, learned sequences and edge amplification have separate
//! modules. The recorder implements storage downstream, avoiding a core cycle.

mod amplification;
mod probability;
mod probability_data;
mod sequences;

pub use amplification::{apply_amplification, get_rare_edges, ApplyAmplificationRequest};
pub use probability::{
    build_probability_model, get_probability_model, list_probability_models, sample_latency,
    sample_status_code, BuildProbabilityModelRequest, ProbabilityModelResponse,
};
pub use sequences::{
    discover_sequences, generate_sequence_scenario, get_sequence, list_sequences,
    DiscoverSequencesRequest, SequenceDiscoveryResponse,
};

use crate::behavioral_cloning::storage::BehavioralCloningStateProvider;

/// Create router for behavioral cloning endpoints
pub fn behavioral_cloning_router<S: BehavioralCloningStateProvider>(state: S) -> axum::Router {
    use axum::routing::{get, post};
    use axum::Router;

    Router::new()
        // Probability model endpoints
        .route(
            "/api/v1/behavioral-cloning/probability-models",
            post(build_probability_model::<S>).get(list_probability_models::<S>),
        )
        .route(
            "/api/v1/behavioral-cloning/probability-models/{endpoint}/{method}",
            get(get_probability_model::<S>),
        )
        .route(
            "/api/v1/behavioral-cloning/probability-models/{endpoint}/{method}/sample/status-code",
            post(sample_status_code::<S>),
        )
        .route(
            "/api/v1/behavioral-cloning/probability-models/{endpoint}/{method}/sample/latency",
            post(sample_latency::<S>),
        )
        // Sequence endpoints
        .route(
            "/api/v1/behavioral-cloning/sequences",
            get(list_sequences::<S>),
        )
        .route(
            "/api/v1/behavioral-cloning/sequences/discover",
            post(discover_sequences::<S>),
        )
        .route(
            "/api/v1/behavioral-cloning/sequences/{sequence_id}",
            get(get_sequence::<S>),
        )
        .route(
            "/api/v1/behavioral-cloning/sequences/{sequence_id}/scenario",
            post(generate_sequence_scenario::<S>),
        )
        // Amplification endpoints
        .route(
            "/api/v1/behavioral-cloning/amplification/apply",
            post(apply_amplification::<S>),
        )
        .route(
            "/api/v1/behavioral-cloning/amplification/rare-edges/{endpoint}/{method}",
            get(get_rare_edges::<S>),
        )
        .with_state(state)
}
