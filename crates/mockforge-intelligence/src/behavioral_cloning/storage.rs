//! Storage boundary for behavioral cloning, independent of recorder and core.

use super::{BehavioralSequence, EndpointProbabilityModel};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Recorded request fields consumed by the behavioral handlers.
#[derive(Clone, Debug)]
pub struct CloningRecordedRequest {
    /// Request timestamp, preserving database order within each trace.
    pub timestamp: DateTime<Utc>,
    /// HTTP method.
    pub method: String,
    /// Endpoint path.
    pub path: String,
    /// Request body in the recorder's existing representation.
    pub body: Option<String>,
    /// Recorded request duration in milliseconds.
    pub duration_ms: Option<i64>,
    /// Status recorded on the request when a separate response is absent.
    pub status_code: Option<i32>,
}

/// Recorded response fields consumed by probability-model generation.
#[derive(Clone, Debug)]
pub struct CloningRecordedResponse {
    /// HTTP status code.
    pub status_code: i32,
    /// Response body in the recorder's existing representation.
    pub body: Option<String>,
}

/// Recording queries and model persistence required by behavioral cloning.
///
/// Implementations preserve trace ordering, endpoint filtering, limits and
/// existing error messages. The concrete SQLite implementation stays in recorder.
#[async_trait]
pub trait BehavioralCloningStore: Send + Sync {
    /// Read request/response exchanges for an endpoint and method.
    async fn get_exchanges_for_endpoint(
        &self,
        endpoint: &str,
        method: &str,
        limit: Option<i32>,
    ) -> Result<Vec<(CloningRecordedRequest, Option<CloningRecordedResponse>)>, String>;
    /// Read timestamp-ordered requests grouped by trace, applying the minimum.
    async fn get_requests_by_trace(
        &self,
        min: Option<i32>,
    ) -> Result<Vec<(String, Vec<CloningRecordedRequest>)>, String>;
    /// Persist a probability model.
    async fn insert_endpoint_probability_model(
        &self,
        model: &EndpointProbabilityModel,
    ) -> Result<(), String>;
    /// Read one probability model.
    async fn get_endpoint_probability_model(
        &self,
        endpoint: &str,
        method: &str,
    ) -> Result<Option<EndpointProbabilityModel>, String>;
    /// Read all probability models.
    async fn get_all_endpoint_probability_models(
        &self,
    ) -> Result<Vec<EndpointProbabilityModel>, String>;
    /// Persist a learned sequence.
    async fn insert_behavioral_sequence(&self, sequence: &BehavioralSequence)
        -> Result<(), String>;
    /// Read all learned sequences.
    async fn get_behavioral_sequences(&self) -> Result<Vec<BehavioralSequence>, String>;
}

/// Router state that opens its recording store when a handler needs it.
///
/// Keeping this interface generic lets the HTTP compatibility state retain its
/// database-path constructors and public fields without a reverse dependency.
#[async_trait]
pub trait BehavioralCloningStateProvider: Clone + Send + Sync + 'static {
    /// Concrete recording store supplied by this state.
    type Store: BehavioralCloningStore;
    /// Open the state-selected store, returning the existing error text on failure.
    async fn open_store(&self) -> Result<Self::Store, String>;
}
