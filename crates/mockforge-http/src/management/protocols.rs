#[cfg(any(feature = "smtp", feature = "mqtt", feature = "kafka"))]
use axum::extract::Path;
#[cfg(any(feature = "mqtt", feature = "kafka"))]
use axum::extract::Query;
#[cfg(any(feature = "mqtt", feature = "kafka"))]
use axum::response::sse::{Event, Sse};
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
};
#[cfg(any(feature = "mqtt", feature = "kafka"))]
use futures::stream::{self, Stream};
#[cfg(any(feature = "mqtt", feature = "kafka"))]
use serde::{Deserialize, Serialize};
#[cfg(any(feature = "mqtt", feature = "kafka"))]
use std::convert::Infallible;
#[cfg(any(feature = "mqtt", feature = "kafka"))]
use tokio::sync::broadcast;
#[cfg(any(feature = "mqtt", feature = "kafka"))]
use tracing::*;

use super::ManagementState;
#[cfg(any(feature = "mqtt", feature = "kafka"))]
use super::MessageEvent;
#[cfg(feature = "mqtt")]
use super::MqttMessageEvent;

// ========== SMTP Handlers ==========

#[cfg(feature = "smtp")]
use mockforge_smtp::EmailSearchFilters;

#[cfg(feature = "smtp")]
/// List SMTP emails in mailbox
pub(crate) async fn list_smtp_emails(State(state): State<ManagementState>) -> impl IntoResponse {
    if let Some(ref smtp_registry) = state.smtp_registry {
        match smtp_registry.get_emails() {
            Ok(emails) => (StatusCode::OK, Json(serde_json::json!(emails))),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to retrieve emails",
                    "message": e.to_string()
                })),
            ),
        }
    } else {
        (
            StatusCode::NOT_IMPLEMENTED,
            Json(serde_json::json!({
                "error": "SMTP mailbox management not available",
                "message": "SMTP server is not enabled or registry not available."
            })),
        )
    }
}

/// Get specific SMTP email
#[cfg(feature = "smtp")]
pub(crate) async fn get_smtp_email(
    State(state): State<ManagementState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if let Some(ref smtp_registry) = state.smtp_registry {
        match smtp_registry.get_email_by_id(&id) {
            Ok(Some(email)) => (StatusCode::OK, Json(serde_json::json!(email))),
            Ok(None) => (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": "Email not found",
                    "id": id
                })),
            ),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to retrieve email",
                    "message": e.to_string()
                })),
            ),
        }
    } else {
        (
            StatusCode::NOT_IMPLEMENTED,
            Json(serde_json::json!({
                "error": "SMTP mailbox management not available",
                "message": "SMTP server is not enabled or registry not available."
            })),
        )
    }
}

/// Clear SMTP mailbox
#[cfg(feature = "smtp")]
pub(crate) async fn clear_smtp_mailbox(State(state): State<ManagementState>) -> impl IntoResponse {
    if let Some(ref smtp_registry) = state.smtp_registry {
        match smtp_registry.clear_mailbox() {
            Ok(()) => (
                StatusCode::OK,
                Json(serde_json::json!({
                    "message": "Mailbox cleared successfully"
                })),
            ),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to clear mailbox",
                    "message": e.to_string()
                })),
            ),
        }
    } else {
        (
            StatusCode::NOT_IMPLEMENTED,
            Json(serde_json::json!({
                "error": "SMTP mailbox management not available",
                "message": "SMTP server is not enabled or registry not available."
            })),
        )
    }
}

/// Export SMTP mailbox
#[cfg(feature = "smtp")]
pub(crate) async fn export_smtp_mailbox(
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let format = params.get("format").unwrap_or(&"json".to_string()).clone();
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({
            "error": "SMTP mailbox management not available via HTTP API",
            "message": "SMTP server runs separately from HTTP server. Use CLI commands to access mailbox.",
            "requested_format": format
        })),
    )
}

