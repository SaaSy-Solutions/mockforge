//! MQTT server implementation using tokio TCP listener
//!
//! Note: This is a minimal implementation that listens on the MQTT port.
//! The actual MQTT broker logic is in the `MqttBroker` struct, which is
//! used via the management API. This server primarily serves to bind
//! the port and handle basic connections for compatibility.

use crate::broker::MqttConfig;
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use tracing::{error, info, warn};

/// Start an MQTT server using tokio TCP listener
///
/// This implementation binds to the MQTT port and accepts connections.
/// For a full MQTT broker implementation, consider using the management
/// API endpoints which integrate with the `MqttBroker` struct.
pub async fn start_mqtt_server(
    config: MqttConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = format!("{}:{}", config.host, config.port);

    info!(
        "📡 Starting MQTT broker on {}:{} (MQTT {:?})",
        config.host, config.port, config.version
    );

    let listener = TcpListener::bind(&addr).await?;

    info!(
        "✅ MQTT broker listening on {}:{} (MQTT {:?})",
        config.host, config.port, config.version
    );

    // Accept connections in a loop
    loop {
        match listener.accept().await {
            Ok((mut socket, addr)) => {
                info!("New MQTT connection from {}", addr);

                // Spawn a task to handle the connection
                // For now, we just close it immediately since the actual
                // MQTT handling is done via the management API and MqttBroker
                tokio::spawn(async move {
                    // Read a small buffer to detect MQTT protocol
                    let mut buf = [0u8; 1024];
                    match socket.read(&mut buf).await {
                        Ok(n) if n > 0 => {
                            // Basic MQTT protocol detection (MQTT packet starts with control byte)
                            // For a mock server, we can just acknowledge and close
                            // Real MQTT handling would parse the protocol here
                            info!("Received {} bytes from MQTT client {}", n, addr);
                        }
                        Ok(_) => {}
                        Err(e) => {
                            warn!("Error reading from MQTT client {}: {}", addr, e);
                        }
                    }

                    // Close the connection
                    // In a full implementation, this would parse MQTT packets
                    // and handle the protocol properly
                    drop(socket);
                    info!("Closed MQTT connection from {}", addr);
                });
            }
            Err(e) => {
                error!("Error accepting MQTT connection: {}", e);
                // Continue accepting connections
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::broker::MqttVersion;

    #[test]
    fn test_mqtt_config_address_formatting() {
        let config = MqttConfig {
            host: "127.0.0.1".to_string(),
            port: 1883,
            ..Default::default()
        };
        let addr = format!("{}:{}", config.host, config.port);
        assert_eq!(addr, "127.0.0.1:1883");
    }

    #[test]
    fn test_mqtt_config_default_host_port() {
        let config = MqttConfig::default();
        let addr = format!("{}:{}", config.host, config.port);
        assert_eq!(addr, "0.0.0.0:1883");
    }

    #[test]
    fn test_mqtt_config_custom_port() {
        let config = MqttConfig {
            port: 8883,
            ..Default::default()
        };
        assert_eq!(config.port, 8883);
    }

    #[test]
    fn test_mqtt_config_version_v3() {
        let config = MqttConfig {
            version: MqttVersion::V3_1_1,
            ..Default::default()
        };
        assert!(matches!(config.version, MqttVersion::V3_1_1));
    }

    #[test]
    fn test_mqtt_config_version_v5() {
        let config = MqttConfig {
            version: MqttVersion::V5_0,
            ..Default::default()
        };
        assert!(matches!(config.version, MqttVersion::V5_0));
    }

    #[tokio::test]
    async fn test_tcp_listener_bind_localhost() {
        let config = MqttConfig {
            host: "127.0.0.1".to_string(),
            port: 0, // Use port 0 to get a random available port
            ..Default::default()
        };
        let addr = format!("{}:{}", config.host, config.port);

        // Test that we can bind to the address
        let listener = TcpListener::bind(&addr).await;
        assert!(listener.is_ok());
    }

    #[tokio::test]
    async fn test_tcp_listener_local_addr() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        assert_eq!(addr.ip().to_string(), "127.0.0.1");
        assert!(addr.port() > 0);
    }

    #[test]
    fn test_mqtt_version_debug_format() {
        let v3 = MqttVersion::V3_1_1;
        let v5 = MqttVersion::V5_0;
        assert!(format!("{:?}", v3).contains("V3_1_1"));
        assert!(format!("{:?}", v5).contains("V5_0"));
    }

    #[test]
    fn test_config_max_connections() {
        let config = MqttConfig {
            max_connections: 500,
            ..Default::default()
        };
        assert_eq!(config.max_connections, 500);
    }

    #[test]
    fn test_config_max_packet_size() {
        let config = MqttConfig {
            max_packet_size: 2048,
            ..Default::default()
        };
        assert_eq!(config.max_packet_size, 2048);
    }

    #[test]
    fn test_config_keep_alive_secs() {
        let config = MqttConfig {
            keep_alive_secs: 120,
            ..Default::default()
        };
        assert_eq!(config.keep_alive_secs, 120);
    }

    #[test]
    fn test_config_clone() {
        let config1 = MqttConfig {
            port: 9999,
            host: "localhost".to_string(),
            max_connections: 200,
            max_packet_size: 4096,
            keep_alive_secs: 90,
            version: MqttVersion::V3_1_1,
        };
        let config2 = config1.clone();
        assert_eq!(config1.port, config2.port);
        assert_eq!(config1.host, config2.host);
        assert_eq!(config1.max_connections, config2.max_connections);
    }

    #[test]
    fn test_config_debug_format() {
        let config = MqttConfig::default();
        let debug = format!("{:?}", config);
        assert!(debug.contains("MqttConfig"));
        assert!(debug.contains("1883"));
    }
}
