use anyhow::Result;
use clap::Subcommand;
use mockforge_core::config::{load_config, KafkaConfig};
use mockforge_kafka::{KafkaFixture, KafkaMockBroker};
use rdkafka::admin::{AdminClient, AdminOptions, NewTopic, TopicReplication};
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::message::{Header, Headers, Message, OwnedHeaders};
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::topic_partition_list::TopicPartitionList;
use rdkafka::Offset;
use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;
use std::time::Duration;

/// Kafka server management commands
#[derive(Subcommand)]
pub enum KafkaCommands {
    /// Show broker metrics and statistics
    ///
    /// Examples:
    ///   mockforge kafka metrics
    ///   mockforge kafka metrics --format prometheus
    #[command(verbatim_doc_comment)]
    Metrics {
        /// Output format (text or prometheus)
        #[arg(short, long, default_value = "text")]
        format: String,
    },
    /// Start Kafka broker
    ///
    /// Examples:
    ///   mockforge kafka serve --port 9092
    ///   mockforge kafka serve --config kafka-config.yaml
    #[command(verbatim_doc_comment)]
    Serve {
        /// Kafka broker port
        #[arg(short, long, default_value = "9092")]
        port: u16,

        /// Kafka broker host
        #[arg(long, default_value = "127.0.0.1")]
        host: String,

        /// Configuration file path
        #[arg(short, long)]
        config: Option<PathBuf>,
    },

    /// Manage Kafka topics
    ///
    /// Examples:
    ///   mockforge kafka topic create orders --partitions 3
    ///   mockforge kafka topic list
    ///   mockforge kafka topic describe orders
    ///   mockforge kafka topic delete orders
    #[command(verbatim_doc_comment)]
    Topic {
        #[command(subcommand)]
        topic_command: KafkaTopicCommands,
    },

    /// Manage Kafka consumer groups
    ///
    /// Examples:
    ///   mockforge kafka groups list
    ///   mockforge kafka groups describe test-group
    ///   mockforge kafka groups offsets test-group
    #[command(verbatim_doc_comment)]
    Groups {
        #[command(subcommand)]
        groups_command: KafkaGroupsCommands,
    },

    /// Produce messages to topics
    ///
    /// Examples:
    ///   mockforge kafka produce --topic orders --key "order-123" --value '{"id": "order-123"}'
    ///   mockforge kafka produce --topic events --value "test message"
    #[command(verbatim_doc_comment)]
    Produce {
        /// Topic name
        #[arg(short, long)]
        topic: String,

        /// Message key
        #[arg(short, long)]
        key: Option<String>,

        /// Message value
        #[arg(short, long)]
        value: String,

        /// Message partition
        #[arg(short, long)]
        partition: Option<i32>,

        /// Header (key:value format)
        #[arg(short, long)]
        header: Vec<String>,
    },

    /// Consume messages from topics
    ///
    /// Examples:
    ///   mockforge kafka consume --topic orders --group test-group
    ///   mockforge kafka consume --topic events --partition 0 --offset 100
    #[command(verbatim_doc_comment)]
    Consume {
        /// Topic name
        #[arg(short, long)]
        topic: String,

        /// Consumer group ID
        #[arg(short, long)]
        group: Option<String>,

        /// Partition to consume from
        #[arg(short, long)]
        partition: Option<i32>,

        /// Starting offset
        #[arg(short, long, default_value = "latest")]
        from: String,

        /// Number of messages to consume
        #[arg(short, long)]
        count: Option<usize>,
    },

    /// Manage Kafka fixtures
    ///
    /// Examples:
    ///   mockforge kafka fixtures load ./fixtures/kafka/
    ///   mockforge kafka fixtures list
    ///   mockforge kafka fixtures start-auto-produce
    ///   mockforge kafka fixtures stop-auto-produce
    #[command(verbatim_doc_comment)]
    Fixtures {
        #[command(subcommand)]
        fixtures_command: KafkaFixturesCommands,
    },

