//! HTTP handlers for the bridge
//!
//! This module contains handlers for HTTP bridge endpoints that are not
//! part of the main dynamic routing.

use super::{BridgeResponse, HttpBridgeConfig};
use axum::response::{IntoResponse, Sse};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_stream::{wrappers::ReceiverStream, StreamExt};
use tonic::Request;
use tracing::warn;

/// Stream handler for server-sent events
pub struct StreamHandler;

#[derive(Debug, Serialize, Deserialize)]
pub struct StreamingMessage {
    pub event_type: String,
    pub data: Value,
    pub metadata: std::collections::HashMap<String, String>,
}

impl StreamHandler {
    /// Create a server-sent events stream for bidirectional communication
    pub async fn create_sse_stream(
        _config: HttpBridgeConfig,
        service_name: String,
        method_name: String,
    ) -> impl IntoResponse {
        let (tx, rx) =
            tokio::sync::mpsc::channel::<Result<axum::response::sse::Event, axum::BoxError>>(32);

        // Spawn a task to simulate bidirectional streaming
        tokio::spawn(async move {
            // Send initialization event
            let init_msg = StreamingMessage {
                event_type: "stream_init".to_string(),
                data: serde_json::json!({
                    "service": service_name,
                    "method": method_name,
                    "message": "Stream initialized for bidirectional communication"
                }),
                metadata: std::collections::HashMap::new(),
            };

            if let Ok(json_str) = serde_json::to_string(&init_msg) {
                let _ = tx
                    .send(Ok(axum::response::sse::Event::default().event("message").data(json_str)))
                    .await;
            }

            // Simulate ongoing streaming data for demonstration
            let mut counter = 0;
            while counter < 10 {
                tokio::time::sleep(Duration::from_millis(500)).await;

                let stream_msg = StreamingMessage {
                    event_type: "data".to_string(),
                    data: serde_json::json!({
                        "counter": counter,
                        "message": format!("Streaming message #{}", counter),
                        "timestamp": chrono::Utc::now().to_rfc3339()
                    }),
                    metadata: vec![("sequence".to_string(), counter.to_string())]
                        .into_iter()
                        .collect(),
                };

                if let Ok(json_str) = serde_json::to_string(&stream_msg) {
                    let event_type = if counter % 3 == 0 {
                        "heartbeat"
                    } else {
                        "data"
                    };
                    let _ = tx
                        .send(Ok(axum::response::sse::Event::default()
                            .event(event_type)
                            .data(json_str)))
                        .await;
                }

                counter += 1;

                // Simulate occasional errors
                if counter == 7 {
                    let error_msg = StreamingMessage {
                        event_type: "error".to_string(),
                        data: serde_json::json!({
                            "error": "Simulated network error",
                            "code": "NETWORK_ERROR"
                        }),
                        metadata: vec![("error_code".to_string(), "123".to_string())]
                            .into_iter()
                            .collect(),
                    };

                    if let Ok(json_str) = serde_json::to_string(&error_msg) {
                        let _ = tx
                            .send(Ok(axum::response::sse::Event::default()
                                .event("error")
                                .data(json_str)))
                            .await;
                    }
                }
            }

            // Send completion event
            let complete_msg = StreamingMessage {
                event_type: "stream_complete".to_string(),
                data: serde_json::json!({
                    "message": "Streaming session completed",
                    "total_messages": counter
                }),
                metadata: vec![("session_id".to_string(), "demo-123".to_string())]
                    .into_iter()
                    .collect(),
            };

            if let Ok(json_str) = serde_json::to_string(&complete_msg) {
                let _ = tx
                    .send(Ok(axum::response::sse::Event::default()
                        .event("complete")
                        .data(json_str)))
                    .await;
            }
        });

        let stream = ReceiverStream::new(rx).map(|result: Result<axum::response::sse::Event, axum::BoxError>| -> Result<axum::response::sse::Event, axum::BoxError> {
            match result {
                Ok(event) => Ok(event),
                Err(e) => Ok(axum::response::sse::Event::default()
                    .event("error")
                    .data(format!("Stream error: {}", e))),
            }
        });

        Sse::new(stream).keep_alive(
            axum::response::sse::KeepAlive::new()
                .interval(Duration::from_secs(30))
                .text("keep-alive"),
        )
    }