/// Search SMTP emails
#[cfg(feature = "smtp")]
pub(crate) async fn search_smtp_emails(
    State(state): State<ManagementState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    if let Some(ref smtp_registry) = state.smtp_registry {
        let filters = EmailSearchFilters {
            sender: params.get("sender").cloned(),
            recipient: params.get("recipient").cloned(),
            subject: params.get("subject").cloned(),
            body: params.get("body").cloned(),
            since: params
                .get("since")
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&chrono::Utc)),
            until: params
                .get("until")
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&chrono::Utc)),
            use_regex: params.get("regex").map(|s| s == "true").unwrap_or(false),
            case_sensitive: params.get("case_sensitive").map(|s| s == "true").unwrap_or(false),
        };

        match smtp_registry.search_emails(filters) {
            Ok(emails) => (StatusCode::OK, Json(serde_json::json!(emails))),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to search emails",
                    "message": e.to_string()
                })),
            ),
        }
    } else {
        (
            StatusCode::NOT_IMPLEMENTED,
            Json(serde_json::json!({
                "error": "SMTP mailbox management not available",
                "message": "SMTP server is not enabled or registry not available."
            })),
        )
    }
}

// ========== MQTT Handlers ==========

/// MQTT broker statistics
#[cfg(feature = "mqtt")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttBrokerStats {
    /// Number of connected MQTT clients
    pub connected_clients: usize,
    /// Number of active MQTT topics
    pub active_topics: usize,
    /// Number of retained messages
    pub retained_messages: usize,
    /// Total number of subscriptions
    pub total_subscriptions: usize,
}

/// MQTT management handlers
#[cfg(feature = "mqtt")]
pub(crate) async fn get_mqtt_stats(State(state): State<ManagementState>) -> impl IntoResponse {
    if let Some(broker) = &state.mqtt_broker {
        let connected_clients = broker.get_connected_clients().await.len();
        let active_topics = broker.get_active_topics().await.len();
        let stats = broker.get_topic_stats().await;

        let broker_stats = MqttBrokerStats {
            connected_clients,
            active_topics,
            retained_messages: stats.retained_messages,
            total_subscriptions: stats.total_subscriptions,
        };

        Json(broker_stats).into_response()
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "MQTT broker not available").into_response()
    }
}

#[cfg(feature = "mqtt")]
pub(crate) async fn get_mqtt_clients(State(state): State<ManagementState>) -> impl IntoResponse {
    if let Some(broker) = &state.mqtt_broker {
        let clients = broker.get_connected_clients().await;
        Json(serde_json::json!({
            "clients": clients
        }))
        .into_response()
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "MQTT broker not available").into_response()
    }
}

#[cfg(feature = "mqtt")]
pub(crate) async fn get_mqtt_topics(State(state): State<ManagementState>) -> impl IntoResponse {
    if let Some(broker) = &state.mqtt_broker {
        let topics = broker.get_active_topics().await;
        Json(serde_json::json!({
            "topics": topics
        }))
        .into_response()
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "MQTT broker not available").into_response()
    }
}

#[cfg(feature = "mqtt")]
pub(crate) async fn disconnect_mqtt_client(
    State(state): State<ManagementState>,
    Path(client_id): Path<String>,
) -> impl IntoResponse {
    if let Some(broker) = &state.mqtt_broker {
        match broker.disconnect_client(&client_id).await {
            Ok(_) => {
                (StatusCode::OK, format!("Client '{}' disconnected", client_id)).into_response()
            }
            Err(e) => {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to disconnect client: {}", e))
                    .into_response()
            }
        }
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "MQTT broker not available").into_response()
    }
}

// ========== MQTT Publish Handler ==========

#[cfg(feature = "mqtt")]
/// Request to publish a single MQTT message
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct MqttPublishRequest {
    /// Topic to publish to
    pub topic: String,
    /// Message payload (string or JSON)
    pub payload: String,
    /// QoS level (0, 1, or 2)
    #[serde(default = "default_qos")]
    pub qos: u8,
    /// Whether to retain the message
    #[serde(default)]
    pub retain: bool,
}

