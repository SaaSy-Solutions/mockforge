//! MQTT broker management and topic operations

use clap::Subcommand;
use rumqttc::{AsyncClient, MqttOptions, QoS};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Subcommand)]
pub(crate) enum MqttCommands {
    /// Publish message to MQTT topic
    Publish {
        /// MQTT broker host
        #[arg(long, default_value = "localhost")]
        host: String,

        /// MQTT broker port
        #[arg(long, default_value = "1883")]
        port: u16,

        /// Topic to publish to
        #[arg(short, long)]
        topic: String,

        /// Message payload (JSON string)
        #[arg(short, long)]
        payload: String,

        /// QoS level (0, 1, 2)
        #[arg(short, long, default_value = "0")]
        qos: u8,

        /// Retain message
        #[arg(long)]
        retain: bool,
    },

    /// Subscribe to MQTT topic
    Subscribe {
        /// MQTT broker host
        #[arg(long, default_value = "localhost")]
        host: String,

        /// MQTT broker port
        #[arg(long, default_value = "1883")]
        port: u16,

        /// Topic filter to subscribe to
        #[arg(short, long)]
        topic: String,

        /// QoS level (0, 1, 2)
        #[arg(short, long, default_value = "0")]
        qos: u8,
    },

    /// Topic management commands
    Topics {
        #[command(subcommand)]
        topics_command: MqttTopicsCommands,
    },

    /// Fixture management commands
    Fixtures {
        #[command(subcommand)]
        fixtures_command: MqttFixturesCommands,
    },

    /// Client management commands
    Clients {
        #[command(subcommand)]
        clients_command: MqttClientsCommands,
    },
}

#[derive(Subcommand)]
pub(crate) enum MqttTopicsCommands {
    /// List active topics
    List,

    /// Clear retained messages
    ClearRetained,
}

#[derive(Subcommand)]
pub(crate) enum MqttFixturesCommands {
    /// Load fixtures from directory
    Load {
        /// Path to fixtures directory
        path: PathBuf,
    },

    /// Start auto-publish for all fixtures
    StartAutoPublish,

    /// Stop auto-publish for all fixtures
    StopAutoPublish,
}

#[derive(Subcommand)]
pub(crate) enum MqttClientsCommands {
    /// List connected clients
    List,

    /// Disconnect client
    Disconnect {
        /// Client ID to disconnect
        client_id: String,
    },
}

/// Handle MQTT commands
pub async fn handle_mqtt_command(
    mqtt_command: MqttCommands,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match mqtt_command {
        MqttCommands::Publish {
            host,
            port,
            topic,
            payload,
            qos,
            retain,
        } => {
            handle_publish_command(host, port, topic, payload, qos, retain).await?;
        }
        MqttCommands::Subscribe {
            host,
            port,
            topic,
            qos,
        } => {
            handle_subscribe_command(host, port, topic, qos).await?;
        }
        MqttCommands::Topics { topics_command } => {
            handle_topics_command(topics_command).await?;
        }
        MqttCommands::Fixtures { fixtures_command } => {
            handle_fixtures_command(fixtures_command).await?;
        }
        MqttCommands::Clients { clients_command } => {
            handle_clients_command(clients_command).await?;
        }
    }
    Ok(())
}

/// Handle publish command
async fn handle_publish_command(
    host: String,
    port: u16,
    topic: String,
    payload: String,
    qos: u8,
    retain: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Connecting to MQTT broker at {}:{}...", host, port);

    // Create MQTT client options
    let mut mqtt_options = MqttOptions::new("mockforge-cli", host, port);
    mqtt_options.set_keep_alive(Duration::from_secs(5));

    // Create client
    let (client, _eventloop) = AsyncClient::new(mqtt_options, 10);

    // Convert QoS level
    let qos_level = match qos {
        0 => QoS::AtMostOnce,
        1 => QoS::AtLeastOnce,
        2 => QoS::ExactlyOnce,
        _ => return Err("Invalid QoS level. Must be 0, 1, or 2".into()),
    };

    println!("📤 Publishing to topic '{}' with QoS {}...", topic, qos);

    // Publish message
    client.publish(topic.clone(), qos_level, retain, payload.clone()).await?;

    println!("✅ Published to topic '{}': {}", topic, payload);
    println!("   QoS: {}, Retain: {}", qos, retain);

    // Wait a bit for the publish to complete
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Disconnect gracefully
    println!("🔌 Disconnecting...");
    client.disconnect().await?;
    println!("✅ Disconnected successfully");

    Ok(())
}

