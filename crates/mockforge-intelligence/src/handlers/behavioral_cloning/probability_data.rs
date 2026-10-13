//! Prepare model inputs from recorded traffic, preserving storage precedence.

use crate::behavioral_cloning::storage::{CloningRecordedRequest, CloningRecordedResponse};
use serde_json::{json, Value};

/// Data used by the probability builder. Payloads include exchanges without status;
/// latency and error observations require a known status, as in the recorder API.
pub(super) struct ModelData {
    pub status_codes: Vec<u16>,
    pub latencies_ms: Vec<u64>,
    pub error_responses: Vec<(u16, Value)>,
    pub request_payloads: Vec<Value>,
    pub response_payloads: Vec<Value>,
}

impl ModelData {
    pub(super) fn from_exchanges(
        exchanges: &[(CloningRecordedRequest, Option<CloningRecordedResponse>)],
    ) -> Self {
        // A response status takes precedence over the request's fallback status.
        let outcomes: Vec<_> = exchanges
            .iter()
            .filter_map(|(request, response)| {
                response
                    .as_ref()
                    .map(|r| r.status_code)
                    .or(request.status_code)
                    .map(|status| (request, response, status as u16))
            })
            .collect();
        Self {
            status_codes: outcomes.iter().map(|(_, _, status)| *status).collect(),
            latencies_ms: outcomes
                .iter()
                .filter_map(|(request, _, _)| request.duration_ms.map(|ms| ms as u64))
                .collect(),
            error_responses: outcomes
                .iter()
                .filter(|(_, _, status)| *status >= 400)
                .filter_map(|(_, response, status)| {
                    response.as_ref().and_then(|r| r.body.as_ref()).map(|body| {
                        let value =
                            serde_json::from_str(body).unwrap_or_else(|_| json!({ "error": body }));
                        (*status, value)
                    })
                })
                .collect(),
            request_payloads: exchanges
                .iter()
                .filter_map(|(request, _)| request.body.as_deref())
                .filter_map(|body| serde_json::from_str(body).ok())
                .collect(),
            response_payloads: exchanges
                .iter()
                .filter_map(|(_, response)| response.as_ref().and_then(|r| r.body.as_deref()))
                .filter_map(|body| serde_json::from_str(body).ok())
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separates_status_observations_from_payloads_and_preserves_error_bodies() {
        let request = |status_code, duration_ms, body: &str| CloningRecordedRequest {
            timestamp: chrono::Utc::now(),
            method: "POST".into(),
            path: "/orders".into(),
            body: Some(body.into()),
            duration_ms,
            status_code,
        };
        let exchanges = vec![
            (
                request(Some(418), Some(12), "{}"),
                Some(CloningRecordedResponse {
                    status_code: 200,
                    body: Some("{}".into()),
                }),
            ),
            (request(None, Some(999), "{\"unknown_status\":true}"), None),
            (request(Some(404), Some(34), "not json"), None),
            (
                request(None, None, "[]"),
                Some(CloningRecordedResponse {
                    status_code: 503,
                    body: Some("upstream unavailable".into()),
                }),
            ),
            (
                request(None, Some(56), "{}"),
                Some(CloningRecordedResponse {
                    status_code: 422,
                    body: Some("{\"reason\":\"invalid\"}".into()),
                }),
            ),
        ];
        let data = ModelData::from_exchanges(&exchanges);
        assert_eq!(data.status_codes, vec![200, 404, 503, 422]);
        assert_eq!(data.latencies_ms, vec![12, 34, 56]);
        assert_eq!(
            data.error_responses,
            vec![
                (503, json!({"error": "upstream unavailable"})),
                (422, json!({"reason": "invalid"})),
            ]
        );
        assert_eq!(
            data.request_payloads,
            vec![
                json!({}),
                json!({"unknown_status":true}),
                json!([]),
                json!({})
            ]
        );
        assert_eq!(data.response_payloads, vec![json!({}), json!({"reason":"invalid"})]);
    }
}
