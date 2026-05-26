//! Convenience builders for recording Kafka / MQTT / AMQP exchanges (#683).
//!
//! Each broker server can call these to drop an event into the recorder
//! sqlite with a consistent shape, so replay + diff tooling sees a
//! single uniform schema regardless of source protocol. Without this
//! shim every broker would invent its own \`RecordedRequest\` shape and
//! the query side would have to special-case each one.
//!
//! Storage convention:
//! - \`method\` is the broker-op verb (produce/consume, publish/subscribe,
//!   publish/deliver/ack/nack)
//! - \`path\` is the topic name (Kafka, MQTT) or
//!   \`<exchange>/<routing-key>\` (AMQP publish) / queue name (AMQP consume)
//! - \`query_params\` carries protocol-specific metadata as compact JSON
//!   so we don't need a column-migration per protocol
//! - \`body\` carries the message payload UTF-8 when valid, base64
//!   otherwise (matches the HTTP/gRPC convention)

use crate::models::{Protocol, RecordedRequest};
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

/// Encode an opaque payload for storage. UTF-8 when the bytes parse
/// cleanly, base64 otherwise — mirrors how the HTTP recorder handles
/// binary bodies.
fn encode_body(payload: &[u8]) -> (Option<String>, String) {
    match std::str::from_utf8(payload) {
        Ok(s) => (Some(s.to_string()), "utf8".to_string()),
        Err(_) => {
            use base64::Engine;
            (
                Some(base64::engine::general_purpose::STANDARD.encode(payload)),
                "base64".to_string(),
            )
        }
    }
}

/// Build a `RecordedRequest` for a Kafka produce/consume exchange.
///
/// `op` is "produce" or "consume". `partition` / `offset` / `key` are
/// optional — they go into `query_params` as compact JSON.
pub fn kafka_event(
    op: &str,
    topic: &str,
    partition: Option<i32>,
    offset: Option<i64>,
    key: Option<&str>,
    payload: &[u8],
) -> RecordedRequest {
    let (body, body_encoding) = encode_body(payload);
    let meta = json!({
        "partition": partition,
        "offset": offset,
        "key": key,
    });
    RecordedRequest {
        id: Uuid::new_v4().to_string(),
        protocol: Protocol::Kafka,
        timestamp: Utc::now(),
        method: op.to_string(),
        path: topic.to_string(),
        query_params: Some(meta.to_string()),
        headers: "{}".to_string(),
        body,
        body_encoding,
        client_ip: None,
        trace_id: None,
        span_id: None,
        duration_ms: None,
        status_code: None,
        tags: None,
    }
}

/// Build a `RecordedRequest` for an MQTT publish/subscribe.
///
/// `op` is "publish" or "subscribe". QoS + retain go into
/// `query_params`.
pub fn mqtt_event(op: &str, topic: &str, qos: u8, retain: bool, payload: &[u8]) -> RecordedRequest {
    let (body, body_encoding) = encode_body(payload);
    let meta = json!({
        "qos": qos,
        "retain": retain,
    });
    RecordedRequest {
        id: Uuid::new_v4().to_string(),
        protocol: Protocol::Mqtt,
        timestamp: Utc::now(),
        method: op.to_string(),
        path: topic.to_string(),
        query_params: Some(meta.to_string()),
        headers: "{}".to_string(),
        body,
        body_encoding,
        client_ip: None,
        trace_id: None,
        span_id: None,
        duration_ms: None,
        status_code: None,
        tags: None,
    }
}

/// Build a `RecordedRequest` for an AMQP basic-op event.
///
/// For publish, pass `exchange` + `routing_key` (joined as the path);
/// for consumer-side ops (deliver/ack/nack) pass the queue name as
/// `routing_key` and leave `exchange` as empty.
pub fn amqp_event(op: &str, exchange: &str, routing_key: &str, payload: &[u8]) -> RecordedRequest {
    let (body, body_encoding) = encode_body(payload);
    let path = if exchange.is_empty() {
        routing_key.to_string()
    } else {
        format!("{exchange}/{routing_key}")
    };
    RecordedRequest {
        id: Uuid::new_v4().to_string(),
        protocol: Protocol::Amqp,
        timestamp: Utc::now(),
        method: op.to_string(),
        path,
        query_params: None,
        headers: "{}".to_string(),
        body,
        body_encoding,
        client_ip: None,
        trace_id: None,
        span_id: None,
        duration_ms: None,
        status_code: None,
        tags: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kafka_event_encodes_utf8_payload_as_utf8() {
        let r = kafka_event("produce", "orders", Some(0), Some(42), Some("k1"), b"hello");
        assert_eq!(r.protocol, Protocol::Kafka);
        assert_eq!(r.method, "produce");
        assert_eq!(r.path, "orders");
        assert_eq!(r.body.as_deref(), Some("hello"));
        assert_eq!(r.body_encoding, "utf8");
        let meta: serde_json::Value =
            serde_json::from_str(r.query_params.as_ref().unwrap()).unwrap();
        assert_eq!(meta["partition"], 0);
        assert_eq!(meta["offset"], 42);
        assert_eq!(meta["key"], "k1");
    }

    #[test]
    fn kafka_event_encodes_binary_payload_as_base64() {
        // Invalid UTF-8 should round-trip via base64.
        let r = kafka_event("produce", "orders", None, None, None, &[0xff, 0xfe, 0x00]);
        assert_eq!(r.body_encoding, "base64");
        assert!(r.body.is_some());
    }

    #[test]
    fn mqtt_event_carries_qos_and_retain() {
        let r = mqtt_event("publish", "sensors/temp", 1, true, b"22.5");
        assert_eq!(r.protocol, Protocol::Mqtt);
        assert_eq!(r.method, "publish");
        assert_eq!(r.path, "sensors/temp");
        let meta: serde_json::Value =
            serde_json::from_str(r.query_params.as_ref().unwrap()).unwrap();
        assert_eq!(meta["qos"], 1);
        assert_eq!(meta["retain"], true);
    }

    #[test]
    fn amqp_event_joins_exchange_and_routing_key() {
        let r = amqp_event("publish", "orders", "order.created", b"{}");
        assert_eq!(r.path, "orders/order.created");
    }

    #[test]
    fn amqp_event_consumer_side_uses_queue_name() {
        let r = amqp_event("deliver", "", "orders.new", b"{}");
        assert_eq!(r.path, "orders.new");
    }
}