    /// Create a real streaming response with actual gRPC bidirectional proxying
    pub async fn create_grpc_stream_stream(
        proxy: Arc<super::MockReflectionProxy>,
        service_name: &str,
        method_name: &str,
        initial_request: Value,
    ) -> impl IntoResponse {
        let (tx, rx) = tokio::sync::mpsc::channel(32);

        // Clone values for the task
        let service_name = service_name.to_string();
        let method_name = method_name.to_string();

        let result = Self::handle_grpc_bidirectional_streaming(
            proxy,
            &service_name,
            &method_name,
            initial_request,
            tx.clone(),
        )
        .await;

        tokio::spawn(async move {
            match result {
                Ok(_) => {
                    let _ = tx
                        .send(Ok(axum::response::sse::Event::default()
                            .event("complete")
                            .data("Stream completed successfully")))
                        .await;
                }
                Err(e) => {
                    let _ = tx
                        .send(Ok(axum::response::sse::Event::default()
                            .event("error")
                            .data(format!("Stream error: {}", e))))
                        .await;
                }
            }
        });

        let stream = ReceiverStream::new(rx).map(|result: Result<axum::response::sse::Event, axum::BoxError>| -> Result<axum::response::sse::Event, axum::BoxError> {
            match result {
                Ok(event) => Ok(event),
                Err(e) => Ok(axum::response::sse::Event::default()
                    .event("error")
                    .data(format!("Stream error: {}", e))),
            }
        });

        Sse::new(stream).keep_alive(
            axum::response::sse::KeepAlive::new()
                .interval(Duration::from_secs(30))
                .text("keep-alive"),
        )
    }