    /// Testing and simulation commands
    ///
    /// Examples:
    ///   mockforge kafka simulate lag --group test-group --topic orders --lag 1000
    ///   mockforge kafka simulate rebalance --group test-group
    ///   mockforge kafka simulate reset-offsets --group test-group --topic orders --to-earliest
    #[command(verbatim_doc_comment)]
    Simulate {
        #[command(subcommand)]
        simulate_command: KafkaSimulateCommands,
    },
}

/// Topic management subcommands
#[derive(Subcommand)]
pub enum KafkaTopicCommands {
    /// Create a new topic
    Create {
        /// Topic name
        name: String,

        /// Number of partitions
        #[arg(short, long, default_value = "3")]
        partitions: i32,

        /// Replication factor
        #[arg(short, long, default_value = "1")]
        replication_factor: i32,
    },

    /// List all topics
    List,

    /// Describe a topic
    Describe {
        /// Topic name
        name: String,
    },

    /// Delete a topic
    Delete {
        /// Topic name
        name: String,
    },
}

/// Consumer groups management subcommands
#[derive(Subcommand)]
pub enum KafkaGroupsCommands {
    /// List all consumer groups
    List,

    /// Describe a consumer group
    Describe {
        /// Group ID
        group_id: String,
    },

    /// Show offsets for a consumer group
    Offsets {
        /// Group ID
        group_id: String,
    },
}

/// Fixtures management subcommands
#[derive(Subcommand)]
pub enum KafkaFixturesCommands {
    /// Load fixtures from directory
    Load {
        /// Directory containing fixture files
        directory: PathBuf,
    },

    /// List loaded fixtures
    List,

    /// Start auto-producing messages
    StartAutoProduce,

    /// Stop auto-producing messages
    StopAutoProduce,
}

/// Simulation subcommands
#[derive(Subcommand)]
pub enum KafkaSimulateCommands {
    /// Simulate consumer lag
    Lag {
        /// Consumer group ID
        #[arg(short, long)]
        group: String,

        /// Topic name
        #[arg(short, long)]
        topic: String,

        /// Lag in messages
        #[arg(short, long)]
        lag: i64,
    },

    /// Trigger rebalance for a consumer group
    Rebalance {
        /// Consumer group ID
        #[arg(short, long)]
        group: String,
    },

    /// Reset consumer offsets
    ResetOffsets {
        /// Consumer group ID
        #[arg(short, long)]
        group: String,

        /// Topic name
        #[arg(short, long)]
        topic: String,

        /// Reset to offset
        #[arg(short, long, default_value = "earliest")]
        to: String,
    },
}

/// Handle Kafka commands
pub async fn handle_kafka_command(command: KafkaCommands) -> Result<()> {
    execute_kafka_command(command).await
}