#[cfg(feature = "mqtt")]
#[allow(dead_code)]
fn default_qos() -> u8 {
    0
}

#[cfg(feature = "mqtt")]
/// Publish a message to an MQTT topic (only compiled when mqtt feature is enabled)
pub(crate) async fn publish_mqtt_message_handler(
    State(state): State<ManagementState>,
    Json(request): Json<serde_json::Value>,
) -> impl IntoResponse {
    // Extract fields from JSON manually
    let topic = request.get("topic").and_then(|v| v.as_str()).map(|s| s.to_string());
    let payload = request.get("payload").and_then(|v| v.as_str()).map(|s| s.to_string());
    let qos = request.get("qos").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
    let retain = request.get("retain").and_then(|v| v.as_bool()).unwrap_or(false);

    if topic.is_none() || payload.is_none() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Invalid request",
                "message": "Missing required fields: topic and payload"
            })),
        );
    }

    let topic = topic.unwrap();
    let payload = payload.unwrap();

    if let Some(broker) = &state.mqtt_broker {
        // Validate QoS
        if qos > 2 {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Invalid QoS",
                    "message": "QoS must be 0, 1, or 2"
                })),
            );
        }

        // Convert payload to bytes
        let payload_bytes = payload.as_bytes().to_vec();
        let client_id = "mockforge-management-api".to_string();

        let publish_result = broker
            .handle_publish(&client_id, &topic, payload_bytes, qos, retain)
            .await
            .map_err(|e| format!("{}", e));

        match publish_result {
            Ok(_) => {
                // Emit message event for real-time monitoring
                let event = MessageEvent::Mqtt(MqttMessageEvent {
                    topic: topic.clone(),
                    payload: payload.clone(),
                    qos,
                    retain,
                    timestamp: chrono::Utc::now().to_rfc3339(),
                });
                let _ = state.message_events.send(event);

                (
                    StatusCode::OK,
                    Json(serde_json::json!({
                        "success": true,
                        "message": format!("Message published to topic '{}'", topic),
                        "topic": topic,
                        "qos": qos,
                        "retain": retain
                    })),
                )
            }
            Err(error_msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to publish message",
                    "message": error_msg
                })),
            ),
        }
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "MQTT broker not available",
                "message": "MQTT broker is not enabled or not available."
            })),
        )
    }
}

#[cfg(not(feature = "mqtt"))]
/// Publish a message to an MQTT topic (stub when mqtt feature is disabled)
pub(crate) async fn publish_mqtt_message_handler(
    State(_state): State<ManagementState>,
    Json(_request): Json<serde_json::Value>,
) -> impl IntoResponse {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(serde_json::json!({
            "error": "MQTT feature not enabled",
            "message": "MQTT support is not compiled into this build"
        })),
    )
}

#[cfg(feature = "mqtt")]
/// Request to publish multiple MQTT messages
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct MqttBatchPublishRequest {
    /// List of messages to publish
    pub messages: Vec<MqttPublishRequest>,
    /// Delay between messages in milliseconds
    #[serde(default = "default_delay")]
    pub delay_ms: u64,
}

#[cfg(feature = "mqtt")]
#[allow(dead_code)]
fn default_delay() -> u64 {
    100
}