/// Handle subscribe command
async fn handle_subscribe_command(
    host: String,
    port: u16,
    topic: String,
    qos: u8,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Connecting to MQTT broker at {}:{}...", host, port);
    println!("Subscribing to topic '{}' with QoS {}", topic, qos);

    // Create MQTT client options
    let mut mqtt_options = MqttOptions::new("mockforge-cli-subscriber", host, port);
    mqtt_options.set_keep_alive(Duration::from_secs(5));

    // Create client
    let (client, mut eventloop) = AsyncClient::new(mqtt_options, 10);

    // Convert QoS level
    let qos_level = match qos {
        0 => QoS::AtMostOnce,
        1 => QoS::AtLeastOnce,
        2 => QoS::ExactlyOnce,
        _ => return Err("Invalid QoS level. Must be 0, 1, or 2".into()),
    };

    // Subscribe to topic
    client.subscribe(topic.clone(), qos_level).await?;
    println!("✅ Subscribed to topic '{}'", topic);
    println!("Listening for messages... (Press Ctrl+C to stop)");

    // Listen for messages
    loop {
        match eventloop.poll().await {
            Ok(notification) => {
                match notification {
                    rumqttc::Event::Incoming(rumqttc::Packet::Publish(publish)) => {
                        let payload = String::from_utf8_lossy(&publish.payload);
                        println!("📨 [{}] {}", publish.topic, payload);
                    }
                    rumqttc::Event::Incoming(rumqttc::Packet::SubAck(_)) => {
                        println!("✅ Subscription acknowledged");
                    }
                    rumqttc::Event::Incoming(rumqttc::Packet::ConnAck(_)) => {
                        println!("✅ Connected to broker");
                    }
                    _ => {} // Ignore other events
                }
            }
            Err(e) => {
                eprintln!("Connection error: {}", e);
                break;
            }
        }
    }

    Ok(())
}

/// Handle topics command
async fn handle_topics_command(
    topics_command: MqttTopicsCommands,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match topics_command {
        MqttTopicsCommands::List => {
            handle_topics_list().await?;
        }
        MqttTopicsCommands::ClearRetained => {
            handle_topics_clear_retained().await?;
        }
    }
    Ok(())
}

/// List active MQTT topics
async fn handle_topics_list() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("📋 Listing active MQTT topics...");

    // Connect to MockForge management API
    let client = reqwest::Client::new();
    let management_url = std::env::var("MOCKFORGE_MANAGEMENT_URL")
        .unwrap_or_else(|_| "http://localhost:8080/__mockforge/api".to_string());

    match client.get(format!("{}/mqtt/topics", management_url)).send().await {
        Ok(response) => {
            if response.status().is_success() {
                let topics: Vec<String> = response.json().await?;
                if topics.is_empty() {
                    println!("📭 No active MQTT topics found");
                } else {
                    println!("📬 Found {} active topics:", topics.len());
                    println!("{:<50} Type", "Topic");
                    println!("{}", "-".repeat(70));

                    for topic in topics {
                        println!("{:<50} subscription/retained", topic);
                    }
                }
            } else if response.status() == reqwest::StatusCode::NOT_FOUND {
                println!("❌ MQTT broker management API not available");
                println!("   Make sure MockForge server is running with MQTT support");
            } else {
                println!("❌ Failed to list topics: HTTP {}", response.status());
            }
        }
        Err(e) => {
            println!("❌ Failed to connect to management API: {}", e);
            println!("   Make sure MockForge server is running on {}", management_url);
        }
    }

    Ok(())
}