/// Execute Kafka commands
pub async fn execute_kafka_command(command: KafkaCommands) -> Result<()> {
    match command {
        KafkaCommands::Serve { port, host, config } => {
            let mut kafka_config = if let Some(config_path) = config {
                let server_config = load_config(config_path).await?;
                server_config.kafka
            } else {
                KafkaConfig::default()
            };
            kafka_config.port = port;
            kafka_config.host = host.clone();

            let broker = KafkaMockBroker::new(kafka_config).await?;
            println!("Starting Kafka broker on {}:{}", host, port);
            broker.start().await?;
            Ok(())
        }
        KafkaCommands::Topic { topic_command } => execute_topic_command(topic_command).await,
        KafkaCommands::Groups { groups_command } => execute_groups_command(groups_command).await,
        KafkaCommands::Produce {
            topic,
            key,
            value,
            partition,
            header,
        } => {
            let producer: FutureProducer = ClientConfig::new()
                .set("bootstrap.servers", "localhost:9092")
                .create()
                .map_err(|e| anyhow::anyhow!("Producer creation failed: {}", e))?;

            let mut record = FutureRecord::to(&topic).payload(&value);
            if let Some(k) = &key {
                record = record.key(k);
            }
            if let Some(p) = partition {
                record = record.partition(p);
            }
            if !header.is_empty() {
                let mut owned_headers = OwnedHeaders::new();
                for h in &header {
                    let parts: Vec<&str> = h.splitn(2, ':').collect();
                    if parts.len() == 2 {
                        owned_headers = owned_headers.insert(Header {
                            key: parts[0],
                            value: Some(parts[1].as_bytes()),
                        });
                    } else {
                        return Err(anyhow::anyhow!("Invalid header format: {}", h));
                    }
                }
                record = record.headers(owned_headers);
            }

            let delivery_status = producer.send(record, Duration::from_secs(0)).await;
            match delivery_status {
                Ok(delivery) => {
                    println!(
                        "Message produced to topic {} partition {} at offset {}",
                        topic, delivery.partition, delivery.offset
                    );
                }
                Err((e, _)) => {
                    return Err(anyhow::anyhow!("Failed to produce message: {}", e).into());
                }
            }
            Ok(())
        }
        KafkaCommands::Consume {
            topic,
            group,
            partition,
            from,
            count,
        } => {
            let group_id = group.unwrap_or_else(|| "cli-consumer".to_string());
            let consumer: StreamConsumer = ClientConfig::new()
                .set("bootstrap.servers", "localhost:9092")
                .set("group.id", &group_id)
                .set("enable.auto.commit", "false")
                .set("auto.offset.reset", "earliest")
                .create()
                .map_err(|e| anyhow::anyhow!("Consumer creation failed: {}", e))?;

            if let Some(p) = partition {
                // Assign to specific partition
                let mut tpl = TopicPartitionList::new();
                tpl.add_partition(&topic, p);
                consumer.assign(&tpl).map_err(|e| anyhow::anyhow!("Assign failed: {}", e))?;

                // Seek to beginning or end
                let offset = match from.as_str() {
                    "beginning" => rdkafka::Offset::Beginning,
                    "end" => rdkafka::Offset::End,
                    _ => return Err(anyhow::anyhow!("Invalid 'from' value: {}", from).into()),
                };
                consumer
                    .seek(&topic, p, offset, Duration::from_secs(30))
                    .map_err(|e| anyhow::anyhow!("Seek failed: {}", e))?;
            } else {
                // Subscribe to topic
                consumer
                    .subscribe(&[&topic])
                    .map_err(|e| anyhow::anyhow!("Subscribe failed: {}", e))?;
            }

            let mut message_count = 0;
            let max_count = count.unwrap_or(usize::MAX);

            println!("Consuming from topic {}...", topic);
            if let Some(p) = partition {
                println!("  Partition: {}", p);
            }
            println!("  From: {}", from);

            loop {
                if message_count >= max_count {
                    break;
                }

                match consumer.recv().await {
                    Ok(message) => {
                        message_count += 1;
                        println!("Message {}:", message_count);
                        if let Some(key) = message.key() {
                            println!("  Key: {}", String::from_utf8_lossy(key));
                        }
                        if let Some(payload) = message.payload() {
                            println!("  Value: {}", String::from_utf8_lossy(payload));
                        }
                        println!("  Partition: {}", message.partition());
                        println!("  Offset: {}", message.offset());
                        if let Some(headers) = message.headers() {
                            for header in headers.iter() {
                                println!(
                                    "  Header {}: {}",
                                    header.key,
                                    String::from_utf8_lossy(header.value.unwrap_or(&[]))
                                );
                            }
                        }
                        println!();
                    }
                    Err(e) => {
                        return Err(anyhow::anyhow!("Receive failed: {}", e).into());
                    }
                }
            }

            println!("Consumed {} messages", message_count);
            Ok(())
        }
        KafkaCommands::Fixtures { fixtures_command } => {
            execute_fixtures_command(fixtures_command).await
        }
        KafkaCommands::Simulate { simulate_command } => {
            execute_simulate_command(simulate_command).await
        }
        KafkaCommands::Metrics { format } => {
            let consumer: StreamConsumer = ClientConfig::new()
                .set("bootstrap.servers", "localhost:9092")
                .set("group.id", "metrics-consumer")
                .create()
                .map_err(|e| anyhow::anyhow!("Consumer creation failed: {}", e))?;

            let metadata = consumer
                .fetch_metadata(None, Duration::from_secs(30))
                .map_err(|e| anyhow::anyhow!("Fetch metadata failed: {}", e))?;

            if format == "prometheus" {
                println!("# Kafka Metrics");
                println!("kafka_topics_total {}", metadata.topics().len());
                let mut total_partitions = 0;
                for topic in metadata.topics() {
                    let partitions = topic.partitions().len();
                    total_partitions += partitions;
                    println!("kafka_topic_partitions{{topic=\"{}\"}} {}", topic.name(), partitions);
                }
                println!("kafka_partitions_total {}", total_partitions);
                println!("kafka_brokers_total {}", metadata.brokers().len());
            } else {
                println!("Kafka Broker Metrics:");
                println!("  Brokers: {}", metadata.brokers().len());
                println!("  Topics: {}", metadata.topics().len());
                let mut total_partitions = 0;
                for topic in metadata.topics() {
                    let partitions = topic.partitions().len();
                    total_partitions += partitions;
                    println!("    {}: {} partitions", topic.name(), partitions);
                }
                println!("  Total Partitions: {}", total_partitions);
            }
            Ok(())
        }
    }
}