#[cfg(feature = "mqtt")]
/// Publish multiple messages to MQTT topics (only compiled when mqtt feature is enabled)
pub(crate) async fn publish_mqtt_batch_handler(
    State(state): State<ManagementState>,
    Json(request): Json<serde_json::Value>,
) -> impl IntoResponse {
    // Extract fields from JSON manually
    let messages_json = request.get("messages").and_then(|v| v.as_array());
    let delay_ms = request.get("delay_ms").and_then(|v| v.as_u64()).unwrap_or(100);

    if messages_json.is_none() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Invalid request",
                "message": "Missing required field: messages"
            })),
        );
    }

    let messages_json = messages_json.unwrap();

    if let Some(broker) = &state.mqtt_broker {
        if messages_json.is_empty() {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Empty batch",
                    "message": "At least one message is required"
                })),
            );
        }

        let mut results = Vec::new();
        let client_id = "mockforge-management-api".to_string();

        for (index, msg_json) in messages_json.iter().enumerate() {
            let topic = msg_json.get("topic").and_then(|v| v.as_str()).map(|s| s.to_string());
            let payload = msg_json.get("payload").and_then(|v| v.as_str()).map(|s| s.to_string());
            let qos = msg_json.get("qos").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
            let retain = msg_json.get("retain").and_then(|v| v.as_bool()).unwrap_or(false);

            if topic.is_none() || payload.is_none() {
                results.push(serde_json::json!({
                    "index": index,
                    "success": false,
                    "error": "Missing required fields: topic and payload"
                }));
                continue;
            }

            let topic = topic.unwrap();
            let payload = payload.unwrap();

            // Validate QoS
            if qos > 2 {
                results.push(serde_json::json!({
                    "index": index,
                    "success": false,
                    "error": "Invalid QoS (must be 0, 1, or 2)"
                }));
                continue;
            }

            // Convert payload to bytes
            let payload_bytes = payload.as_bytes().to_vec();

            let publish_result = broker
                .handle_publish(&client_id, &topic, payload_bytes, qos, retain)
                .await
                .map_err(|e| format!("{}", e));

            match publish_result {
                Ok(_) => {
                    // Emit message event
                    let event = MessageEvent::Mqtt(MqttMessageEvent {
                        topic: topic.clone(),
                        payload: payload.clone(),
                        qos,
                        retain,
                        timestamp: chrono::Utc::now().to_rfc3339(),
                    });
                    let _ = state.message_events.send(event);

                    results.push(serde_json::json!({
                        "index": index,
                        "success": true,
                        "topic": topic,
                        "qos": qos
                    }));
                }
                Err(error_msg) => {
                    results.push(serde_json::json!({
                        "index": index,
                        "success": false,
                        "error": error_msg
                    }));
                }
            }

            // Add delay between messages (except for the last one)
            if index < messages_json.len() - 1 && delay_ms > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            }
        }

        let success_count =
            results.iter().filter(|r| r["success"].as_bool().unwrap_or(false)).count();

        (
            StatusCode::OK,
            Json(serde_json::json!({
                "success": true,
                "total": messages_json.len(),
                "succeeded": success_count,
                "failed": messages_json.len() - success_count,
                "results": results
            })),
        )
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "MQTT broker not available",
                "message": "MQTT broker is not enabled or not available."
            })),
        )
    }
}

#[cfg(not(feature = "mqtt"))]
/// Publish multiple messages to MQTT topics (stub when mqtt feature is disabled)
pub(crate) async fn publish_mqtt_batch_handler(
    State(_state): State<ManagementState>,
    Json(_request): Json<serde_json::Value>,
) -> impl IntoResponse {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(serde_json::json!({
            "error": "MQTT feature not enabled",
            "message": "MQTT support is not compiled into this build"
        })),
    )
}

// ========== MQTT SSE Stream ==========