/// Clear retained messages
async fn handle_topics_clear_retained() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🧹 Clearing retained MQTT messages...");

    // Connect to MockForge management API
    let client = reqwest::Client::new();
    let management_url = std::env::var("MOCKFORGE_MANAGEMENT_URL")
        .unwrap_or_else(|_| "http://localhost:8080/__mockforge/api".to_string());

    match client.delete(format!("{}/mqtt/topics/retained", management_url)).send().await {
        Ok(response) => {
            if response.status().is_success() {
                println!("✅ Retained messages cleared successfully");
            } else if response.status() == reqwest::StatusCode::NOT_FOUND {
                println!("❌ MQTT broker management API not available");
                println!("   Make sure MockForge server is running with MQTT support");
            } else {
                println!("❌ Failed to clear retained messages: HTTP {}", response.status());
            }
        }
        Err(e) => {
            println!("❌ Failed to connect to management API: {}", e);
            println!("   Make sure MockForge server is running on {}", management_url);
        }
    }

    Ok(())
}

/// Handle fixtures command
async fn handle_fixtures_command(
    fixtures_command: MqttFixturesCommands,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match fixtures_command {
        MqttFixturesCommands::Load { path } => {
            handle_fixtures_load(path).await?;
        }
        MqttFixturesCommands::StartAutoPublish => {
            handle_fixtures_start_auto_publish().await?;
        }
        MqttFixturesCommands::StopAutoPublish => {
            handle_fixtures_stop_auto_publish().await?;
        }
    }
    Ok(())
}

/// Load MQTT fixtures from directory
async fn handle_fixtures_load(
    path: std::path::PathBuf,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("📁 Loading MQTT fixtures from: {}", path.display());

    // Check if path exists
    if !path.exists() {
        return Err(format!("Fixtures path does not exist: {}", path.display()).into());
    }

    // Load fixtures from directory
    let mut fixtures = Vec::new();
    let mut loaded_count = 0;

    // Read all .json and .yaml files from the directory
    for entry in std::fs::read_dir(&path)? {
        let entry = entry?;
        let file_path = entry.path();

        if file_path.is_file() {
            if let Some(extension) = file_path.extension() {
                if extension == "json" || extension == "yaml" || extension == "yml" {
                    match std::fs::read_to_string(&file_path) {
                        Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
                            Ok(fixture) => {
                                fixtures.push(fixture);
                                loaded_count += 1;
                                println!("  ✓ Loaded fixture from {}", file_path.display());
                            }
                            Err(e) => {
                                eprintln!(
                                    "  ⚠️  Failed to parse fixture {}: {}",
                                    file_path.display(),
                                    e
                                );
                            }
                        },
                        Err(e) => {
                            eprintln!(
                                "  ⚠️  Failed to read fixture {}: {}",
                                file_path.display(),
                                e
                            );
                        }
                    }
                }
            }
        }
    }

    if fixtures.is_empty() {
        println!("⚠️  No valid fixtures found in {}", path.display());
        return Ok(());
    }

    // Connect to MockForge management API
    let client = reqwest::Client::new();
    let management_url = std::env::var("MOCKFORGE_MANAGEMENT_URL")
        .unwrap_or_else(|_| "http://localhost:8080/__mockforge/api".to_string());

    match client
        .post(format!("{}/mqtt/fixtures", management_url))
        .json(&fixtures)
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
                println!(
                    "✅ Successfully loaded {} MQTT fixtures into broker registry",
                    loaded_count
                );
            } else if response.status() == reqwest::StatusCode::NOT_FOUND {
                println!("❌ MQTT broker management API not available");
                println!("   Make sure MockForge server is running with MQTT support");
            } else {
                println!("❌ Failed to load fixtures: HTTP {}", response.status());
            }
        }
        Err(e) => {
            println!("❌ Failed to connect to management API: {}", e);
            println!("   Make sure MockForge server is running on {}", management_url);
        }
    }

    Ok(())
}

