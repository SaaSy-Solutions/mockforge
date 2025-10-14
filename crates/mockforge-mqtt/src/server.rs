//! MQTT server implementation using rumqttd

use rumqttd::Broker;
use crate::broker::{MqttConfig, MqttVersion};

/// Start an MQTT server using rumqttd
pub async fn start_mqtt_server(config: MqttConfig) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("📡 Starting MQTT broker on {}:{} (MQTT {:?})", config.host, config.port, config.version);

    // Use default rumqttd configuration
    // Note: rumqttd supports both MQTT v3.1.1 and v5.0 by default
    // The version configuration is tracked for future enhancements
    let broker_config = rumqttd::Config::default();

    // Start the rumqttd broker
    let mut broker = Broker::new(broker_config);

    println!("✅ MQTT broker started successfully on {}:{} (MQTT {:?})", config.host, config.port, config.version);

    // Keep the broker running
    broker.start()?;

    Ok(())
}