#[cfg(feature = "mqtt")]
/// SSE stream for MQTT messages
pub(crate) async fn mqtt_messages_stream(
    State(state): State<ManagementState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.message_events.subscribe();
    let topic_filter = params.get("topic").cloned();

    let stream = stream::unfold(rx, move |mut rx| {
        let topic_filter = topic_filter.clone();

        async move {
            loop {
                match rx.recv().await {
                    Ok(MessageEvent::Mqtt(event)) => {
                        // Apply topic filter if specified
                        if let Some(filter) = &topic_filter {
                            if !event.topic.contains(filter) {
                                continue;
                            }
                        }

                        let event_json = serde_json::json!({
                            "protocol": "mqtt",
                            "topic": event.topic,
                            "payload": event.payload,
                            "qos": event.qos,
                            "retain": event.retain,
                            "timestamp": event.timestamp,
                        });

                        if let Ok(event_data) = serde_json::to_string(&event_json) {
                            let sse_event = Event::default().event("mqtt_message").data(event_data);
                            return Some((Ok(sse_event), rx));
                        }
                    }
                    #[cfg(feature = "kafka")]
                    Ok(MessageEvent::Kafka(_)) => {
                        // Skip Kafka events in MQTT stream
                        continue;
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        return None;
                    }
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        warn!("MQTT message stream lagged, skipped {} messages", skipped);
                        continue;
                    }
                }
            }
        }
    });

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(std::time::Duration::from_secs(15))
            .text("keep-alive-text"),
    )
}

// ========== Kafka Handlers ==========

#[cfg(feature = "kafka")]
use super::KafkaMessageEvent;

#[cfg(feature = "kafka")]
/// Kafka broker statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaBrokerStats {
    /// Number of topics
    pub topics: usize,
    /// Total number of partitions
    pub partitions: usize,
    /// Number of consumer groups
    pub consumer_groups: usize,
    /// Total messages produced
    pub messages_produced: u64,
    /// Total messages consumed
    pub messages_consumed: u64,
}

#[cfg(feature = "kafka")]
#[allow(missing_docs)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaTopicInfo {
    pub name: String,
    pub partitions: usize,
    pub replication_factor: i32,
}

#[cfg(feature = "kafka")]
#[allow(missing_docs)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaConsumerGroupInfo {
    pub group_id: String,
    pub members: usize,
    pub state: String,
}

#[cfg(feature = "kafka")]
/// Get Kafka broker statistics
pub(crate) async fn get_kafka_stats(State(state): State<ManagementState>) -> impl IntoResponse {
    if let Some(broker) = &state.kafka_broker {
        let topics = broker.topics.read().await;
        let consumer_groups = broker.consumer_groups.read().await;

        let total_partitions: usize = topics.values().map(|t| t.partitions.len()).sum();

        // Get metrics snapshot for message counts
        let metrics_snapshot = broker.metrics().snapshot();

        let stats = KafkaBrokerStats {
            topics: topics.len(),
            partitions: total_partitions,
            consumer_groups: consumer_groups.groups().len(),
            messages_produced: metrics_snapshot.messages_produced_total,
            messages_consumed: metrics_snapshot.messages_consumed_total,
        };

        Json(stats).into_response()
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "Kafka broker not available",
                "message": "Kafka broker is not enabled or not available."
            })),
        )
            .into_response()
    }
}

#[cfg(feature = "kafka")]
/// List Kafka topics
pub(crate) async fn get_kafka_topics(State(state): State<ManagementState>) -> impl IntoResponse {
    if let Some(broker) = &state.kafka_broker {
        let topics = broker.topics.read().await;
        let topic_list: Vec<KafkaTopicInfo> = topics
            .iter()
            .map(|(name, topic)| KafkaTopicInfo {
                name: name.clone(),
                partitions: topic.partitions.len(),
                replication_factor: topic.config.replication_factor as i32,
            })
            .collect();

        Json(serde_json::json!({
            "topics": topic_list
        }))
        .into_response()
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "Kafka broker not available",
                "message": "Kafka broker is not enabled or not available."
            })),
        )
            .into_response()
    }
}

#[cfg(feature = "kafka")]
/// Get Kafka topic details
pub(crate) async fn get_kafka_topic(
    State(state): State<ManagementState>,
    Path(topic_name): Path<String>,
) -> impl IntoResponse {
    if let Some(broker) = &state.kafka_broker {
        let topics = broker.topics.read().await;
        if let Some(topic) = topics.get(&topic_name) {
            Json(serde_json::json!({
                "name": topic_name,
                "partitions": topic.partitions.len(),
                "replication_factor": topic.config.replication_factor,
                "partitions_detail": topic.partitions.iter().enumerate().map(|(idx, partition)| serde_json::json!({
                    "id": idx as i32,
                    "leader": 0,
                    "replicas": vec![0],
                    "message_count": partition.messages.len()
                })).collect::<Vec<_>>()
            })).into_response()
        } else {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": "Topic not found",
                    "topic": topic_name
                })),
            )
                .into_response()
        }
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "Kafka broker not available",
                "message": "Kafka broker is not enabled or not available."
            })),
        )
            .into_response()
    }
}

