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
