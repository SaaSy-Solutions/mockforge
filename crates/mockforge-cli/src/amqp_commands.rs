use anyhow::Result;
use clap::Subcommand;
use futures_lite::stream::StreamExt;
use lapin::{options::*, types::FieldTable, Connection, ConnectionProperties, BasicProperties};
use mockforge_amqp::{AmqpBroker, AmqpSpecRegistry};
use mockforge_core::config::{load_config, AmqpConfig};
use std::path::PathBuf;
use std::sync::Arc;

/// AMQP server management commands
#[derive(Subcommand)]
pub enum AmqpCommands {
    /// Start AMQP broker
    ///
    /// Examples:
    ///   mockforge amqp serve --port 5672
    ///   mockforge amqp serve --config amqp-config.yaml
    #[command(verbatim_doc_comment)]
    Serve {
        /// AMQP broker port
        #[arg(short, long, default_value = "5672")]
        port: u16,

        /// AMQP broker host
        #[arg(long, default_value = "127.0.0.1")]
        host: String,

        /// Configuration file path
        #[arg(short, long)]
        config: Option<PathBuf>,
    },

    /// Manage AMQP exchanges
    ///
    /// Examples:
    ///   mockforge amqp exchange declare orders --type topic --durable
    ///   mockforge amqp exchange list
    ///   mockforge amqp exchange delete orders
    #[command(verbatim_doc_comment)]
    Exchange {
        #[command(subcommand)]
        command: ExchangeCommands,
    },

    /// Manage AMQP queues
    ///
    /// Examples:
    ///   mockforge amqp queue declare orders.new --durable
    ///   mockforge amqp queue list
    ///   mockforge amqp queue purge orders.new
    ///   mockforge amqp queue delete orders.new
    #[command(verbatim_doc_comment)]
    Queue {
        #[command(subcommand)]
        command: QueueCommands,
    },

    /// Manage AMQP bindings
    ///
    /// Examples:
    ///   mockforge amqp bind orders orders.new --routing-key "order.created"
    ///   mockforge amqp unbind orders orders.new --routing-key "order.created"
    ///   mockforge amqp list-bindings
    #[command(verbatim_doc_comment)]
    Bind {
        /// Exchange name
        exchange: String,
        /// Queue name
        queue: String,
        /// Routing key
        #[arg(short, long)]
        routing_key: String,
    },

    /// Unbind a queue from an exchange
    Unbind {
        /// Exchange name
        exchange: String,
        /// Queue name
        queue: String,
        /// Routing key
        #[arg(short, long)]
        routing_key: String,
    },

    /// List all bindings
    ListBindings,

    /// Publish a message
    ///
    /// Examples:
    ///   mockforge amqp publish --exchange orders --routing-key "order.created" --body '{"order_id": "123"}'
    ///   mockforge amqp publish --exchange logs --routing-key "app.error" --body "Error occurred"
    #[command(verbatim_doc_comment)]
    Publish {
        /// Exchange name
        #[arg(short, long)]
        exchange: String,
        /// Routing key
        #[arg(short = 'k', long)]
        routing_key: String,
        /// Message body
        #[arg(short, long)]
        body: String,
        /// Content type
        #[arg(long, default_value = "application/json")]
        content_type: String,
        /// Make message persistent
        #[arg(long)]
        persistent: bool,
    },

    /// Consume messages from a queue
    ///
    /// Examples:
    ///   mockforge amqp consume --queue orders.new
    ///   mockforge amqp consume --queue logs --auto-ack
    #[command(verbatim_doc_comment)]
    Consume {
        /// Queue name
        #[arg(short, long)]
        queue: String,
        /// Auto acknowledge messages
        #[arg(long)]
        auto_ack: bool,
    },

    /// Get a single message from a queue
    Get {
        /// Queue name
        #[arg(short, long)]
        queue: String,
    },

    /// Manage AMQP fixtures
    ///
    /// Examples:
    ///   mockforge amqp fixtures load ./fixtures/amqp/
    ///   mockforge amqp fixtures list
    ///   mockforge amqp fixtures start-auto-publish
    ///   mockforge amqp fixtures stop-auto-publish
    #[command(verbatim_doc_comment)]
    Fixtures {
        #[command(subcommand)]
        command: FixtureCommands,
    },
}