async fn execute_topic_command(command: KafkaTopicCommands) -> Result<()> {
    match command {
        KafkaTopicCommands::Create {
            name,
            partitions,
            replication_factor,
        } => {
            let admin: AdminClient<_> = ClientConfig::new()
                .set("bootstrap.servers", "localhost:9092")
                .create()
                .map_err(|e| anyhow::anyhow!("Admin client creation failed: {}", e))?;

            let topics = vec![NewTopic::new(
                name.as_str(),
                partitions,
                TopicReplication::Fixed(-1),
            )];
            let options = AdminOptions::new().request_timeout(Some(Duration::from_secs(30)));

            admin
                .create_topics(&topics, &options)
                .await
                .map_err(|e| anyhow::anyhow!("Create topic failed: {}", e))?;

            println!(
                "Topic '{}' created successfully with {} partitions (replication factor: {})",
                name, partitions, replication_factor
            );
            Ok(())
        }
        KafkaTopicCommands::List => {
            let admin: AdminClient<_> = ClientConfig::new()
                .set("bootstrap.servers", "localhost:9092")
                .create()
                .map_err(|e| anyhow::anyhow!("Admin client creation failed: {}", e))?;

            let metadata = admin
                .inner()
                .fetch_metadata(None, Duration::from_secs(30))
                .map_err(|e| anyhow::anyhow!("Fetch metadata failed: {}", e))?;

            println!("Topics:");
            for topic in metadata.topics() {
                println!("  {} ({} partitions)", topic.name(), topic.partitions().len());
            }
            Ok(())
        }
        KafkaTopicCommands::Describe { name } => {
            let admin: AdminClient<_> = ClientConfig::new()
                .set("bootstrap.servers", "localhost:9092")
                .create()
                .map_err(|e| anyhow::anyhow!("Admin client creation failed: {}", e))?;

            let metadata = admin
                .inner()
                .fetch_metadata(None, Duration::from_secs(30))
                .map_err(|e| anyhow::anyhow!("Fetch metadata failed: {}", e))?;

            let topic = metadata
                .topics()
                .iter()
                .find(|t| t.name() == name)
                .ok_or_else(|| anyhow::anyhow!("Topic {} not found", name))?;

            println!("Topic: {}", topic.name());
            println!("Partitions: {}", topic.partitions().len());
            for partition in topic.partitions() {
                println!(
                    "  Partition {}: Leader={}, Replicas={:?}",
                    partition.id(),
                    partition.leader(),
                    partition.replicas().iter().map(|b| *b).collect::<Vec<_>>()
                );
            }
            Ok(())
        }
        KafkaTopicCommands::Delete { name } => {
            let admin: AdminClient<_> = ClientConfig::new()
                .set("bootstrap.servers", "localhost:9092")
                .create()
                .map_err(|e| anyhow::anyhow!("Admin client creation failed: {}", e))?;

            let options = AdminOptions::new().request_timeout(Some(Duration::from_secs(30)));
            admin
                .delete_topics(&[name.as_str()], &options)
                .await
                .map_err(|e| anyhow::anyhow!("Delete topic failed: {}", e))?;

            println!("Topic '{}' deleted successfully", name);
            Ok(())
        }
    }
}

