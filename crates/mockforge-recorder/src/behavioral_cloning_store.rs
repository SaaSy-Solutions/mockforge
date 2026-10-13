//! Recorder database adapter for intelligence's behavioral cloning handlers.

use crate::{database::RecorderDatabase, models::RecordedRequest};
use async_trait::async_trait;
use mockforge_intelligence::behavioral_cloning::storage::{
    BehavioralCloningStore, CloningRecordedRequest, CloningRecordedResponse,
};
use mockforge_intelligence::behavioral_cloning::{BehavioralSequence, EndpointProbabilityModel};

fn project_request(request: RecordedRequest) -> CloningRecordedRequest {
    CloningRecordedRequest {
        timestamp: request.timestamp,
        method: request.method,
        path: request.path,
        body: request.body,
        duration_ms: request.duration_ms,
        status_code: request.status_code,
    }
}

#[async_trait]
impl BehavioralCloningStore for RecorderDatabase {
    async fn get_exchanges_for_endpoint(
        &self,
        endpoint: &str,
        method: &str,
        limit: Option<i32>,
    ) -> Result<Vec<(CloningRecordedRequest, Option<CloningRecordedResponse>)>, String> {
        RecorderDatabase::get_exchanges_for_endpoint(self, endpoint, method, limit)
            .await
            .map(|rows| {
                rows.into_iter()
                    .map(|(request, response)| {
                        (
                            project_request(request),
                            response.map(|r| CloningRecordedResponse {
                                status_code: r.status_code,
                                body: r.body,
                            }),
                        )
                    })
                    .collect()
            })
            .map_err(|error| error.to_string())
    }

    async fn get_requests_by_trace(
        &self,
        min: Option<i32>,
    ) -> Result<Vec<(String, Vec<CloningRecordedRequest>)>, String> {
        RecorderDatabase::get_requests_by_trace(self, min)
            .await
            .map(|groups| {
                groups
                    .into_iter()
                    .map(|(trace, requests)| {
                        (trace, requests.into_iter().map(project_request).collect())
                    })
                    .collect()
            })
            .map_err(|error| error.to_string())
    }

    async fn insert_endpoint_probability_model(
        &self,
        model: &EndpointProbabilityModel,
    ) -> Result<(), String> {
        RecorderDatabase::insert_endpoint_probability_model(self, model)
            .await
            .map_err(|error| error.to_string())
    }

    async fn get_endpoint_probability_model(
        &self,
        endpoint: &str,
        method: &str,
    ) -> Result<Option<EndpointProbabilityModel>, String> {
        RecorderDatabase::get_endpoint_probability_model(self, endpoint, method)
            .await
            .map_err(|error| error.to_string())
    }

    async fn get_all_endpoint_probability_models(
        &self,
    ) -> Result<Vec<EndpointProbabilityModel>, String> {
        RecorderDatabase::get_all_endpoint_probability_models(self)
            .await
            .map_err(|error| error.to_string())
    }

    async fn insert_behavioral_sequence(
        &self,
        sequence: &BehavioralSequence,
    ) -> Result<(), String> {
        RecorderDatabase::insert_behavioral_sequence(self, sequence)
            .await
            .map_err(|error| error.to_string())
    }

    async fn get_behavioral_sequences(&self) -> Result<Vec<BehavioralSequence>, String> {
        RecorderDatabase::get_behavioral_sequences(self)
            .await
            .map_err(|error| error.to_string())
    }
}
