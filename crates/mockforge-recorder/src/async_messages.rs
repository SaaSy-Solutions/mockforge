//! Normalize asynchronous recorder events without guessing message provenance.

use crate::models::{Protocol, RecordedExchange};
use base64::Engine;
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::BTreeMap;

/// Decoded broker/WebSocket message data for offline validation consumers.
pub struct DecodedAsyncMessage {
    /// Recorder protocol, retaining its original identity.
    pub protocol: Protocol,
    /// Original wire operation, from the mock server's perspective.
    pub operation: String,
    /// Channel address, or None if the original WebSocket path is unavailable.
    pub address: Option<String>,
    /// Captured message headers, excluding recorder-only metadata.
    pub headers: Option<Value>,
    /// Message bytes decoded from the recorder's encoding.
    pub payload: Option<Vec<u8>>,
    /// Original capture time, never substituted with the current time.
    pub timestamp: DateTime<Utc>,
}

/// Decode actual message events. Control events are used for WebSocket path
/// attribution but are not counted as messages. Invalid encodings fail the whole
/// export instead of silently dropping or replacing captured bytes.
pub fn decode(records: &[RecordedExchange]) -> Result<Vec<DecodedAsyncMessage>, String> {
    let connections: BTreeMap<_, _> = records
        .iter()
        .filter(|r| {
            r.request.protocol == Protocol::WebSocket
                && r.request.method.eq_ignore_ascii_case("CONNECT")
        })
        .map(|r| (r.request.id.as_str(), r.request.path.as_str()))
        .collect();
    let mut out = Vec::new();
    for exchange in records {
        let request = &exchange.request;
        if !is_message(request.protocol, &request.method) {
            continue;
        }
        let mut headers: Value = serde_json::from_str(&request.headers)
            .map_err(|_| "Recorded headers must be a JSON object")?;
        let map = headers.as_object_mut().ok_or("Recorded headers must be a JSON object")?;
        let address = if request.protocol == Protocol::WebSocket {
            if let Some(connection_id) = map.get("ws-connection-id").and_then(Value::as_str) {
                connections.get(connection_id).map(|path| (*path).to_string())
            } else {
                Some(request.path.clone())
            }
        } else {
            Some(request.path.clone())
        };
        map.remove("ws-connection-id");
        let headers = if map.is_empty() { None } else { Some(headers) };
        let payload = request
            .body
            .as_ref()
            .map(|body| match request.body_encoding.as_str() {
                "utf8" | "utf-8" => Ok(body.as_bytes().to_vec()),
                "base64" => base64::engine::general_purpose::STANDARD
                    .decode(body)
                    .map_err(|_| "Invalid base64 recorded message body".to_string()),
                _ => Err("Unsupported recorded message body encoding".into()),
            })
            .transpose()?;
        out.push(DecodedAsyncMessage {
            protocol: request.protocol,
            operation: request.method.clone(),
            address,
            headers,
            payload,
            timestamp: request.timestamp,
        });
    }
    if out.is_empty() {
        return Err("The sample contains no MQTT, Kafka, AMQP or WebSocket message events".into());
    }
    Ok(out)
}

fn is_message(protocol: Protocol, operation: &str) -> bool {
    matches!(
        (protocol, operation.to_ascii_lowercase().as_str()),
        (Protocol::Mqtt | Protocol::Amqp, "publish" | "deliver")
            | (Protocol::Kafka, "produce" | "consume")
            | (Protocol::WebSocket, "inbound" | "outbound")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{protocols::async_brokers, RecorderDatabase};
    use serde_json::json;

    fn websocket(id: &str, method: &str, path: &str, headers: Value) -> RecordedExchange {
        serde_json::from_value(json!({"request":{"id":id,"protocol":"WebSocket","timestamp":"2026-10-20T00:00:00Z","method":method,"path":path,"headers":headers.to_string(),"body":"{\\\"id\\\":1}","body_encoding":"utf8"}})).unwrap()
    }

    #[test]
    fn websocket_frames_recover_original_paths_and_do_not_use_connection_ids_as_channels() {
        let frame = websocket("frame", "INBOUND", "/ws/conn", json!({"ws-connection-id":"conn"}));
        assert!(decode(std::slice::from_ref(&frame)).unwrap()[0].address.is_none());
        let connection = websocket("conn", "CONNECT", "/orders", json!({}));
        let records = decode(&[frame, connection]).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].address.as_deref(), Some("/orders"));
        assert!(records[0].headers.is_none());
    }

    #[test]
    fn broker_binary_payloads_decode_exactly_and_invalid_data_fails() {
        let request = async_brokers::mqtt_event("publish", "orders", 1, false, &[0, 255, 2]);
        let mut exchange = RecordedExchange {
            request,
            response: None,
        };
        assert_eq!(
            decode(std::slice::from_ref(&exchange)).unwrap()[0].payload.as_deref(),
            Some(&[0, 255, 2][..])
        );
        exchange.request.body = Some("invalid base64!".into());
        assert!(decode(&[exchange]).is_err());
        assert!(decode(&[websocket("conn", "CONNECT", "/orders", json!({}))]).is_err());
    }

    #[tokio::test]
    async fn message_query_filters_controls_and_other_protocols_before_limit() {
        let db = RecorderDatabase::new_in_memory().await.unwrap();
        let message = async_brokers::mqtt_event("publish", "orders", 1, false, b"{}");
        db.insert_request(&message).await.unwrap();
        for protocol in [
            Protocol::Http,
            Protocol::Mqtt,
            Protocol::Kafka,
            Protocol::WebSocket,
        ] {
            let mut control = async_brokers::mqtt_event("subscribe", "controls", 1, false, b"{}");
            control.protocol = protocol;
            control.timestamp = message.timestamp + chrono::Duration::seconds(10);
            db.insert_request(&control).await.unwrap();
        }
        let selected = db.list_recent_async_messages(1).await.unwrap();
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].id, message.id);
        assert!(db.list_recent_async_messages(0).await.unwrap().is_empty());
        assert!(db.list_recent_async_messages(-1).await.unwrap().is_empty());
    }
}
