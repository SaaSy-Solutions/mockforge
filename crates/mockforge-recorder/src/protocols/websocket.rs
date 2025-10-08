//! WebSocket recording helpers

use crate::{models::*, recorder::Recorder};
use chrono::Utc;
use std::collections::HashMap;
use tracing::debug;
use uuid::Uuid;

/// Record a WebSocket connection request
pub async fn record_ws_connection(
    recorder: &Recorder,
    path: &str,
    headers: &HashMap<String, String>,
    client_ip: Option<&str>,
    trace_id: Option<&str>,
    span_id: Option<&str>,
) -> Result<String, crate::RecorderError> {
    let request_id = Uuid::new_v4().to_string();

    let request = RecordedRequest {
        id: request_id.clone(),
        protocol: Protocol::WebSocket,
        timestamp: Utc::now(),
        method: "CONNECT".to_string(),
        path: path.to_string(),
        query_params: None,
        headers: serde_json::to_string(&headers)?,
        body: None,
        body_encoding: "utf8".to_string(),
        client_ip: client_ip.map(|s| s.to_string()),
        trace_id: trace_id.map(|s| s.to_string()),
        span_id: span_id.map(|s| s.to_string()),
        duration_ms: None,
        status_code: Some(101), // Switching Protocols
        tags: Some(serde_json::to_string(&vec!["websocket", "connection"])?),
    };

    recorder.record_request(request).await?;
    debug!("Recorded WebSocket connection: {} {}", request_id, path);

    Ok(request_id)
}

/// Record a WebSocket message
pub async fn record_ws_message(
    recorder: &Recorder,
    connection_id: &str,
    direction: &str, // "inbound" or "outbound"
    message: &[u8],
    is_binary: bool,
    trace_id: Option<&str>,
    span_id: Option<&str>,
) -> Result<String, crate::RecorderError> {
    let message_id = Uuid::new_v4().to_string();

    let (body_str, body_encoding) = if is_binary {
        let encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, message);
        (Some(encoded), "base64".to_string())
    } else {
        match std::str::from_utf8(message) {
            Ok(text) => (Some(text.to_string()), "utf8".to_string()),
            Err(_) => {
                let encoded =
                    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, message);
                (Some(encoded), "base64".to_string())
            }
        }
    };

    let tags = vec!["websocket", "message", direction];

    let request = RecordedRequest {
        id: message_id.clone(),
        protocol: Protocol::WebSocket,
        timestamp: Utc::now(),
        method: direction.to_uppercase(),
        path: format!("/ws/{}", connection_id),
        query_params: None,
        headers: serde_json::to_string(&HashMap::from([(
            "ws-connection-id".to_string(),
            connection_id.to_string(),
        )]))?,
        body: body_str,
        body_encoding,
        client_ip: None,
        trace_id: trace_id.map(|s| s.to_string()),
        span_id: span_id.map(|s| s.to_string()),
        duration_ms: None,
        status_code: Some(200),
        tags: Some(serde_json::to_string(&tags)?),
    };

    recorder.record_request(request).await?;
    debug!(
        "Recorded WebSocket message: {} {} {} bytes",
        message_id,
        direction,
        message.len()
    );

    Ok(message_id)
}

/// Record WebSocket disconnection
pub async fn record_ws_disconnection(
    recorder: &Recorder,
    connection_id: &str,
    reason: Option<&str>,
    duration_ms: i64,
) -> Result<(), crate::RecorderError> {
    let disconnect_id = Uuid::new_v4().to_string();

    let request = RecordedRequest {
        id: disconnect_id.clone(),
        protocol: Protocol::WebSocket,
        timestamp: Utc::now(),
        method: "DISCONNECT".to_string(),
        path: format!("/ws/{}", connection_id),
        query_params: None,
        headers: serde_json::to_string(&HashMap::from([(
            "ws-connection-id".to_string(),
            connection_id.to_string(),
        )]))?,
        body: reason.map(|r| r.to_string()),
        body_encoding: "utf8".to_string(),
        client_ip: None,
        trace_id: None,
        span_id: None,
        duration_ms: Some(duration_ms),
        status_code: Some(1000), // Normal closure
        tags: Some(serde_json::to_string(&vec!["websocket", "disconnection"])?),
    };

    recorder.record_request(request).await?;
    debug!(
        "Recorded WebSocket disconnection: {} connection={} duration={}ms",
        disconnect_id, connection_id, duration_ms
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::RecorderDatabase;

    #[tokio::test]
    async fn test_record_ws_connection() {
        let db = RecorderDatabase::new_in_memory().await.unwrap();
        let recorder = Recorder::new(db);

        let headers = HashMap::from([
            ("upgrade".to_string(), "websocket".to_string()),
            ("connection".to_string(), "Upgrade".to_string()),
        ]);

        let connection_id = record_ws_connection(
            &recorder,
            "/ws/chat",
            &headers,
            Some("127.0.0.1"),
            None,
            None,
        )
        .await
        .unwrap();

        // Verify it was recorded
        let exchange = recorder.database().get_exchange(&connection_id).await.unwrap();
        assert!(exchange.is_some());

        let exchange = exchange.unwrap();
        assert_eq!(exchange.request.protocol, Protocol::WebSocket);
        assert_eq!(exchange.request.method, "CONNECT");
    }

    #[tokio::test]
    async fn test_record_ws_message() {
        let db = RecorderDatabase::new_in_memory().await.unwrap();
        let recorder = Recorder::new(db);

        let message_id = record_ws_message(
            &recorder,
            "conn-123",
            "inbound",
            b"Hello, WebSocket!",
            false,
            None,
            None,
        )
        .await
        .unwrap();

        // Verify it was recorded
        let exchange = recorder.database().get_exchange(&message_id).await.unwrap();
        assert!(exchange.is_some());

        let exchange = exchange.unwrap();
        assert_eq!(exchange.request.protocol, Protocol::WebSocket);
        assert_eq!(exchange.request.method, "INBOUND");
    }
}