#[cfg(feature = "kafka")]
/// List Kafka consumer groups
pub(crate) async fn get_kafka_groups(State(state): State<ManagementState>) -> impl IntoResponse {
    if let Some(broker) = &state.kafka_broker {
        let consumer_groups = broker.consumer_groups.read().await;
        let groups: Vec<KafkaConsumerGroupInfo> = consumer_groups
            .groups()
            .iter()
            .map(|(group_id, group)| KafkaConsumerGroupInfo {
                group_id: group_id.clone(),
                members: group.members.len(),
                state: "Stable".to_string(), // Simplified - could be more detailed
            })
            .collect();

        Json(serde_json::json!({
            "groups": groups
        }))
        .into_response()
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "Kafka broker not available",
                "message": "Kafka broker is not enabled or not available."
            })),
        )
            .into_response()
    }
}

#[cfg(feature = "kafka")]
/// Get Kafka consumer group details
pub(crate) async fn get_kafka_group(
    State(state): State<ManagementState>,
    Path(group_id): Path<String>,
) -> impl IntoResponse {
    if let Some(broker) = &state.kafka_broker {
        let consumer_groups = broker.consumer_groups.read().await;
        if let Some(group) = consumer_groups.groups().get(&group_id) {
            Json(serde_json::json!({
                "group_id": group_id,
                "members": group.members.len(),
                "state": "Stable",
                "members_detail": group.members.iter().map(|(member_id, member)| serde_json::json!({
                    "member_id": member_id,
                    "client_id": member.client_id,
                    "assignments": member.assignment.iter().map(|a| serde_json::json!({
                        "topic": a.topic,
                        "partitions": a.partitions
                    })).collect::<Vec<_>>()
                })).collect::<Vec<_>>(),
                "offsets": group.offsets.iter().map(|((topic, partition), offset)| serde_json::json!({
                    "topic": topic,
                    "partition": partition,
                    "offset": offset
                })).collect::<Vec<_>>()
            })).into_response()
        } else {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": "Consumer group not found",
                    "group_id": group_id
                })),
            )
                .into_response()
        }
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "Kafka broker not available",
                "message": "Kafka broker is not enabled or not available."
            })),
        )
            .into_response()
    }
}

// ========== Kafka Produce Handler ==========

#[cfg(feature = "kafka")]
/// Request body for producing a Kafka message
#[derive(Debug, Deserialize)]
pub struct KafkaProduceRequest {
    /// Topic to produce to
    pub topic: String,
    /// Message key (optional)
    #[serde(default)]
    pub key: Option<String>,
    /// Message value (JSON string or plain string)
    pub value: String,
    /// Partition ID (optional, auto-assigned if not provided)
    #[serde(default)]
    pub partition: Option<i32>,
    /// Message headers (optional, key-value pairs)
    #[serde(default)]
    pub headers: Option<std::collections::HashMap<String, String>>,
}

