//! Unified protocol server implementation for the MQTT mock broker.

use async_trait::async_trait;
use mockforge_core::protocol_abstraction::Protocol;
use mockforge_core::protocol_server::MockProtocolServer;

use crate::broker::MqttConfig;

/// A `MockProtocolServer` wrapper around the MQTT server startup.
///
/// Wraps [`crate::start_mqtt_server`] with shutdown-signal integration.
pub struct MqttMockServer {
    config: MqttConfig,
}

impl MqttMockServer {
    /// Create a new `MqttMockServer` with the given configuration.
    pub fn new(config: MqttConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl MockProtocolServer for MqttMockServer {
    fn protocol(&self) -> Protocol {
        Protocol::Mqtt
    }

    async fn start(
        &self,
        mut shutdown: tokio::sync::watch::Receiver<()>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let config = self.config.clone();

        tokio::select! {
            result = crate::start_mqtt_server(config) => {
                result
            }
            _ = shutdown.changed() => {
                tracing::info!("Shutting down MQTT broker on port {}", self.config.port);
                Ok(())
            }
        }
    }

    fn port(&self) -> u16 {
        self.config.port
    }

    fn description(&self) -> String {
        format!("MQTT broker on {}:{}", self.config.host, self.config.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mqtt_mock_server_new() {
        let config = MqttConfig::default();
        let server = MqttMockServer::new(config);
        assert_eq!(server.port(), 1883);
    }

    #[test]
    fn test_mqtt_mock_server_protocol() {
        let server = MqttMockServer::new(MqttConfig::default());
        assert_eq!(server.protocol(), Protocol::Mqtt);
    }

    #[test]
    fn test_mqtt_mock_server_description() {
        let config = MqttConfig {
            host: "127.0.0.1".to_string(),
            port: 1883,
            ..Default::default()
        };
        let server = MqttMockServer::new(config);
        assert_eq!(server.description(), "MQTT broker on 127.0.0.1:1883");
    }
}
