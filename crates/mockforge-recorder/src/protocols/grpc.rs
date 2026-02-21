//! gRPC recording helpers

use crate::{models::*, recorder::Recorder};
use chrono::Utc;
use std::collections::HashMap;
use tracing::debug;
use uuid::Uuid;

/// Record a gRPC request
pub async fn record_grpc_request(
    recorder: &Recorder,
    service: &str,
    method: &str,
    metadata: &HashMap<String, String>,
    message: Option<&[u8]>,
    context: &RequestContext,
) -> Result<String, crate::RecorderError> {
    let request_id = Uuid::new_v4().to_string();
    let full_method = format!("{}/{}", service, method);

    let (body_str, body_encoding) = if let Some(msg) = message {
        // gRPC messages are protobuf, so always base64 encode
        let encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, msg);
        (Some(encoded), "base64".to_string())
    } else {
        (None, "utf8".to_string())
    };

    let request = RecordedRequest {
        id: request_id.clone(),
        protocol: Protocol::Grpc,
        timestamp: Utc::now(),
        method: full_method.clone(),
        path: format!("/{}", full_method),
        query_params: None,
        headers: serde_json::to_string(&metadata)?,
        body: body_str,
        body_encoding,
        client_ip: context.client_ip.clone(),
        trace_id: context.trace_id.clone(),
        span_id: context.span_id.clone(),
        duration_ms: None,
        status_code: None,
        tags: None,
    };

    recorder.record_request(request).await?;
    debug!("Recorded gRPC request: {} {}", request_id, full_method);

    Ok(request_id)
}

/// Record a gRPC response
pub async fn record_grpc_response(
    recorder: &Recorder,
    request_id: &str,
    status_code: i32, // gRPC status code (0 = OK)
    metadata: &HashMap<String, String>,
    message: Option<&[u8]>,
    duration_ms: i64,
) -> Result<(), crate::RecorderError> {
    let (body_str, body_encoding) = if let Some(msg) = message {
        let encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, msg);
        (Some(encoded), "base64".to_string())
    } else {
        (None, "utf8".to_string())
    };

    let size_bytes = message.map(|m| m.len()).unwrap_or(0) as i64;

    let response = RecordedResponse {
        request_id: request_id.to_string(),
        status_code,
        headers: serde_json::to_string(&metadata)?,
        body: body_str,
        body_encoding,
        size_bytes,
        timestamp: Utc::now(),
    };

    recorder.record_response(response).await?;
    debug!(
        "Recorded gRPC response: {} status={} duration={}ms",
        request_id, status_code, duration_ms
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::RecorderDatabase;

    #[tokio::test]
    async fn test_record_grpc_exchange() {
        let db = RecorderDatabase::new_in_memory().await.unwrap();
        let recorder = Recorder::new(db);

        let metadata =
            HashMap::from([("content-type".to_string(), "application/grpc".to_string())]);

        let context = crate::models::RequestContext::new(Some("127.0.0.1"), None, None);
        let request_id = record_grpc_request(
            &recorder,
            "helloworld.Greeter",
            "SayHello",
            &metadata,
            Some(b"\x00\x00\x00\x00\x05hello"),
            &context,
        )
        .await
        .unwrap();

        record_grpc_response(
            &recorder,
            &request_id,
            0,
            &metadata,
            Some(b"\x00\x00\x00\x00\x05world"),
            42,
        )
        .await
        .unwrap();

        // Verify it was recorded
        let exchange = recorder.database().get_exchange(&request_id).await.unwrap();
        assert!(exchange.is_some());

        let exchange = exchange.unwrap();
        assert_eq!(exchange.request.protocol, Protocol::Grpc);
        assert_eq!(exchange.request.method, "helloworld.Greeter/SayHello");
    }
}