#[cfg(feature = "kafka")]
/// Produce a message to a Kafka topic
pub(crate) async fn produce_kafka_message(
    State(state): State<ManagementState>,
    Json(request): Json<KafkaProduceRequest>,
) -> impl IntoResponse {
    if let Some(broker) = &state.kafka_broker {
        let mut topics = broker.topics.write().await;

        // Get or create the topic
        let topic_entry = topics.entry(request.topic.clone()).or_insert_with(|| {
            mockforge_kafka::topics::Topic::new(
                request.topic.clone(),
                mockforge_kafka::topics::TopicConfig::default(),
            )
        });

        // Determine partition
        let partition_id = if let Some(partition) = request.partition {
            partition
        } else {
            topic_entry.assign_partition(request.key.as_ref().map(|k| k.as_bytes()))
        };

        // Validate partition exists
        if partition_id < 0 || partition_id >= topic_entry.partitions.len() as i32 {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Invalid partition",
                    "message": format!("Partition {} does not exist (topic has {} partitions)", partition_id, topic_entry.partitions.len())
                })),
            )
                .into_response();
        }

        // Create the message
        let key_clone = request.key.clone();
        let headers_clone = request.headers.clone();
        let message = mockforge_kafka::partitions::KafkaMessage {
            offset: 0, // Will be set by partition.append
            timestamp: chrono::Utc::now().timestamp_millis(),
            key: key_clone.clone().map(|k| k.as_bytes().to_vec()),
            value: request.value.as_bytes().to_vec(),
            headers: headers_clone
                .clone()
                .unwrap_or_default()
                .into_iter()
                .map(|(k, v)| (k, v.as_bytes().to_vec()))
                .collect(),
        };

        // Produce to partition
        match topic_entry.produce(partition_id, message).await {
            Ok(offset) => {
                // Record metrics for successful message production
                if let Some(broker) = &state.kafka_broker {
                    broker.metrics().record_messages_produced(1);
                }

                // Emit message event for real-time monitoring
                #[cfg(feature = "kafka")]
                {
                    let event = MessageEvent::Kafka(KafkaMessageEvent {
                        topic: request.topic.clone(),
                        key: key_clone,
                        value: request.value.clone(),
                        partition: partition_id,
                        offset,
                        headers: headers_clone,
                        timestamp: chrono::Utc::now().to_rfc3339(),
                    });
                    let _ = state.message_events.send(event);
                }

                Json(serde_json::json!({
                    "success": true,
                    "message": format!("Message produced to topic '{}'", request.topic),
                    "topic": request.topic,
                    "partition": partition_id,
                    "offset": offset
                }))
                .into_response()
            }
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to produce message",
                    "message": e.to_string()
                })),
            )
                .into_response(),
        }
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "Kafka broker not available",
                "message": "Kafka broker is not enabled or not available."
            })),
        )
            .into_response()
    }
}

#[cfg(feature = "kafka")]
/// Request body for producing a batch of Kafka messages
#[derive(Debug, Deserialize)]
pub struct KafkaBatchProduceRequest {
    /// List of messages to produce
    pub messages: Vec<KafkaProduceRequest>,
    /// Delay between messages in milliseconds
    #[serde(default = "kafka_default_delay")]
    pub delay_ms: u64,
}

#[cfg(feature = "kafka")]
fn kafka_default_delay() -> u64 {
    100
}

