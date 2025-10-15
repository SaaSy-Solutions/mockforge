use serde::{Deserialize, Serialize};
use crate::exchanges::ExchangeType;

/// Configuration for an exchange in fixtures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeConfig {
    pub name: String,
    #[serde(rename = "type")]
    pub exchange_type: ExchangeType,
    pub durable: bool,
}

/// Configuration for a queue in fixtures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueConfig {
    pub name: String,
    pub durable: bool,
    pub message_template: Option<serde_json::Value>,
}

/// Configuration for a binding in fixtures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindingConfig {
    pub exchange: String,
    pub queue: String,
    pub routing_key: String,
}

/// Configuration for auto-publish
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoPublishConfig {
    pub enabled: bool,
    pub exchange: String,
    pub routing_key: String,
    pub rate_per_second: u64,
    pub message_template: serde_json::Value,
}

/// AMQP fixture configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmqpFixture {
    pub identifier: String,
    pub name: String,
    pub exchanges: Vec<ExchangeConfig>,
    pub queues: Vec<QueueConfig>,
    pub bindings: Vec<BindingConfig>,
    pub auto_publish: Option<AutoPublishConfig>,
}

impl AmqpFixture {
    /// Load fixtures from a directory
    pub fn load_from_dir(dir: &std::path::PathBuf) -> mockforge_core::Result<Vec<Self>> {
        let mut fixtures = Vec::new();

        if !dir.exists() {
            return Ok(fixtures);
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("yaml") ||
               path.extension().and_then(|s| s.to_str()) == Some("yml") {
                match Self::load_from_file(&path) {
                    Ok(fixture) => fixtures.push(fixture),
                    Err(e) => {
                        tracing::warn!("Failed to load fixture from {:?}: {}", path, e);
                    }
                }
            }
        }

        Ok(fixtures)
    }

    /// Load a single fixture from a YAML file
    fn load_from_file(path: &std::path::Path) -> mockforge_core::Result<Self> {
        println!("Loading fixture from: {:?}", path);
        let content = std::fs::read_to_string(path)?;
        println!("File content length: {}", content.len());
        let fixture: AmqpFixture = serde_yaml::from_str(&content)
            .map_err(|e| {
                println!("YAML parsing error: {}", e);
                e
            })?;
        println!("Successfully loaded fixture: {}", fixture.identifier);
        Ok(fixture)
    }
}