#[derive(Subcommand)]
pub enum ExchangeCommands {
    /// Declare an exchange
    Declare {
        /// Exchange name
        name: String,
        /// Exchange type (direct, fanout, topic, headers)
        #[arg(short, long, default_value = "direct")]
        r#type: String,
        /// Make exchange durable
        #[arg(long)]
        durable: bool,
        /// Auto-delete exchange when unused
        #[arg(long)]
        auto_delete: bool,
    },
    /// List all exchanges
    List,
    /// Delete an exchange
    Delete {
        /// Exchange name
        name: String,
    },
}

#[derive(Subcommand)]
pub enum QueueCommands {
    /// Declare a queue
    Declare {
        /// Queue name
        name: String,
        /// Make queue durable
        #[arg(long)]
        durable: bool,
        /// Make queue exclusive
        #[arg(long)]
        exclusive: bool,
        /// Auto-delete queue when unused
        #[arg(long)]
        auto_delete: bool,
    },
    /// List all queues
    List,
    /// Purge a queue (delete all messages)
    Purge {
        /// Queue name
        name: String,
    },
    /// Delete a queue
    Delete {
        /// Queue name
        name: String,
    },
}

#[derive(Subcommand)]
pub enum FixtureCommands {
    /// Load fixtures from directory
    Load {
        /// Directory containing fixture files
        dir: PathBuf,
    },
    /// List loaded fixtures
    List,
    /// Start auto-publishing for fixtures
    StartAutoPublish,
    /// Stop auto-publishing for fixtures
    StopAutoPublish,
}

/// Execute AMQP commands
pub async fn execute_amqp_command(command: AmqpCommands) -> Result<()> {
    match command {
        AmqpCommands::Serve { port, host, config } => serve_amqp(port, host, config).await,
        AmqpCommands::Exchange { command } => execute_exchange_command(command).await,
        AmqpCommands::Queue { command } => execute_queue_command(command).await,
        AmqpCommands::Bind {
            exchange,
            queue,
            routing_key,
        } => bind_queue(&exchange, &queue, &routing_key).await,
        AmqpCommands::Unbind {
            exchange,
            queue,
            routing_key,
        } => unbind_queue(&exchange, &queue, &routing_key).await,
        AmqpCommands::ListBindings => list_bindings().await,
        AmqpCommands::Publish {
            exchange,
            routing_key,
            body,
            content_type,
            persistent,
        } => publish_message(&exchange, &routing_key, &body, &content_type, persistent).await,
        AmqpCommands::Consume { queue, auto_ack } => consume_messages(&queue, auto_ack).await,
        AmqpCommands::Get { queue } => get_message(&queue).await,
        AmqpCommands::Fixtures { command } => execute_fixture_command(command).await,
    }
}

async fn serve_amqp(port: u16, host: String, config: Option<PathBuf>) -> Result<()> {
    let amqp_config = if let Some(config_path) = config {
        let server_config = load_config(config_path).await?;
        let mut amqp_config = server_config.amqp;
        amqp_config.port = port;
        amqp_config.host = host;
        amqp_config
    } else {
        AmqpConfig {
            port,
            host,
            ..Default::default()
        }
    };

    let spec_registry = Arc::new(AmqpSpecRegistry::new(amqp_config.clone()).await?);
    let broker = AmqpBroker::new(amqp_config, spec_registry);

    println!("Starting AMQP broker on {}:{}", broker.config.host, broker.config.port);
    broker.start().await.map_err(|e| anyhow::anyhow!("Failed to start broker: {}", e))
}