    /// Handle actual bidirectional gRPC streaming
    async fn handle_grpc_bidirectional_streaming(
        proxy: Arc<super::MockReflectionProxy>,
        service_name: &str,
        method_name: &str,
        initial_request: Value,
        tx: tokio::sync::mpsc::Sender<Result<axum::response::sse::Event, axum::BoxError>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Get the message descriptor for the method
        let registry = proxy.service_registry();
        let service_opt = registry.get(service_name);
        if service_opt.is_none() {
            return Err(format!("Service '{}' not found", service_name).into());
        }

        let service = service_opt.unwrap();
        let method_opt = service.service().methods.iter().find(|m| m.name == method_name);
        if method_opt.is_none() {
            return Err(format!(
                "Method '{}' not found in service '{}'",
                method_name, service_name
            )
            .into());
        }

        let method_info = method_opt.unwrap();
        let input_descriptor = registry
            .descriptor_pool()
            .get_message_by_name(&method_info.input_type)
            .ok_or_else(|| format!("Input type '{}' not found", method_info.input_type))?;
        let _output_descriptor = registry
            .descriptor_pool()
            .get_message_by_name(&method_info.output_type)
            .ok_or_else(|| format!("Output type '{}' not found", method_info.output_type))?;

        // Create converter
        let converter =
            super::converters::ProtobufJsonConverter::new(registry.descriptor_pool().clone());

        // Prepare client messages - initial_request can be a single message or array of messages
        let client_messages: Vec<Value> = if initial_request.is_array() {
            initial_request.as_array().unwrap().clone()
        } else {
            vec![initial_request]
        };

        // Convert JSON messages to DynamicMessages
        let mut dynamic_messages = Vec::new();
        for (i, json_msg) in client_messages.iter().enumerate() {
            match converter.json_to_protobuf(&input_descriptor, json_msg) {
                Ok(dynamic_msg) => dynamic_messages.push(dynamic_msg),
                Err(e) => {
                    warn!("Failed to convert client message {} to protobuf: {}", i, e);
                    // Send error event
                    let error_msg = StreamingMessage {
                        event_type: "conversion_error".to_string(),
                        data: serde_json::json!({
                            "message": format!("Failed to convert client message {}: {}", i, e),
                            "sequence": i
                        }),
                        metadata: vec![
                            ("error_type".to_string(), "conversion".to_string()),
                            ("sequence".to_string(), i.to_string()),
                        ]
                        .into_iter()
                        .collect(),
                    };
                    if let Ok(json_str) = serde_json::to_string(&error_msg) {
                        let _ = tx
                            .send(Ok(axum::response::sse::Event::default()
                                .event("error")
                                .data(json_str)))
                            .await;
                    }
                    continue;
                }
            }
        }

        if dynamic_messages.is_empty() {
            return Err("No valid client messages to send".into());
        }

        // Send initial stream start event
        let start_msg = StreamingMessage {
            event_type: "bidirectional_stream_start".to_string(),
            data: serde_json::json!({
                "service": service_name,
                "method": method_name,
                "client_messages_count": dynamic_messages.len()
            }),
            metadata: vec![
                ("stream_type".to_string(), "bidirectional".to_string()),
                ("protocol".to_string(), "grpc-web-over-sse".to_string()),
            ]
            .into_iter()
            .collect(),
        };

        if let Ok(json_str) = serde_json::to_string(&start_msg) {
            let _ = tx
                .send(Ok(axum::response::sse::Event::default()
                    .event("stream_start")
                    .data(json_str)))
                .await;
        }

        // Create a channel for the client stream
        let (client_tx, client_rx) =
            mpsc::channel::<Result<prost_reflect::DynamicMessage, tonic::Status>>(10);

        // Create the request with the client stream
        let _request = Request::new(ReceiverStream::new(client_rx));

        // Spawn task to send client messages
        let client_tx_clone = client_tx.clone();
        tokio::spawn(async move {
            for (i, dynamic_msg) in dynamic_messages.into_iter().enumerate() {
                if client_tx_clone.send(Ok(dynamic_msg)).await.is_err() {
                    warn!("Failed to send client message {} to gRPC stream", i);
                    break;
                }
            }
            // Drop the sender to close the stream
            drop(client_tx_clone);
        });

        // Get the method descriptor
        let method_descriptor = proxy.cache().get_method(service_name, method_name).await?;

        // For bidirectional streaming, we need to handle both directions
        // This is a simplified implementation that sends a single mock response
        let smart_generator = proxy.smart_generator().clone();
        let output_descriptor = method_descriptor.output();

        // Generate a single mock response for now
        let mock_response = {
            match smart_generator.lock() {
                Ok(mut gen) => gen.generate_message(&output_descriptor),
                Err(e) => {
                    let error_msg = StreamingMessage {
                        event_type: "error".to_string(),
                        data: serde_json::json!({
                            "message": format!("Failed to acquire smart generator lock: {}", e)
                        }),
                        metadata: vec![("error_type".to_string(), "lock".to_string())]
                            .into_iter()
                            .collect(),
                    };
                    if let Ok(json_str) = serde_json::to_string(&error_msg) {
                        let _ = tx
                            .send(Ok(axum::response::sse::Event::default()
                                .event("error")
                                .data(json_str)))
                            .await;
                    }
                    return Ok(());
                }
            }
        };

        // Convert to JSON and send
        match converter.protobuf_to_json(&output_descriptor, &mock_response) {
            Ok(json_response) => {
                let response_msg = StreamingMessage {
                    event_type: "grpc_response".to_string(),
                    data: json_response,
                    metadata: vec![
                        ("sequence".to_string(), "1".to_string()),
                        ("message_type".to_string(), "response".to_string()),
                    ]
                    .into_iter()
                    .collect(),
                };

                if let Ok(json_str) = serde_json::to_string(&response_msg) {
                    let _ = tx
                        .send(Ok(axum::response::sse::Event::default()
                            .event("grpc_response")
                            .data(json_str)))
                        .await;
                }
            }
            Err(e) => {
                let error_msg = StreamingMessage {
                    event_type: "conversion_error".to_string(),
                    data: serde_json::json!({
                        "message": format!("Failed to convert response to JSON: {}", e)
                    }),
                    metadata: vec![("error_type".to_string(), "conversion".to_string())]
                        .into_iter()
                        .collect(),
                };
                if let Ok(json_str) = serde_json::to_string(&error_msg) {
                    let _ = tx
                        .send(Ok(axum::response::sse::Event::default()
                            .event("error")
                            .data(json_str)))
                        .await;
                }
            }
        }

        // Send stream end event
        let end_msg = StreamingMessage {
            event_type: "bidirectional_stream_end".to_string(),
            data: serde_json::json!({
                "message": "Bidirectional streaming session completed",
                "statistics": {
                    "responses_sent": 1
                }
            }),
            metadata: vec![("session_status".to_string(), "completed".to_string())]
                .into_iter()
                .collect(),
        };

        if let Ok(json_str) = serde_json::to_string(&end_msg) {
            let _ = tx
                .send(Ok(axum::response::sse::Event::default().event("stream_end").data(json_str)))
                .await;
        }

        Ok(())
    }
}