/// Start auto-publish for all fixtures
async fn handle_fixtures_start_auto_publish() -> Result<(), Box<dyn std::error::Error + Send + Sync>>
{
    println!("▶️  Starting auto-publish for all MQTT fixtures...");

    // Connect to MockForge management API
    let client = reqwest::Client::new();
    let management_url = std::env::var("MOCKFORGE_MANAGEMENT_URL")
        .unwrap_or_else(|_| "http://localhost:8080/__mockforge/api".to_string());

    match client
        .post(format!("{}/mqtt/fixtures/auto-publish/start", management_url))
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
                println!("✅ Auto-publish started for all MQTT fixtures");
            } else if response.status() == reqwest::StatusCode::NOT_FOUND {
                println!("❌ MQTT broker management API not available");
                println!("   Make sure MockForge server is running with MQTT support");
            } else {
                println!("❌ Failed to start auto-publish: HTTP {}", response.status());
            }
        }
        Err(e) => {
            println!("❌ Failed to connect to management API: {}", e);
            println!("   Make sure MockForge server is running on {}", management_url);
        }
    }

    Ok(())
}

/// Stop auto-publish for all fixtures
async fn handle_fixtures_stop_auto_publish() -> Result<(), Box<dyn std::error::Error + Send + Sync>>
{
    println!("⏹️  Stopping auto-publish for all MQTT fixtures...");

    // Connect to MockForge management API
    let client = reqwest::Client::new();
    let management_url = std::env::var("MOCKFORGE_MANAGEMENT_URL")
        .unwrap_or_else(|_| "http://localhost:8080/__mockforge/api".to_string());

    match client
        .post(format!("{}/mqtt/fixtures/auto-publish/stop", management_url))
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
                println!("✅ Auto-publish stopped for all MQTT fixtures");
            } else if response.status() == reqwest::StatusCode::NOT_FOUND {
                println!("❌ MQTT broker management API not available");
                println!("   Make sure MockForge server is running with MQTT support");
            } else {
                println!("❌ Failed to stop auto-publish: HTTP {}", response.status());
            }
        }
        Err(e) => {
            println!("❌ Failed to connect to management API: {}", e);
            println!("   Make sure MockForge server is running on {}", management_url);
        }
    }

    Ok(())
}

/// Handle clients command
async fn handle_clients_command(
    clients_command: MqttClientsCommands,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match clients_command {
        MqttClientsCommands::List => {
            handle_clients_list().await?;
        }
        MqttClientsCommands::Disconnect { client_id } => {
            handle_clients_disconnect(client_id).await?;
        }
    }
    Ok(())
}

/// List connected MQTT clients
async fn handle_clients_list() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("👥 Listing connected MQTT clients...");

    // Connect to MockForge management API
    let client = reqwest::Client::new();
    let management_url = std::env::var("MOCKFORGE_MANAGEMENT_URL")
        .unwrap_or_else(|_| "http://localhost:8080/__mockforge/api".to_string());

    match client.get(format!("{}/mqtt/clients", management_url)).send().await {
        Ok(response) => {
            if response.status().is_success() {
                let clients: Vec<serde_json::Value> = response.json().await?;
                if clients.is_empty() {
                    println!("📭 No connected MQTT clients");
                } else {
                    println!("📬 Found {} connected clients:", clients.len());
                    println!("{:<30} {:<20} Subscriptions", "Client ID", "Connected At");
                    println!("{}", "-".repeat(80));

                    for client_info in clients {
                        let client_id = client_info["client_id"].as_str().unwrap_or("N/A");
                        let connected_at = client_info["connected_at"].as_str().unwrap_or("N/A");
                        let subscriptions = client_info["subscriptions"]
                            .as_array()
                            .map(|subs| subs.len().to_string())
                            .unwrap_or_else(|| "N/A".to_string());

                        println!("{:<30} {:<20} {}", client_id, connected_at, subscriptions);
                    }
                }
            } else if response.status() == reqwest::StatusCode::NOT_FOUND {
                println!("❌ MQTT broker management API not available");
                println!("   Make sure MockForge server is running with MQTT support");
            } else {
                println!("❌ Failed to list clients: HTTP {}", response.status());
            }
        }
        Err(e) => {
            println!("❌ Failed to connect to management API: {}", e);
            println!("   Make sure MockForge server is running on {}", management_url);
        }
    }

    Ok(())
}