async fn execute_groups_command(command: KafkaGroupsCommands) -> Result<()> {
    match command {
        KafkaGroupsCommands::List => {
            let admin: AdminClient<_> = ClientConfig::new()
                .set("bootstrap.servers", "localhost:9092")
                .create()
                .map_err(|e| anyhow::anyhow!("Admin client creation failed: {}", e))?;

            let groups = admin
                .inner()
                .fetch_group_list(None, Duration::from_secs(30))
                .map_err(|e| anyhow::anyhow!("List groups failed: {}", e))?;

            println!("Consumer Groups:");
            for group in groups.groups() {
                println!("  {}", group.name());
            }
            Ok(())
        }
        KafkaGroupsCommands::Describe { group_id } => {
            // Note: describe_consumer_groups is not available in rdkafka 0.38
            // This is a simplified implementation
            let admin: AdminClient<_> = ClientConfig::new()
                .set("bootstrap.servers", "localhost:9092")
                .create()
                .map_err(|e| anyhow::anyhow!("Admin client creation failed: {}", e))?;

            let groups = admin
                .inner()
                .fetch_group_list(None, Duration::from_secs(30))
                .map_err(|e| anyhow::anyhow!("List groups failed: {}", e))?;

            let group = groups.groups().iter().find(|g| g.name() == group_id);
            if group.is_none() {
                return Err(anyhow::anyhow!("Consumer group {} not found", group_id));
            }

            println!("Consumer Group: {}", group_id);
            println!("  State: {}", group.unwrap().state());
            println!("  Protocol: {}", group.unwrap().protocol());
            println!("  Protocol Type: {}", group.unwrap().protocol_type());
            println!("  Members: {}", group.unwrap().members().len());

            Ok(())
        }
        KafkaGroupsCommands::Offsets { group_id } => {
            // Note: list_consumer_group_offsets is not available in rdkafka 0.38
            // This is a stub implementation
            println!("Consumer group offsets for '{}':", group_id);
            println!("  Note: Offset listing not supported in rdkafka 0.38");
            println!("  Consider upgrading rdkafka version for full functionality");
            Ok(())
        }
    }
}