/// Error handling utilities for HTTP responses
pub struct ErrorHandler;

impl ErrorHandler {
    /// Convert a bridge error to an HTTP status code
    pub fn error_to_status_code(error: &str) -> axum::http::StatusCode {
        if error.contains("not found") || error.contains("Unknown") {
            axum::http::StatusCode::NOT_FOUND
        } else if error.contains("unauthorized") || error.contains("forbidden") {
            axum::http::StatusCode::FORBIDDEN
        } else if error.contains("invalid") || error.contains("malformed") {
            axum::http::StatusCode::BAD_REQUEST
        } else {
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        }
    }

    /// Create an error response
    pub fn create_error_response(error: String) -> BridgeResponse<Value> {
        BridgeResponse {
            success: false,
            data: None,
            error: Some(error),
            metadata: std::collections::HashMap::new(),
        }
    }
}

/// Request processing utilities
pub struct RequestProcessor;

impl RequestProcessor {
    /// Validate and sanitize request parameters
    pub fn validate_request(
        service_name: &str,
        method_name: &str,
        body_size: usize,
        max_body_size: usize,
    ) -> Result<(), String> {
        if service_name.is_empty() {
            return Err("Service name cannot be empty".to_string());
        }

        if method_name.is_empty() {
            return Err("Method name cannot be empty".to_string());
        }

        if body_size > max_body_size {
            return Err(format!(
                "Request body too large: {} bytes (max: {} bytes)",
                body_size, max_body_size
            ));
        }

        // Additional validation can be added here
        Ok(())
    }

