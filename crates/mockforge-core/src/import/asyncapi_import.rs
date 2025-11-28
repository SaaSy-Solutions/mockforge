//! AsyncAPI specification import functionality
//!
//! This module handles parsing AsyncAPI 2.x/3.x specifications and converting them
//! to MockForge WebSocket, MQTT, Kafka, and AMQP configurations.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Import result for AsyncAPI specs
#[derive(Debug)]
pub struct AsyncApiImportResult {
    /// Converted channels from the AsyncAPI spec
    pub channels: Vec<MockForgeChannel>,
    /// Warnings encountered during import
    pub warnings: Vec<String>,
    /// Extracted specification metadata
    pub spec_info: AsyncApiSpecInfo,
}

/// MockForge channel structure for AsyncAPI import
#[derive(Debug, Serialize)]
pub struct MockForgeChannel {
    /// Protocol used by this channel
    pub protocol: ChannelProtocol,
    /// Channel name
    pub name: String,
    /// Channel path/endpoint
    pub path: String,
    /// Optional channel description
    pub description: Option<String>,
    /// Operations available on this channel
    pub operations: Vec<ChannelOperation>,
}

/// Channel protocol type
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ChannelProtocol {
    /// WebSocket protocol
    Websocket,
    /// MQTT protocol
    Mqtt,
    /// Kafka protocol
    Kafka,
    /// AMQP protocol
    Amqp,
}

/// Channel operation (subscribe/publish)
#[derive(Debug, Serialize)]
pub struct ChannelOperation {
    /// Type of operation (subscribe or publish)
    pub operation_type: OperationType,
    /// JSON schema for messages
    pub message_schema: Option<Value>,
    /// Example message payload
    pub example_message: Option<Value>,
}

/// Operation type for channels
#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OperationType {
    /// Subscribe to messages
    Subscribe,
    /// Publish messages
    Publish,
}

/// AsyncAPI specification metadata
#[derive(Debug)]
pub struct AsyncApiSpecInfo {
    /// Specification title
    pub title: String,
    /// Specification version
    pub version: String,
    /// Optional specification description
    pub description: Option<String>,
    /// AsyncAPI version used
    pub asyncapi_version: String,
    /// List of server URLs
    pub servers: Vec<String>,
}

/// AsyncAPI 2.x/3.x specification structure (simplified)
#[derive(Debug, Deserialize)]
struct AsyncApiSpec {
    asyncapi: String,
    info: AsyncApiInfo,
    servers: Option<HashMap<String, AsyncApiServer>>,
    channels: Option<HashMap<String, AsyncApiChannel>>,
}

