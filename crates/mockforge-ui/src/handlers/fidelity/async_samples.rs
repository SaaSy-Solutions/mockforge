//! Adapt recorder exports to the shared AsyncAPI validator without filesystem access.
use mockforge_contracts::fidelity::async_sample::{self, AsyncMessage, AsyncSampleReport};
use mockforge_recorder::models::RecordedExchange;
use serde_json::Value;

pub(super) fn verify(
    source: &Value,
    records: Vec<RecordedExchange>,
) -> Result<AsyncSampleReport, String> {
    if records.len() > 500 {
        return Err("A sample must contain at most 500 exchanges".into());
    }
    let records = mockforge_recorder::async_messages::decode(&records)?;
    let messages: Vec<_> = records
        .into_iter()
        .map(|r| AsyncMessage {
            protocol: r.protocol.as_str().into(),
            operation: r.operation,
            address: r.address,
            headers: r.headers,
            payload: r.payload,
            timestamp: r.timestamp,
        })
        .collect();
    async_sample::verify(source, &messages, 16 * 1024 * 1024)
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use serde_json::json;

    fn spec() -> Value {
        json!({"asyncapi":"2.6.0","info":{"title":"owned-events","version":"1"},"defaultContentType":"application/json","servers":{"local":{"url":"localhost:1883","protocol":"mqtt"}},"channels":{"orders":{"publish":{"message":{"payload":{"type":"object","required":["id"],"properties":{"id":{"type":"integer"}}}}}}}})
    }
    fn record(body: &str) -> RecordedExchange {
        serde_json::from_value(json!({"request":{"id":"owned","protocol":"Mqtt","method":"publish","path":"orders","headers":"{}","body":body,"body_encoding":"utf8","timestamp":"2026-10-20T00:00:00Z"}})).unwrap()
    }
    fn payload(source: Value, records: Option<Vec<RecordedExchange>>) -> CompareRequest {
        CompareRequest {
            mock_spec: source.to_string(),
            source_spec: source.to_string(),
            sample: records,
        }
    }

    #[test]
    fn one_way_messages_check_payloads_without_requiring_a_response() {
        for (body, status) in [
            (r#"{"id":1}"#, "verified"),
            (r#"{"id":"1"}"#, "drift-detected"),
            ("malformed", "drift-detected"),
        ] {
            let report = assess(payload(spec(), Some(vec![record(body)]))).unwrap();
            let sample = report.sample.unwrap();
            assert_eq!(sample["exchanges_checked"], 1);
            assert!(sample["routes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r["endpoint"] == "RECEIVE orders" && r["status"] == status));
        }
    }
    #[test]
    fn definition_drift_and_empty_or_malformed_captures_never_pass() {
        let mut input = payload(spec(), None);
        let mut changed = spec();
        changed["channels"]["orders"]["publish"]["message"]["payload"]["properties"]["id"]
            ["type"] = json!("string");
        input.source_spec = changed.to_string();
        assert!(assess(input).unwrap().definition.routes_with_drift > 0);
        assert!(assess(payload(spec(), Some(vec![]))).is_err());
        let mut sample = record("bad");
        sample.request.body_encoding = "base64".into();
        assert!(assess(payload(spec(), Some(vec![sample]))).is_err());
    }
    #[test]
    fn missing_header_capture_and_unsupported_media_remain_unverified() {
        let mut source = spec();
        source["channels"]["orders"]["publish"]["message"]["headers"] =
            json!({"type":"object","required":["trace"]});
        let report = assess(payload(source, Some(vec![record(r#"{"id":1}"#)]))).unwrap();
        assert!(report.sample.unwrap()["routes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["status"] == "not-verified"));
        let mut source = spec();
        source["channels"]["orders"]["publish"]["message"]["contentType"] =
            json!("application/octet-stream");
        let report = assess(payload(source, Some(vec![record("bytes")]))).unwrap();
        assert!(report.sample.unwrap()["routes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["status"] == "not-verified"));
    }
}