async fn execute_exchange_command(command: ExchangeCommands) -> Result<()> {
    // Connect to the AMQP broker (assuming it's running on localhost:5672)
    let addr = "amqp://127.0.0.1:5672";
    let conn = Connection::connect(addr, ConnectionProperties::default())
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to AMQP broker at {}: {}", addr, e))?;

    let channel = conn
        .create_channel()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create channel: {}", e))?;

    match command {
        ExchangeCommands::Declare {
            name,
            r#type,
            durable,
            auto_delete,
        } => {
            let exchange_type = match r#type.as_str() {
                "direct" => lapin::ExchangeKind::Direct,
                "fanout" => lapin::ExchangeKind::Fanout,
                "topic" => lapin::ExchangeKind::Topic,
                "headers" => lapin::ExchangeKind::Headers,
                _ => return Err(anyhow::anyhow!("Invalid exchange type: {}", r#type)),
            };

            channel
                .exchange_declare(
                    &name,
                    exchange_type,
                    ExchangeDeclareOptions {
                        durable,
                        auto_delete,
                        ..Default::default()
                    },
                    FieldTable::default(),
                )
                .await
                .map_err(|e| anyhow::anyhow!("Failed to declare exchange: {}", e))?;

            println!("Exchange '{}' declared successfully", name);
        }
        ExchangeCommands::List => {
            // AMQP doesn't have a standard way to list exchanges
            // This would require a management plugin or custom implementation
            println!("Listing exchanges is not supported in this AMQP implementation");
            println!("Consider using the broker's management interface if available");
        }
        ExchangeCommands::Delete { name } => {
            channel
                .exchange_delete(&name, ExchangeDeleteOptions::default())
                .await
                .map_err(|e| anyhow::anyhow!("Failed to delete exchange: {}", e))?;

            println!("Exchange '{}' deleted successfully", name);
        }
    }

    conn.close(0, "Done").await?;
    Ok(())
}

async fn execute_queue_command(command: QueueCommands) -> Result<()> {
    // Connect to the AMQP broker (assuming it's running on localhost:5672)
    let addr = "amqp://127.0.0.1:5672";
    let conn = Connection::connect(addr, ConnectionProperties::default())
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to AMQP broker at {}: {}", addr, e))?;

    let channel = conn
        .create_channel()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create channel: {}", e))?;

    match command {
        QueueCommands::Declare {
            name,
            durable,
            exclusive,
            auto_delete,
        } => {
            channel
                .queue_declare(
                    &name,
                    QueueDeclareOptions {
                        durable,
                        exclusive,
                        auto_delete,
                        ..Default::default()
                    },
                    FieldTable::default(),
                )
                .await
                .map_err(|e| anyhow::anyhow!("Failed to declare queue: {}", e))?;

            println!("Queue '{}' declared successfully", name);
        }
        QueueCommands::List => {
            // AMQP doesn't have a standard way to list queues
            // This would require a management plugin or custom implementation
            println!("Listing queues is not supported in this AMQP implementation");
            println!("Consider using the broker's management interface if available");
        }
        QueueCommands::Purge { name } => {
            channel
                .queue_purge(&name, QueuePurgeOptions::default())
                .await
                .map_err(|e| anyhow::anyhow!("Failed to purge queue: {}", e))?;

            println!("Queue '{}' purged successfully", name);
        }
        QueueCommands::Delete { name } => {
            channel
                .queue_delete(&name, QueueDeleteOptions::default())
                .await
                .map_err(|e| anyhow::anyhow!("Failed to delete queue: {}", e))?;

            println!("Queue '{}' deleted successfully", name);
        }
    }

    conn.close(0, "Done").await?;
    Ok(())
}

async fn bind_queue(exchange: &str, queue: &str, routing_key: &str) -> Result<()> {
    // Connect to the AMQP broker (assuming it's running on localhost:5672)
    let addr = "amqp://127.0.0.1:5672";
    let conn = Connection::connect(addr, ConnectionProperties::default())
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to AMQP broker at {}: {}", addr, e))?;

    let channel = conn
        .create_channel()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create channel: {}", e))?;

    channel
        .queue_bind(
            queue,
            exchange,
            routing_key,
            QueueBindOptions::default(),
            FieldTable::default(),
        )
        .await
        .map_err(|e| {
            anyhow::anyhow!(
                "Failed to bind queue '{}' to exchange '{}' with routing key '{}': {}",
                queue,
                exchange,
                routing_key,
                e
            )
        })?;

    println!(
        "Queue '{}' bound to exchange '{}' with routing key '{}' successfully",
        queue, exchange, routing_key
    );

    conn.close(0, "Done").await?;
    Ok(())
}

async fn unbind_queue(exchange: &str, queue: &str, routing_key: &str) -> Result<()> {
    // Connect to the AMQP broker (assuming it's running on localhost:5672)
    let addr = "amqp://127.0.0.1:5672";
    let conn = Connection::connect(addr, ConnectionProperties::default())
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to AMQP broker at {}: {}", addr, e))?;

    let channel = conn
        .create_channel()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create channel: {}", e))?;

    channel
        .queue_unbind(
            queue,
            exchange,
            routing_key,
            FieldTable::default(),
        )
        .await
        .map_err(|e| {
            anyhow::anyhow!(
                "Failed to unbind queue '{}' from exchange '{}' with routing key '{}': {}",
                queue,
                exchange,
                routing_key,
                e
            )
        })?;

    println!(
        "Queue '{}' unbound from exchange '{}' with routing key '{}' successfully",
        queue, exchange, routing_key
    );

    conn.close(0, "Done").await?;
    Ok(())
}

async fn list_bindings() -> Result<()> {
    // AMQP doesn't have a standard way to list bindings
    // This would require a management plugin or custom implementation
    println!("Listing bindings is not supported in this AMQP implementation");
    println!("Consider using the broker's management interface if available");
    Ok(())
}

async fn publish_message(
    exchange: &str,
    routing_key: &str,
    body: &str,
    content_type: &str,
    persistent: bool,
) -> Result<()> {
    // Connect to the AMQP broker (assuming it's running on localhost:5672)
    let addr = "amqp://127.0.0.1:5672";
    let conn = Connection::connect(addr, ConnectionProperties::default())
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to AMQP broker at {}: {}", addr, e))?;

    let channel = conn
        .create_channel()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create channel: {}", e))?;

    let mut properties = BasicProperties::default().with_content_type(content_type.into());

    if persistent {
        properties = properties.with_delivery_mode(2); // persistent delivery mode
    }

    channel
        .basic_publish(
            exchange,
            routing_key,
            BasicPublishOptions::default(),
            body.as_bytes(),
            properties,
        )
        .await
        .map_err(|e| {
            anyhow::anyhow!(
                "Failed to publish message to exchange '{}' with routing key '{}': {}",
                exchange,
                routing_key,
                e
            )
        })?;

    println!(
        "Message published to exchange '{}' with routing key '{}' successfully",
        exchange, routing_key
    );

    conn.close(0, "Done").await?;
    Ok(())
}

async fn consume_messages(queue: &str, auto_ack: bool) -> Result<()> {
    // Connect to the AMQP broker (assuming it's running on localhost:5672)
    let addr = "amqp://127.0.0.1:5672";
    let conn = Connection::connect(addr, ConnectionProperties::default())
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to AMQP broker at {}: {}", addr, e))?;

    let channel = conn
        .create_channel()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create channel: {}", e))?;

    let consumer_tag = format!("cli-consumer-{}", std::process::id());

    let mut consumer = channel
        .basic_consume(
            queue,
            &consumer_tag,
            BasicConsumeOptions {
                no_ack: auto_ack,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await
        .map_err(|e| anyhow::anyhow!("Failed to start consuming from queue '{}': {}", queue, e))?;

    println!("Started consuming messages from queue '{}'. Press Ctrl+C to stop.", queue);

    while let Some(delivery) = consumer.next().await {
        let delivery =
            delivery.map_err(|e| anyhow::anyhow!("Failed to receive delivery: {}", e))?;

        let body = String::from_utf8_lossy(&delivery.data);
        println!("Received message: {}", body);

        if !auto_ack {
            delivery
                .ack(BasicAckOptions::default())
                .await
                .map_err(|e| anyhow::anyhow!("Failed to acknowledge message: {}", e))?;
        }
    }

    conn.close(0, "Done").await?;
    Ok(())
}

async fn get_message(queue: &str) -> Result<()> {
    // Connect to the AMQP broker (assuming it's running on localhost:5672)
    let addr = "amqp://127.0.0.1:5672";
    let conn = Connection::connect(addr, ConnectionProperties::default())
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to AMQP broker at {}: {}", addr, e))?;

    let channel = conn
        .create_channel()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create channel: {}", e))?;

    let get_result = channel
        .basic_get(queue, BasicGetOptions::default())
        .await
        .map_err(|e| anyhow::anyhow!("Failed to get message from queue '{}': {}", queue, e))?;

    if let Some(delivery) = get_result {
        let body = String::from_utf8_lossy(&delivery.delivery.data);
        println!("Retrieved message: {}", body);

        // Acknowledge the message
        delivery
            .delivery
            .ack(BasicAckOptions::default())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to acknowledge message: {}", e))?;
    } else {
        println!("No messages available in queue '{}'", queue);
    }

    conn.close(0, "Done").await?;
    Ok(())
}

async fn execute_fixture_command(command: FixtureCommands) -> Result<()> {
    match command {
        FixtureCommands::Load { dir } => {
            println!("Fixture loading is not supported at runtime. Fixtures are loaded at broker startup from the configured fixtures directory.");
            println!("To load fixtures, restart the broker with fixtures_dir set to: {:?}", dir);
        }
        FixtureCommands::List => {
            println!("Listing loaded fixtures is not supported in this AMQP implementation");
            println!("Fixtures are loaded at startup and not accessible via CLI");
        }
        FixtureCommands::StartAutoPublish => {
            println!("Starting auto-publish is not supported at runtime");
            println!("Auto-publish is configured per fixture and starts automatically if enabled");
        }
        FixtureCommands::StopAutoPublish => {
            println!("Stopping auto-publish is not supported at runtime");
            println!("Auto-publish runs based on fixture configuration");
        }
    }
    Ok(())
}