#[cfg(feature = "kafka")]
/// Produce multiple messages to Kafka topics
pub(crate) async fn produce_kafka_batch(
    State(state): State<ManagementState>,
    Json(request): Json<KafkaBatchProduceRequest>,
) -> impl IntoResponse {
    if let Some(broker) = &state.kafka_broker {
        if request.messages.is_empty() {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Empty batch",
                    "message": "At least one message is required"
                })),
            )
                .into_response();
        }

        let mut results = Vec::new();

        for (index, msg_request) in request.messages.iter().enumerate() {
            let mut topics = broker.topics.write().await;

            // Get or create the topic
            let topic_entry = topics.entry(msg_request.topic.clone()).or_insert_with(|| {
                mockforge_kafka::topics::Topic::new(
                    msg_request.topic.clone(),
                    mockforge_kafka::topics::TopicConfig::default(),
                )
            });

            // Determine partition
            let partition_id = if let Some(partition) = msg_request.partition {
                partition
            } else {
                topic_entry.assign_partition(msg_request.key.as_ref().map(|k| k.as_bytes()))
            };

            // Validate partition exists
            if partition_id < 0 || partition_id >= topic_entry.partitions.len() as i32 {
                results.push(serde_json::json!({
                    "index": index,
                    "success": false,
                    "error": format!("Invalid partition {} (topic has {} partitions)", partition_id, topic_entry.partitions.len())
                }));
                continue;
            }

            // Create the message
            let message = mockforge_kafka::partitions::KafkaMessage {
                offset: 0,
                timestamp: chrono::Utc::now().timestamp_millis(),
                key: msg_request.key.clone().map(|k| k.as_bytes().to_vec()),
                value: msg_request.value.as_bytes().to_vec(),
                headers: msg_request
                    .headers
                    .clone()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|(k, v)| (k, v.as_bytes().to_vec()))
                    .collect(),
            };

            // Produce to partition
            match topic_entry.produce(partition_id, message).await {
                Ok(offset) => {
                    // Record metrics for successful message production
                    if let Some(broker) = &state.kafka_broker {
                        broker.metrics().record_messages_produced(1);
                    }

                    // Emit message event
                    let event = MessageEvent::Kafka(KafkaMessageEvent {
                        topic: msg_request.topic.clone(),
                        key: msg_request.key.clone(),
                        value: msg_request.value.clone(),
                        partition: partition_id,
                        offset,
                        headers: msg_request.headers.clone(),
                        timestamp: chrono::Utc::now().to_rfc3339(),
                    });
                    let _ = state.message_events.send(event);

                    results.push(serde_json::json!({
                        "index": index,
                        "success": true,
                        "topic": msg_request.topic,
                        "partition": partition_id,
                        "offset": offset
                    }));
                }
                Err(e) => {
                    results.push(serde_json::json!({
                        "index": index,
                        "success": false,
                        "error": e.to_string()
                    }));
                }
            }

            // Add delay between messages (except for the last one)
            if index < request.messages.len() - 1 && request.delay_ms > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(request.delay_ms)).await;
            }
        }

        let success_count =
            results.iter().filter(|r| r["success"].as_bool().unwrap_or(false)).count();

        Json(serde_json::json!({
            "success": true,
            "total": request.messages.len(),
            "succeeded": success_count,
            "failed": request.messages.len() - success_count,
            "results": results
        }))
        .into_response()
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "Kafka broker not available",
                "message": "Kafka broker is not enabled or not available."
            })),
        )
            .into_response()
    }
}

// ========== Kafka SSE Stream ==========

#[cfg(feature = "kafka")]
/// SSE stream for Kafka messages
pub(crate) async fn kafka_messages_stream(
    State(state): State<ManagementState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.message_events.subscribe();
    let topic_filter = params.get("topic").cloned();

    let stream = stream::unfold(rx, move |mut rx| {
        let topic_filter = topic_filter.clone();

        async move {
            loop {
                match rx.recv().await {
                    #[cfg(feature = "mqtt")]
                    Ok(MessageEvent::Mqtt(_)) => {
                        // Skip MQTT events in Kafka stream
                        continue;
                    }
                    Ok(MessageEvent::Kafka(event)) => {
                        // Apply topic filter if specified
                        if let Some(filter) = &topic_filter {
                            if !event.topic.contains(filter) {
                                continue;
                            }
                        }

                        let event_json = serde_json::json!({
                            "protocol": "kafka",
                            "topic": event.topic,
                            "key": event.key,
                            "value": event.value,
                            "partition": event.partition,
                            "offset": event.offset,
                            "headers": event.headers,
                            "timestamp": event.timestamp,
                        });

                        if let Ok(event_data) = serde_json::to_string(&event_json) {
                            let sse_event =
                                Event::default().event("kafka_message").data(event_data);
                            return Some((Ok(sse_event), rx));
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        return None;
                    }
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        warn!("Kafka message stream lagged, skipped {} messages", skipped);
                        continue;
                    }
                }
            }
        }
    });

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(std::time::Duration::from_secs(15))
            .text("keep-alive-text"),
    )
}