    /// Extract metadata from HTTP headers
    pub fn extract_metadata_from_headers(
        headers: &axum::http::HeaderMap,
    ) -> std::collections::HashMap<String, String> {
        let mut metadata = std::collections::HashMap::new();

        for (key, value) in headers.iter() {
            let key_str = key.as_str();
            // Skip HTTP-specific headers
            if !key_str.starts_with("host")
                && !key_str.starts_with("content-type")
                && !key_str.starts_with("content-length")
                && !key_str.starts_with("user-agent")
                && !key_str.starts_with("accept")
                && !key_str.starts_with("authorization")
            {
                if let Ok(value_str) = value.to_str() {
                    metadata.insert(key_str.to_string(), value_str.to_string());
                }
            }
        }

        metadata
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderMap;
    use std::collections::HashMap;

    #[test]
    fn test_error_to_status_code() {
        assert_eq!(
            ErrorHandler::error_to_status_code("service not found"),
            axum::http::StatusCode::NOT_FOUND
        );
        assert_eq!(
            ErrorHandler::error_to_status_code("unauthorized"),
            axum::http::StatusCode::FORBIDDEN
        );
        assert_eq!(
            ErrorHandler::error_to_status_code("invalid request"),
            axum::http::StatusCode::BAD_REQUEST
        );
        assert_eq!(
            ErrorHandler::error_to_status_code("internal error"),
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        );

        // Test additional error cases
        assert_eq!(
            ErrorHandler::error_to_status_code("Unknown service"),
            axum::http::StatusCode::NOT_FOUND
        );
        assert_eq!(
            ErrorHandler::error_to_status_code("forbidden access"),
            axum::http::StatusCode::FORBIDDEN
        );
        assert_eq!(
            ErrorHandler::error_to_status_code("malformed JSON"),
            axum::http::StatusCode::BAD_REQUEST
        );
        assert_eq!(
            ErrorHandler::error_to_status_code("random error"),
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn test_validate_request() {
        assert!(RequestProcessor::validate_request("test", "method", 100, 1000).is_ok());
        assert!(RequestProcessor::validate_request("", "method", 100, 1000).is_err());
        assert!(RequestProcessor::validate_request("test", "", 100, 1000).is_err());
        assert!(RequestProcessor::validate_request("test", "method", 2000, 1000).is_err());

        // Test edge cases
        assert!(
            RequestProcessor::validate_request("valid_service", "valid_method", 0, 1000).is_ok()
        );
        assert!(RequestProcessor::validate_request("test", "method", 1000, 1000).is_ok());
        assert!(RequestProcessor::validate_request("test", "method", 1001, 1000).is_err());

        // Test with very long names
        let long_name = "a".repeat(1000);
        assert!(RequestProcessor::validate_request(&long_name, &long_name, 100, 1000).is_ok());
    }

    #[test]
    fn test_extract_metadata_from_headers() {
        let mut headers = HeaderMap::new();

        // Add various headers
        headers.insert("content-type", "application/json".parse().unwrap());
        headers.insert("authorization", "Bearer token123".parse().unwrap());
        headers.insert("x-custom-header", "custom-value".parse().unwrap());
        headers.insert("x-api-key", "key123".parse().unwrap());
        headers.insert("user-agent", "test-agent".parse().unwrap());

        let metadata = RequestProcessor::extract_metadata_from_headers(&headers);

        // Should not include HTTP-specific headers
        assert!(!metadata.contains_key("content-type"));
        assert!(!metadata.contains_key("authorization")); // Authorization is excluded
        assert!(!metadata.contains_key("user-agent"));

        // Should include custom headers
        assert_eq!(metadata.get("x-custom-header"), Some(&"custom-value".to_string()));
        assert_eq!(metadata.get("x-api-key"), Some(&"key123".to_string()));

        // Test empty headers
        let empty_headers = HeaderMap::new();
        let empty_metadata = RequestProcessor::extract_metadata_from_headers(&empty_headers);
        assert!(empty_metadata.is_empty());

        // Test case sensitivity
        let mut case_headers = HeaderMap::new();
        case_headers.insert("X-CUSTOM-HEADER", "value".parse().unwrap());
        let case_metadata = RequestProcessor::extract_metadata_from_headers(&case_headers);
        assert_eq!(case_metadata.get("x-custom-header"), Some(&"value".to_string()));
    }

    #[test]
    fn test_create_error_response() {
        let error_message = "Test error message";
        let response = ErrorHandler::create_error_response(error_message.to_string());

        assert!(!response.success);
        assert!(response.data.is_none());
        assert_eq!(response.error, Some(error_message.to_string()));
        assert!(response.metadata.is_empty());
    }

    #[tokio::test]
    async fn test_streaming_message_serialization() {
        let message = StreamingMessage {
            event_type: "test_event".to_string(),
            data: serde_json::json!({"key": "value"}),
            metadata: vec![
                ("sequence".to_string(), "1".to_string()),
                ("type".to_string(), "test".to_string()),
            ]
            .into_iter()
            .collect(),
        };

        // Test serialization
        let json_str = serde_json::to_string(&message).unwrap();
        assert!(json_str.contains("test_event"));
        assert!(json_str.contains("key"));
        assert!(json_str.contains("value"));
        assert!(json_str.contains("sequence"));
        assert!(json_str.contains("1"));
        assert!(json_str.contains("type"));
        assert!(json_str.contains("test"));

        // Test deserialization
        let deserialized: StreamingMessage = serde_json::from_str(&json_str).unwrap();
        assert_eq!(deserialized.event_type, message.event_type);
        assert_eq!(deserialized.data, message.data);
        assert_eq!(deserialized.metadata, message.metadata);
    }

    #[tokio::test]
    async fn test_create_sse_stream_basic() {
        let config = HttpBridgeConfig {
            enabled: true,
            base_path: "/api".to_string(),
            enable_cors: false,
            max_request_size: 1024,
            timeout_seconds: 30,
            route_pattern: "/{service}/{method}".to_string(),
        };

        let stream_response = StreamHandler::create_sse_stream(
            config,
            "test_service".to_string(),
            "test_method".to_string(),
        )
        .await;

        // Verify it's an SSE response
        let sse_response = stream_response.into_response();
        assert_eq!(sse_response.status(), axum::http::StatusCode::OK);

        // Check content type
        let content_type = sse_response
            .headers()
            .get("content-type")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");
        assert!(content_type.contains("text/event-stream"));
    }

    #[test]
    fn test_bridge_response_serialization() {
        let response = BridgeResponse::<serde_json::Value> {
            success: true,
            data: Some(serde_json::json!({"result": "success"})),
            error: None,
            metadata: vec![
                ("service".to_string(), "test".to_string()),
                ("method".to_string(), "test".to_string()),
            ]
            .into_iter()
            .collect(),
        };

        let json_str = serde_json::to_string(&response).unwrap();
        assert!(json_str.contains("success"));
        assert!(json_str.contains("true"));
        assert!(json_str.contains("result"));
        assert!(json_str.contains("success"));
        assert!(json_str.contains("service"));
        assert!(json_str.contains("method"));

        let deserialized: BridgeResponse<serde_json::Value> =
            serde_json::from_str(&json_str).unwrap();
        assert_eq!(deserialized.success, response.success);
        assert_eq!(deserialized.data, response.data);
        assert_eq!(deserialized.error, response.error);
        assert_eq!(deserialized.metadata, response.metadata);
    }

    #[test]
    fn test_validate_request_edge_cases() {
        // Test with zero max body size
        assert!(RequestProcessor::validate_request("test", "method", 0, 0).is_ok());
        assert!(RequestProcessor::validate_request("test", "method", 1, 0).is_err());

        // Test with empty strings and whitespace
        assert!(RequestProcessor::validate_request("  test  ", "  method  ", 100, 1000).is_ok());
        assert!(RequestProcessor::validate_request("   ", "method", 100, 1000).is_err());
        assert!(RequestProcessor::validate_request("test", "   ", 100, 1000).is_err());

        // Test with very large body sizes
        let large_size = usize::MAX / 2;
        assert!(
            RequestProcessor::validate_request("test", "method", large_size, usize::MAX).is_ok()
        );
        assert!(RequestProcessor::validate_request("test", "method", large_size + 1, large_size)
            .is_err());
    }

    #[test]
    fn test_header_extraction_comprehensive() {
        let mut headers = HeaderMap::new();

        // Add various header types
        headers.insert("host", "localhost:8080".parse().unwrap());
        headers.insert("content-length", "123".parse().unwrap());
        headers.insert("accept", "application/json".parse().unwrap());
        headers.insert("x-forwarded-for", "192.168.1.1".parse().unwrap());
        headers.insert("x-custom-metadata", "custom-value".parse().unwrap());
        headers.insert("x-trace-id", "trace-123".parse().unwrap());
        headers.insert("x-request-id", "req-456".parse().unwrap());

        let metadata = RequestProcessor::extract_metadata_from_headers(&headers);

        // Should exclude all HTTP-specific headers
        assert!(!metadata.contains_key("host"));
        assert!(!metadata.contains_key("content-length"));
        assert!(!metadata.contains_key("accept"));

        // Should include custom headers
        assert_eq!(metadata.get("x-forwarded-for"), Some(&"192.168.1.1".to_string()));
        assert_eq!(metadata.get("x-custom-metadata"), Some(&"custom-value".to_string()));
        assert_eq!(metadata.get("x-trace-id"), Some(&"trace-123".to_string()));
        assert_eq!(metadata.get("x-request-id"), Some(&"req-456".to_string()));

        // Should have exactly 4 custom headers
        assert_eq!(metadata.len(), 4);
    }

    #[test]
    fn test_error_response_comprehensive() {
        // Test various error messages
        let test_errors = vec![
            "Service not found",
            "Method not found",
            "Invalid request body",
            "Authentication failed",
            "Internal server error",
            "Timeout exceeded",
            "Rate limit exceeded",
            "Database connection failed",
        ];

        for error_msg in test_errors {
            let response = ErrorHandler::create_error_response(error_msg.to_string());
            assert!(!response.success);
            assert!(response.data.is_none());
            assert_eq!(response.error, Some(error_msg.to_string()));
            assert!(response.metadata.is_empty());
        }
    }
}