/// Disconnect a specific MQTT client
async fn handle_clients_disconnect(
    client_id: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🔌 Disconnecting MQTT client: {}", client_id);

    // Connect to MockForge management API
    let client = reqwest::Client::new();
    let management_url = std::env::var("MOCKFORGE_MANAGEMENT_URL")
        .unwrap_or_else(|_| "http://localhost:8080/__mockforge/api".to_string());

    match client
        .delete(format!("{}/mqtt/clients/{}", management_url, urlencoding::encode(&client_id)))
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
                println!("✅ Client '{}' disconnected successfully", client_id);
            } else if response.status() == reqwest::StatusCode::NOT_FOUND {
                println!(
                    "❌ Client '{}' not found or MQTT broker management API not available",
                    client_id
                );
                println!("   Make sure MockForge server is running with MQTT support");
            } else {
                println!("❌ Failed to disconnect client: HTTP {}", response.status());
            }
        }
        Err(e) => {
            println!("❌ Failed to connect to management API: {}", e);
            println!("   Make sure MockForge server is running on {}", management_url);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mqtt_publish_command_construction() {
        let cmd = MqttCommands::Publish {
            host: "localhost".to_string(),
            port: 1883,
            topic: "test/topic".to_string(),
            payload: r#"{"key":"value"}"#.to_string(),
            qos: 0,
            retain: false,
        };
        assert!(matches!(cmd, MqttCommands::Publish { .. }));
    }

    #[test]
    fn test_mqtt_subscribe_command_construction() {
        let cmd = MqttCommands::Subscribe {
            host: "localhost".to_string(),
            port: 1883,
            topic: "test/#".to_string(),
            qos: 1,
        };
        assert!(matches!(cmd, MqttCommands::Subscribe { .. }));
    }

    #[test]
    fn test_mqtt_topics_command_construction() {
        let cmd = MqttCommands::Topics {
            topics_command: MqttTopicsCommands::List,
        };
        assert!(matches!(cmd, MqttCommands::Topics { .. }));

        let cmd_clear = MqttCommands::Topics {
            topics_command: MqttTopicsCommands::ClearRetained,
        };
        assert!(matches!(cmd_clear, MqttCommands::Topics { .. }));
    }

    #[test]
    fn test_mqtt_fixtures_command_construction() {
        let cmd = MqttCommands::Fixtures {
            fixtures_command: MqttFixturesCommands::Load {
                path: std::path::PathBuf::from("/tmp/fixtures"),
            },
        };
        assert!(matches!(cmd, MqttCommands::Fixtures { .. }));

        let cmd_start = MqttCommands::Fixtures {
            fixtures_command: MqttFixturesCommands::StartAutoPublish,
        };
        assert!(matches!(cmd_start, MqttCommands::Fixtures { .. }));
    }

    #[test]
    fn test_mqtt_clients_command_construction() {
        let cmd = MqttCommands::Clients {
            clients_command: MqttClientsCommands::List,
        };
        assert!(matches!(cmd, MqttCommands::Clients { .. }));

        let cmd_disconnect = MqttCommands::Clients {
            clients_command: MqttClientsCommands::Disconnect {
                client_id: "test-client".to_string(),
            },
        };
        assert!(matches!(cmd_disconnect, MqttCommands::Clients { .. }));
    }
}