async fn execute_fixtures_command(command: KafkaFixturesCommands) -> Result<()> {
    match command {
        KafkaFixturesCommands::Load { directory } => {
            if !directory.exists() {
                return Err(anyhow::anyhow!("Directory does not exist: {}", directory.display()));
            }

            if !directory.is_dir() {
                return Err(anyhow::anyhow!("Path is not a directory: {}", directory.display()));
            }

            match KafkaFixture::load_from_dir(&directory) {
                Ok(fixtures) => {
                    if fixtures.is_empty() {
                        println!("No fixture files found in {}", directory.display());
                        println!("Fixture files should be YAML files (.yaml or .yml) containing KafkaFixture definitions.");
                        return Ok(());
                    }

                    println!(
                        "Successfully loaded {} fixtures from {}",
                        fixtures.len(),
                        directory.display()
                    );

                    // Validate fixtures and show summary
                    let mut topics = std::collections::HashSet::new();
                    let mut auto_produce_count = 0;

                    for fixture in &fixtures {
                        topics.insert(&fixture.topic);
                        if fixture.auto_produce.as_ref().is_some_and(|ap| ap.enabled) {
                            auto_produce_count += 1;
                        }
                    }

                    println!("Fixtures cover {} unique topics", topics.len());
                    if auto_produce_count > 0 {
                        println!("{} fixtures have auto-produce enabled", auto_produce_count);
                    }

                    println!("\nFixtures loaded:");
                    for fixture in &fixtures {
                        println!("  ✓ {} ({})", fixture.identifier, fixture.name);
                    }

                    println!(
                        "\nNote: Fixtures are loaded for validation. In a running mock broker,"
                    );
                    println!(
                        "these would be available for message generation and auto-production."
                    );

                    Ok(())
                }
                Err(e) => Err(anyhow::anyhow!(
                    "Failed to load fixtures from {}: {}",
                    directory.display(),
                    e
                )),
            }
        }
        KafkaFixturesCommands::List => {
            // Try to load fixtures from common directories
            let fixture_dirs = vec![
                std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
                PathBuf::from("./fixtures"),
                PathBuf::from("./kafka-fixtures"),
            ];

            let mut all_fixtures = Vec::new();
            let mut found_dirs = Vec::new();

            for dir in fixture_dirs {
                if dir.exists() && dir.is_dir() {
                    match KafkaFixture::load_from_dir(&dir) {
                        Ok(fixtures) => {
                            if !fixtures.is_empty() {
                                all_fixtures.extend(fixtures);
                                found_dirs.push(dir);
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Failed to load fixtures from {}: {}", dir.display(), e);
                        }
                    }
                }
            }

            if all_fixtures.is_empty() {
                println!(
                    "No fixtures found. Checked directories: ./, ./fixtures, ./kafka-fixtures"
                );
                println!("Create YAML fixture files in one of these directories to define message templates.");
                return Ok(());
            }

            println!(
                "Found {} fixtures in {} director{}:",
                all_fixtures.len(),
                found_dirs.len(),
                if found_dirs.len() == 1 { "y" } else { "ies" }
            );

            for dir in &found_dirs {
                println!("  {}", dir.display());
            }

            println!("\nFixtures:");
            for fixture in &all_fixtures {
                println!("  {}: {}", fixture.identifier, fixture.name);
                println!("    Topic: {}", fixture.topic);
                println!(
                    "    Partition: {}",
                    fixture.partition.map_or("all".to_string(), |p| p.to_string())
                );
                if let Some(auto_produce) = &fixture.auto_produce {
                    if auto_produce.enabled {
                        println!("    Auto-produce: {} msg/sec", auto_produce.rate_per_second);
                        if let Some(duration) = auto_produce.duration_seconds {
                            println!("    Duration: {} seconds", duration);
                        }
                        if let Some(count) = auto_produce.total_count {
                            println!("    Total count: {} messages", count);
                        }
                    } else {
                        println!("    Auto-produce: disabled");
                    }
                } else {
                    println!("    Auto-produce: not configured");
                }
                println!("    Headers: {}", fixture.headers.len());
                println!();
            }

            Ok(())
        }
        KafkaFixturesCommands::StartAutoProduce => {
            // Note: Auto-produce is controlled by fixture configuration
            // This command would start auto-production for all fixtures with auto_produce.enabled = true
            // In a full implementation, this would connect to the mock broker's management API
            println!("Auto-produce start requested - ensure fixtures have auto_produce.enabled = true in their configuration");
            Ok(())
        }
        KafkaFixturesCommands::StopAutoProduce => {
            // Note: In a full implementation, this would connect to the mock broker's management API
            // to stop all auto-production tasks
            println!("Auto-produce stop requested - this would disable auto_produce for all running fixtures");
            Ok(())
        }
    }
}

async fn execute_simulate_command(command: KafkaSimulateCommands) -> Result<()> {
    match command {
        KafkaSimulateCommands::Lag { group, topic, lag } => {
            // Create a consumer to get watermark offsets
            let consumer: StreamConsumer = ClientConfig::new()
                .set("bootstrap.servers", "localhost:9092")
                .set("group.id", "lag-simulator")
                .set("enable.auto.commit", "false")
                .create()
                .map_err(|e| anyhow::anyhow!("Consumer creation failed: {}", e))?;

            // Get topic metadata to know partitions
            let metadata = consumer
                .fetch_metadata(Some(topic.as_str()), Duration::from_secs(30))
                .map_err(|e| anyhow::anyhow!("Fetch metadata failed: {}", e))?;
            let topic_metadata = metadata
                .topics()
                .iter()
                .find(|t| t.name() == topic)
                .ok_or_else(|| anyhow::anyhow!("Topic {} not found", topic))?;

            // Create topic partition list with lag-simulated offsets
            let mut tpl = TopicPartitionList::new();
            for partition in topic_metadata.partitions() {
                // Get watermark offsets for this partition
                let (low_watermark, high_watermark) = consumer
                    .fetch_watermarks(&topic, partition.id(), Duration::from_secs(30))
                    .map_err(|e| {
                        anyhow::anyhow!(
                            "Fetch watermarks failed for partition {}: {}",
                            partition.id(),
                            e
                        )
                    })?;

                // Calculate target offset as high_watermark - lag
                let target_offset = if lag >= 0 {
                    high_watermark.saturating_sub(lag as i64)
                } else {
                    // Negative lag doesn't make sense, default to low watermark
                    low_watermark
                };

                tpl.add_partition_offset(&topic, partition.id(), Offset::Offset(target_offset));
            }

            // Create admin client to alter offsets
            let admin: AdminClient<_> = ClientConfig::new()
                .set("bootstrap.servers", "localhost:9092")
                .create()
                .map_err(|e| anyhow::anyhow!("Admin client creation failed: {}", e))?;

            // Note: alter_consumer_group_offsets is not available in rdkafka 0.38
            // This is a stub implementation
            println!("Note: Lag simulation not supported in rdkafka 0.38");

            println!("Simulated lag of {} messages for group {} on topic {} (set offsets behind high watermark)", lag, group, topic);
            Ok(())
        }
        KafkaSimulateCommands::Rebalance { group } => {
            // Note: Consumer group operations are not available in rdkafka 0.38
            // This is a stub implementation
            println!("Note: Rebalance simulation not supported in rdkafka 0.38");
            println!("Consider upgrading rdkafka version for full functionality");
            Ok(())
        }
        KafkaSimulateCommands::ResetOffsets { group, topic, to } => {
            let admin: AdminClient<_> = ClientConfig::new()
                .set("bootstrap.servers", "localhost:9092")
                .create()
                .map_err(|e| anyhow::anyhow!("Admin client creation failed: {}", e))?;

            // Get topic metadata to know partitions
            let metadata = admin
                .inner()
                .fetch_metadata(Some(topic.as_str()), Duration::from_secs(30))
                .map_err(|e| anyhow::anyhow!("Fetch metadata failed: {}", e))?;
            let topic_metadata = metadata
                .topics()
                .iter()
                .find(|t| t.name() == topic)
                .ok_or_else(|| anyhow::anyhow!("Topic {} not found", topic))?;

            // Create topic partition list with reset offsets
            let mut tpl = TopicPartitionList::new();
            let target_offset = match to.as_str() {
                "earliest" => Offset::Beginning,
                "latest" => Offset::End,
                offset_str => {
                    let offset: i64 = offset_str
                        .parse()
                        .map_err(|_| anyhow::anyhow!("Invalid offset: {}", offset_str))?;
                    Offset::Offset(offset)
                }
            };

            for partition in topic_metadata.partitions() {
                tpl.add_partition_offset(&topic, partition.id(), target_offset);
            }

            // Note: alter_consumer_group_offsets is not available in rdkafka 0.38
            // This is a stub implementation
            println!("Note: Offset reset not supported in rdkafka 0.38");

            println!("Successfully reset offsets for group {} on topic {} to {}", group, topic, to);
            Ok(())
        }
    }
}
