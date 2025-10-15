use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_yaml;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};

/// Kafka fixture for message generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaFixture {
    pub identifier: String,
    pub name: String,
    pub topic: String,
    pub partition: Option<i32>,      // None = all partitions
    pub key_pattern: Option<String>, // Template
    pub value_template: serde_json::Value,
    pub headers: std::collections::HashMap<String, String>,
    pub auto_produce: Option<AutoProduceConfig>,
}

/// Configuration for auto-producing messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoProduceConfig {
    pub enabled: bool,
    pub rate_per_second: u64,
    pub duration_seconds: Option<u64>,
    pub total_count: Option<usize>,
}

/// Auto-producer for fixtures
pub struct AutoProducer {
    fixtures: Arc<RwLock<HashMap<String, KafkaFixture>>>,
    template_engine: mockforge_core::templating::TemplateEngine,
    broker: Arc<super::broker::KafkaMockBroker>,
}

impl AutoProducer {
    /// Create a new auto-producer
    pub fn new(
        broker: Arc<super::broker::KafkaMockBroker>,
        template_engine: mockforge_core::templating::TemplateEngine,
    ) -> Self {
        Self {
            fixtures: Arc::new(RwLock::new(HashMap::new())),
            template_engine,
            broker,
        }
    }

    /// Add a fixture for auto-production
    pub async fn add_fixture(&self, fixture: KafkaFixture) {
        if fixture.auto_produce.as_ref().is_some_and(|ap| ap.enabled) {
            let fixture_id = fixture.identifier.clone();
            self.fixtures.write().await.insert(fixture_id, fixture);
        }
    }

    /// Start auto-producing messages
    pub async fn start(&self) -> anyhow::Result<()> {
        let fixtures = self.fixtures.clone();
        let _template_engine = self.template_engine.clone();
        let _broker = self.broker.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(1));

            loop {
                interval.tick().await;

                let fixtures_read = fixtures.read().await.clone();
                for fixture in fixtures_read.values() {
                    if let Some(auto_produce) = &fixture.auto_produce {
                        if auto_produce.enabled {
                            // Generate and produce messages
                            for _ in 0..auto_produce.rate_per_second {
                                if let Ok(message) = fixture.generate_message(&HashMap::new()) {
                                    // Produce the message to the broker
                                    let mut topics = _broker.topics.write().await;
                                    if let Some(topic) = topics.get_mut(&fixture.topic) {
                                        let partition = fixture.partition.unwrap_or_else(|| {
                                            topic.assign_partition(message.key.as_deref())
                                        });

                                        if let Err(e) = topic.produce(partition, message).await {
                                            tracing::error!(
                                                "Failed to produce message to topic {}: {}",
                                                fixture.topic,
                                                e
                                            );
                                        } else {
                                            tracing::debug!(
                                                "Auto-produced message to topic {} partition {}",
                                                fixture.topic,
                                                partition
                                            );
                                        }
                                    } else {
                                        tracing::warn!(
                                            "Topic {} not found for auto-production",
                                            fixture.topic
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// Stop auto-producing for a specific fixture
    pub async fn stop_fixture(&self, fixture_id: &str) {
        if let Some(fixture) = self.fixtures.write().await.get_mut(fixture_id) {
            if let Some(auto_produce) = &mut fixture.auto_produce {
                auto_produce.enabled = false;
            }
        }
    }
}

impl KafkaFixture {
    /// Load fixtures from a directory
    pub fn load_from_dir(dir: &PathBuf) -> mockforge_core::Result<Vec<Self>> {
        let mut fixtures = Vec::new();
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("yaml")
                || path.extension().and_then(|s| s.to_str()) == Some("yml")
            {
                let file = fs::File::open(&path)?;
                let file_fixtures: Vec<Self> = serde_yaml::from_reader(file)?;
                fixtures.extend(file_fixtures);
            }
        }
        Ok(fixtures)
    }

    /// Generate a message using the fixture
    pub fn generate_message(
        &self,
        context: &std::collections::HashMap<String, String>,
    ) -> mockforge_core::Result<crate::partitions::KafkaMessage> {
        // Render key if pattern provided
        let key = self.key_pattern.as_ref().map(|pattern| self.render_template(pattern, context));

        // Render value template
        let value_str = serde_json::to_string(&self.value_template)?;
        let value_rendered = self.render_template(&value_str, context);
        let value = value_rendered.into_bytes();

        // Render headers
        let headers = self
            .headers
            .iter()
            .map(|(k, v)| (k.clone(), self.render_template(v, context).into_bytes()))
            .collect();

        Ok(crate::partitions::KafkaMessage {
            offset: 0,
            timestamp: Utc::now().timestamp_millis(),
            key: key.map(|k| k.into_bytes()),
            value,
            headers,
        })
    }

    fn render_template(
        &self,
        template: &str,
        context: &std::collections::HashMap<String, String>,
    ) -> String {
        let mut result = template.to_string();
        for (key, value) in context {
            result = result.replace(&format!("{{{{{}}}}}", key), value);
        }
        result
    }
}