#[derive(Debug, Deserialize)]
struct AsyncApiInfo {
    title: String,
    version: String,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AsyncApiServer {
    url: String,
    protocol: String,
    #[serde(rename = "protocolVersion")]
    #[allow(dead_code)]
    protocol_version: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AsyncApiChannel {
    description: Option<String>,
    subscribe: Option<AsyncApiOperation>,
    publish: Option<AsyncApiOperation>,
}

#[derive(Debug, Deserialize)]
struct AsyncApiOperation {
    message: Option<AsyncApiMessage>,
}

#[derive(Debug, Deserialize)]
struct AsyncApiMessage {
    payload: Option<Value>,
    examples: Option<Vec<Value>>,
}

/// Import an AsyncAPI specification
pub fn import_asyncapi_spec(
    content: &str,
    _base_url: Option<&str>,
) -> Result<AsyncApiImportResult, String> {
    // Try parsing as JSON first, then YAML
    let spec: AsyncApiSpec = serde_json::from_str(content)
        .or_else(|_| {
            serde_yaml::from_str(content)
                .map_err(|e| format!("Failed to parse AsyncAPI spec: {}", e))
        })
        .map_err(|e| format!("Failed to parse AsyncAPI spec: {}", e))?;

    // Validate AsyncAPI version
    if !spec.asyncapi.starts_with("2.") && !spec.asyncapi.starts_with("3.") {
        return Err(format!(
            "Unsupported AsyncAPI version: {}. Only 2.x and 3.x are supported.",
            spec.asyncapi
        ));
    }

    // Extract spec info
    let servers = spec
        .servers
        .as_ref()
        .map(|s| {
            s.values()
                .map(|server| format!("{}://{}", server.protocol, server.url))
                .collect()
        })
        .unwrap_or_default();

    let spec_info = AsyncApiSpecInfo {
        title: spec.info.title.clone(),
        version: spec.info.version.clone(),
        description: spec.info.description.clone(),
        asyncapi_version: spec.asyncapi.clone(),
        servers,
    };

    let mut channels = Vec::new();
    let mut warnings = Vec::new();

    // Process channels
    if let Some(channel_map) = spec.channels {
        for (channel_name, channel_spec) in channel_map {
            match convert_channel_to_mockforge(&channel_name, &channel_spec, &spec.servers) {
                Ok(channel) => channels.push(channel),
                Err(e) => {
                    warnings.push(format!("Failed to convert channel '{}': {}", channel_name, e))
                }
            }
        }
    }

    Ok(AsyncApiImportResult {
        channels,
        warnings,
        spec_info,
    })
}

/// Convert AsyncAPI channel to MockForge channel
fn convert_channel_to_mockforge(
    channel_name: &str,
    channel_spec: &AsyncApiChannel,
    servers: &Option<HashMap<String, AsyncApiServer>>,
) -> Result<MockForgeChannel, String> {
    // Determine protocol from first server or default to WebSocket
    let protocol = servers
        .as_ref()
        .and_then(|s| s.values().next())
        .map(|server| match server.protocol.to_lowercase().as_str() {
            "ws" | "wss" | "websocket" => ChannelProtocol::Websocket,
            "mqtt" | "mqtts" => ChannelProtocol::Mqtt,
            "kafka" | "kafka-secure" => ChannelProtocol::Kafka,
            "amqp" | "amqps" => ChannelProtocol::Amqp,
            _ => ChannelProtocol::Websocket,
        })
        .unwrap_or(ChannelProtocol::Websocket);

    let mut operations = Vec::new();

    // Process subscribe operation
    if let Some(subscribe) = &channel_spec.subscribe {
        let message_schema = subscribe.message.as_ref().and_then(|m| m.payload.clone());

        let example_message = subscribe
            .message
            .as_ref()
            .and_then(|m| m.examples.as_ref())
            .and_then(|examples| examples.first().cloned());

        operations.push(ChannelOperation {
            operation_type: OperationType::Subscribe,
            message_schema,
            example_message,
        });
    }

    // Process publish operation
    if let Some(publish) = &channel_spec.publish {
        let message_schema = publish.message.as_ref().and_then(|m| m.payload.clone());

        let example_message = publish
            .message
            .as_ref()
            .and_then(|m| m.examples.as_ref())
            .and_then(|examples| examples.first().cloned());

        operations.push(ChannelOperation {
            operation_type: OperationType::Publish,
            message_schema,
            example_message,
        });
    }

    Ok(MockForgeChannel {
        protocol,
        name: channel_name.to_string(),
        path: format!("/{}", channel_name.trim_start_matches('/')),
        description: channel_spec.description.clone(),
        operations,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_asyncapi_2() {
        let spec = r#"
        {
            "asyncapi": "2.6.0",
            "info": {
                "title": "Test API",
                "version": "1.0.0",
                "description": "Test AsyncAPI spec"
            },
            "servers": {
                "production": {
                    "url": "localhost:1883",
                    "protocol": "mqtt"
                }
            },
            "channels": {
                "sensors/temperature": {
                    "description": "Temperature sensor data",
                    "publish": {
                        "message": {
                            "payload": {
                                "type": "object",
                                "properties": {
                                    "temperature": { "type": "number" },
                                    "unit": { "type": "string" }
                                }
                            }
                        }
                    }
                }
            }
        }
        "#;

        let result = import_asyncapi_spec(spec, None);
        assert!(result.is_ok());

        let import_result = result.unwrap();
        assert_eq!(import_result.spec_info.title, "Test API");
        assert_eq!(import_result.spec_info.version, "1.0.0");
        assert_eq!(import_result.channels.len(), 1);
    }

    #[test]
    fn test_parse_asyncapi_3() {
        let spec = r#"
        {
            "asyncapi": "3.0.0",
            "info": {
                "title": "WebSocket API",
                "version": "1.0.0"
            },
            "servers": {
                "development": {
                    "url": "ws://localhost:8080",
                    "protocol": "ws"
                }
            },
            "channels": {
                "chat/messages": {
                    "description": "Chat messages",
                    "subscribe": {
                        "message": {
                            "payload": {
                                "type": "object",
                                "properties": {
                                    "message": { "type": "string" },
                                    "sender": { "type": "string" }
                                }
                            }
                        }
                    }
                }
            }
        }
        "#;

        let result = import_asyncapi_spec(spec, None);
        assert!(result.is_ok());

        let import_result = result.unwrap();
        assert_eq!(import_result.spec_info.title, "WebSocket API");
        assert_eq!(import_result.channels.len(), 1);
    }
}
