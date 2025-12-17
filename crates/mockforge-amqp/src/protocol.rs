//! AMQP protocol handling
//!
//! This module implements the AMQP 0.9.1 protocol for handling connections,
//! channels, and method frames.

use std::collections::HashMap;
use std::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// AMQP frame types
#[derive(Debug)]
pub enum FrameType {
    Method = 1,
    Header = 2,
    Body = 3,
    Heartbeat = 8,
}

/// AMQP class IDs
pub mod class_id {
    pub const CONNECTION: u16 = 10;
    pub const CHANNEL: u16 = 20;
    pub const EXCHANGE: u16 = 40;
    pub const QUEUE: u16 = 50;
    pub const BASIC: u16 = 60;
    pub const TX: u16 = 90;
    pub const CONFIRM: u16 = 85;
}

/// AMQP method IDs
pub mod method_id {
    // Connection methods
    pub const CONNECTION_START: u16 = 10;
    pub const CONNECTION_START_OK: u16 = 11;
    pub const CONNECTION_TUNE: u16 = 30;
    pub const CONNECTION_TUNE_OK: u16 = 31;
    pub const CONNECTION_OPEN: u16 = 40;
    pub const CONNECTION_OPEN_OK: u16 = 41;
    pub const CONNECTION_CLOSE: u16 = 50;
    pub const CONNECTION_CLOSE_OK: u16 = 51;

    // Channel methods
    pub const CHANNEL_OPEN: u16 = 10;
    pub const CHANNEL_OPEN_OK: u16 = 11;
    pub const CHANNEL_CLOSE: u16 = 40;
    pub const CHANNEL_CLOSE_OK: u16 = 41;

    // Exchange methods
    pub const EXCHANGE_DECLARE: u16 = 10;
    pub const EXCHANGE_DECLARE_OK: u16 = 11;
    pub const EXCHANGE_DELETE: u16 = 20;
    pub const EXCHANGE_DELETE_OK: u16 = 21;

    // Queue methods
    pub const QUEUE_DECLARE: u16 = 10;
    pub const QUEUE_DECLARE_OK: u16 = 11;
    pub const QUEUE_BIND: u16 = 20;
    pub const QUEUE_BIND_OK: u16 = 21;
    pub const QUEUE_UNBIND: u16 = 50;
    pub const QUEUE_UNBIND_OK: u16 = 51;
    pub const QUEUE_DELETE: u16 = 40;
    pub const QUEUE_DELETE_OK: u16 = 41;

    // Basic methods
    pub const BASIC_PUBLISH: u16 = 40;
    pub const BASIC_CONSUME: u16 = 20;
    pub const BASIC_CONSUME_OK: u16 = 21;
    pub const BASIC_DELIVER: u16 = 60;
    pub const BASIC_ACK: u16 = 80;
    pub const BASIC_NACK: u16 = 120;
    pub const BASIC_QOS: u16 = 10;
    pub const BASIC_QOS_OK: u16 = 11;
    pub const BASIC_GET: u16 = 70;
    pub const BASIC_GET_OK: u16 = 71;
    pub const BASIC_GET_EMPTY: u16 = 72;

    // Transaction methods
    pub const TX_SELECT: u16 = 10;
    pub const TX_SELECT_OK: u16 = 11;
    pub const TX_COMMIT: u16 = 20;
    pub const TX_COMMIT_OK: u16 = 21;
    pub const TX_ROLLBACK: u16 = 30;
    pub const TX_ROLLBACK_OK: u16 = 31;

    // Confirm methods
    pub const CONFIRM_SELECT: u16 = 10;
    pub const CONFIRM_SELECT_OK: u16 = 11;
}

/// AMQP method frame
#[derive(Debug)]
pub struct MethodFrame {
    pub class_id: u16,
    pub method_id: u16,
    pub arguments: Vec<u8>,
}

/// Channel state
#[derive(Debug, Clone)]
pub enum ChannelState {
    Closed,
    Opening,
    Open,
    Closing,
}

/// Channel information
#[derive(Debug)]
pub struct Channel {
    pub id: u16,
    pub state: ChannelState,
    pub consumer_tag: Option<String>,
    pub prefetch_count: u16,
    pub prefetch_size: u32,
    pub publisher_confirms: bool,
    pub transaction_mode: bool,
    pub next_delivery_tag: u64,
    pub unconfirmed_messages: HashMap<u64, String>, // delivery_tag -> routing_key
}

/// AMQP frame
#[derive(Debug)]
pub struct Frame {
    pub frame_type: FrameType,
    pub channel: u16,
    pub payload: Vec<u8>,
}

impl Frame {
    /// Read a frame from the stream
    pub async fn read_from_stream(stream: &mut TcpStream) -> io::Result<Self> {
        // AMQP frame format: [frame-type (1 byte)] [channel (2 bytes)] [size (4 bytes)] [payload] [frame-end (1 byte)]
        let mut header = [0u8; 7];
        stream.read_exact(&mut header).await?;

        let frame_type = match header[0] {
            1 => FrameType::Method,
            2 => FrameType::Header,
            3 => FrameType::Body,
            8 => FrameType::Heartbeat,
            _ => return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid frame type")),
        };

        let channel = u16::from_be_bytes([header[1], header[2]]);
        let size = u32::from_be_bytes([header[3], header[4], header[5], header[6]]) as usize;

        let mut payload = vec![0u8; size];
        stream.read_exact(&mut payload).await?;

        // Read frame end marker
        let mut frame_end = [0u8; 1];
        stream.read_exact(&mut frame_end).await?;
        if frame_end[0] != 0xCE {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid frame end marker"));
        }

        Ok(Frame {
            frame_type,
            channel,
            payload,
        })
    }

    /// Write a frame to the stream
    pub async fn write_to_stream(&self, stream: &mut TcpStream) -> io::Result<()> {
        let frame_type_byte = match self.frame_type {
            FrameType::Method => 1,
            FrameType::Header => 2,
            FrameType::Body => 3,
            FrameType::Heartbeat => 8,
        };

        let size = self.payload.len() as u32;
        let size_bytes = size.to_be_bytes();

        // Write frame header
        stream.write_all(&[frame_type_byte]).await?;
        stream.write_all(&self.channel.to_be_bytes()).await?;
        stream.write_all(&size_bytes).await?;

        // Write payload
        stream.write_all(&self.payload).await?;

        // Write frame end marker
        stream.write_all(&[0xCE]).await?;

        Ok(())
    }
}

/// AMQP connection handler
pub struct ConnectionHandler {
    stream: TcpStream,
    channels: HashMap<u16, Channel>,
    next_consumer_tag: u32,
}

impl ConnectionHandler {
    pub fn new(stream: TcpStream) -> Self {
        let mut channels = HashMap::new();
        // Channel 0 is always open for connection-level communication
        channels.insert(
            0,
            Channel {
                id: 0,
                state: ChannelState::Open,
                consumer_tag: None,
                prefetch_count: 0,
                prefetch_size: 0,
                publisher_confirms: false,
                transaction_mode: false,
                next_delivery_tag: 1,
                unconfirmed_messages: HashMap::new(),
            },
        );

        Self {
            stream,
            channels,
            next_consumer_tag: 1,
        }
    }

    /// Handle the AMQP connection
    pub async fn handle(mut self) -> io::Result<()> {
        // Send protocol header
        self.send_protocol_header().await?;

        loop {
            match Frame::read_from_stream(&mut self.stream).await {
                Ok(frame) => {
                    if let Err(e) = self.handle_frame(frame).await {
                        tracing::error!("Error handling frame: {}", e);
                        break;
                    }
                }
                Err(e) => {
                    tracing::error!("Error reading frame: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    async fn send_protocol_header(&mut self) -> io::Result<()> {
        // AMQP 0.9.1 protocol header: "AMQP\x00\x00\x09\x01"
        let header = b"AMQP\x00\x00\x09\x01";
        self.stream.write_all(header).await
    }

    async fn handle_frame(&mut self, frame: Frame) -> io::Result<()> {
        match frame.frame_type {
            FrameType::Method => {
                self.handle_method_frame(frame).await?;
            }
            FrameType::Heartbeat => {
                // Respond to heartbeat
                let response = Frame {
                    frame_type: FrameType::Heartbeat,
                    channel: frame.channel,
                    payload: vec![],
                };
                response.write_to_stream(&mut self.stream).await?;
            }
            _ => {
                tracing::debug!(
                    "Received frame type {:?} on channel {}",
                    frame.frame_type,
                    frame.channel
                );
            }
        }
        Ok(())
    }

    async fn handle_method_frame(&mut self, frame: Frame) -> io::Result<()> {
        if frame.payload.len() < 4 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Method frame too short"));
        }

        let class_id = u16::from_be_bytes([frame.payload[0], frame.payload[1]]);
        let method_id = u16::from_be_bytes([frame.payload[2], frame.payload[3]]);
        let arguments = frame.payload[4..].to_vec();

        let method = MethodFrame {
            class_id,
            method_id,
            arguments,
        };

        tracing::debug!(
            "Received method: class={} method={} on channel {}",
            class_id,
            method_id,
            frame.channel
        );

        match (class_id, method_id) {
            (class_id::CONNECTION, method_id::CONNECTION_START) => {
                self.handle_connection_start(frame.channel).await
            }
            (class_id::CONNECTION, method_id::CONNECTION_START_OK) => {
                self.handle_connection_start_ok(frame.channel, &method).await
            }
            (class_id::CONNECTION, method_id::CONNECTION_TUNE_OK) => {
                self.handle_connection_tune_ok(frame.channel, &method).await
            }
            (class_id::CONNECTION, method_id::CONNECTION_OPEN) => {
                self.handle_connection_open(frame.channel, &method).await
            }
            (class_id::CONNECTION, method_id::CONNECTION_CLOSE) => {
                self.handle_connection_close(frame.channel).await
            }
            (class_id::CHANNEL, method_id::CHANNEL_OPEN) => {
                self.handle_channel_open(frame.channel).await
            }
            (class_id::CHANNEL, method_id::CHANNEL_CLOSE) => {
                self.handle_channel_close(frame.channel).await
            }
            (class_id::EXCHANGE, method_id::EXCHANGE_DECLARE) => {
                self.handle_exchange_declare(frame.channel, &method).await
            }
            (class_id::QUEUE, method_id::QUEUE_DECLARE) => {
                self.handle_queue_declare(frame.channel, &method).await
            }
            (class_id::QUEUE, method_id::QUEUE_BIND) => {
                self.handle_queue_bind(frame.channel, &method).await
            }
            (class_id::BASIC, method_id::BASIC_PUBLISH) => {
                self.handle_basic_publish(frame.channel, &method).await
            }
            (class_id::BASIC, method_id::BASIC_CONSUME) => {
                self.handle_basic_consume(frame.channel, &method).await
            }
            (class_id::BASIC, method_id::BASIC_ACK) => {
                self.handle_basic_ack(frame.channel, &method).await
            }
            (class_id::BASIC, method_id::BASIC_QOS) => {
                self.handle_basic_qos(frame.channel, &method).await
            }
            (class_id::BASIC, method_id::BASIC_GET) => {
                self.handle_basic_get(frame.channel, &method).await
            }
            (class_id::TX, method_id::TX_SELECT) => self.handle_tx_select(frame.channel).await,
            (class_id::TX, method_id::TX_COMMIT) => self.handle_tx_commit(frame.channel).await,
            (class_id::TX, method_id::TX_ROLLBACK) => self.handle_tx_rollback(frame.channel).await,
            (class_id::CONFIRM, method_id::CONFIRM_SELECT) => {
                self.handle_confirm_select(frame.channel).await
            }
            _ => {
                tracing::debug!("Unhandled method: class={} method={}", class_id, method_id);
                Ok(())
            }
        }
    }

    // Connection methods
    async fn handle_connection_start(&mut self, channel: u16) -> io::Result<()> {
        // Send Connection.Start-Ok
        let mut payload = Vec::new();
        payload.extend_from_slice(&class_id::CONNECTION.to_be_bytes());
        payload.extend_from_slice(&method_id::CONNECTION_START_OK.to_be_bytes());
        // Client properties (empty table)
        payload.push(0); // table size
                         // Mechanism: PLAIN
        let mechanism = b"PLAIN";
        payload.extend_from_slice(&(mechanism.len() as u32).to_be_bytes());
        payload.extend_from_slice(mechanism);
        // Response: empty
        payload.extend_from_slice(&(0u32).to_be_bytes());
        // Locale: en_US
        let locale = b"en_US";
        payload.extend_from_slice(&(locale.len() as u32).to_be_bytes());
        payload.extend_from_slice(locale);

        let response = Frame {
            frame_type: FrameType::Method,
            channel,
            payload,
        };
        response.write_to_stream(&mut self.stream).await?;

        // Send Connection.Tune
        let mut tune_payload = Vec::new();
        tune_payload.extend_from_slice(&class_id::CONNECTION.to_be_bytes());
        tune_payload.extend_from_slice(&method_id::CONNECTION_TUNE.to_be_bytes());
        tune_payload.extend_from_slice(&65535u16.to_be_bytes()); // channel_max
        tune_payload.extend_from_slice(&131072u32.to_be_bytes()); // frame_max
        tune_payload.extend_from_slice(&60u16.to_be_bytes()); // heartbeat

        let tune_response = Frame {
            frame_type: FrameType::Method,
            channel,
            payload: tune_payload,
        };
        tune_response.write_to_stream(&mut self.stream).await
    }

    async fn handle_connection_start_ok(
        &mut self,
        _channel: u16,
        _method: &MethodFrame,
    ) -> io::Result<()> {
        // Connection.Start-Ok received, connection is progressing
        Ok(())
    }

    async fn handle_connection_tune_ok(
        &mut self,
        _channel: u16,
        _method: &MethodFrame,
    ) -> io::Result<()> {
        // Connection.Tune-Ok received
        Ok(())
    }

    async fn handle_connection_open(
        &mut self,
        channel: u16,
        _method: &MethodFrame,
    ) -> io::Result<()> {
        // Send Connection.Open-Ok
        let mut payload = Vec::new();
        payload.extend_from_slice(&class_id::CONNECTION.to_be_bytes());
        payload.extend_from_slice(&method_id::CONNECTION_OPEN_OK.to_be_bytes());
        // Reserved field (empty string)
        payload.extend_from_slice(&(0u8 as u32).to_be_bytes());

        let response = Frame {
            frame_type: FrameType::Method,
            channel,
            payload,
        };
        response.write_to_stream(&mut self.stream).await
    }

    async fn handle_connection_close(&mut self, channel: u16) -> io::Result<()> {
        // Send Connection.Close-Ok
        let mut payload = Vec::new();
        payload.extend_from_slice(&class_id::CONNECTION.to_be_bytes());
        payload.extend_from_slice(&method_id::CONNECTION_CLOSE_OK.to_be_bytes());

        let response = Frame {
            frame_type: FrameType::Method,
            channel,
            payload,
        };
        response.write_to_stream(&mut self.stream).await
    }

    // Channel methods
    async fn handle_channel_open(&mut self, channel: u16) -> io::Result<()> {
        // Create new channel
        self.channels.insert(
            channel,
            Channel {
                id: channel,
                state: ChannelState::Open,
                consumer_tag: None,
                prefetch_count: 0,
                prefetch_size: 0,
                publisher_confirms: false,
                transaction_mode: false,
                next_delivery_tag: 1,
                unconfirmed_messages: HashMap::new(),
            },
        );

        // Send Channel.Open-Ok
        let mut payload = Vec::new();
        payload.extend_from_slice(&class_id::CHANNEL.to_be_bytes());
        payload.extend_from_slice(&method_id::CHANNEL_OPEN_OK.to_be_bytes());
        // Reserved field (empty long string)
        payload.extend_from_slice(&(0u32).to_be_bytes());

        let response = Frame {
            frame_type: FrameType::Method,
            channel,
            payload,
        };
        response.write_to_stream(&mut self.stream).await
    }

    async fn handle_channel_close(&mut self, channel: u16) -> io::Result<()> {
        if let Some(ch) = self.channels.get_mut(&channel) {
            ch.state = ChannelState::Closed;
        }

        // Send Channel.Close-Ok
        let mut payload = Vec::new();
        payload.extend_from_slice(&class_id::CHANNEL.to_be_bytes());
        payload.extend_from_slice(&method_id::CHANNEL_CLOSE_OK.to_be_bytes());

        let response = Frame {
            frame_type: FrameType::Method,
            channel,
            payload,
        };
        response.write_to_stream(&mut self.stream).await
    }

    // Exchange methods
    async fn handle_exchange_declare(
        &mut self,
        channel: u16,
        method: &MethodFrame,
    ) -> io::Result<()> {
        // Parse exchange name from arguments (skip reserved fields)
        let mut offset = 0;
        // Skip reserved1 (u16), reserved2 (u16)
        offset += 4;
        // Exchange name (short string)
        if offset + 1 > method.arguments.len() {
            return Ok(());
        }
        let name_len = method.arguments[offset] as usize;
        offset += 1;
        if offset + name_len > method.arguments.len() {
            return Ok(());
        }
        let exchange_name = String::from_utf8_lossy(&method.arguments[offset..offset + name_len]);

        tracing::debug!("Exchange declare: {}", exchange_name);

        // Send Exchange.Declare-Ok
        let mut payload = Vec::new();
        payload.extend_from_slice(&class_id::EXCHANGE.to_be_bytes());
        payload.extend_from_slice(&method_id::EXCHANGE_DECLARE_OK.to_be_bytes());

        let response = Frame {
            frame_type: FrameType::Method,
            channel,
            payload,
        };
        response.write_to_stream(&mut self.stream).await
    }

    // Queue methods
    async fn handle_queue_declare(&mut self, channel: u16, method: &MethodFrame) -> io::Result<()> {
        // Parse queue name from arguments
        let mut offset = 0;
        // Skip reserved1 (u16)
        offset += 2;
        // Queue name (short string)
        if offset + 1 > method.arguments.len() {
            return Ok(());
        }
        let name_len = method.arguments[offset] as usize;
        offset += 1;
        if offset + name_len > method.arguments.len() {
            return Ok(());
        }
        let queue_name = String::from_utf8_lossy(&method.arguments[offset..offset + name_len]);

        tracing::debug!("Queue declare: {}", queue_name);

        // Send Queue.Declare-Ok
        let mut payload = Vec::new();
        payload.extend_from_slice(&class_id::QUEUE.to_be_bytes());
        payload.extend_from_slice(&method_id::QUEUE_DECLARE_OK.to_be_bytes());
        // Queue name
        payload.push(queue_name.len() as u8);
        payload.extend_from_slice(queue_name.as_bytes());
        // Message count
        payload.extend_from_slice(&0u32.to_be_bytes());
        // Consumer count
        payload.extend_from_slice(&0u32.to_be_bytes());

        let response = Frame {
            frame_type: FrameType::Method,
            channel,
            payload,
        };
        response.write_to_stream(&mut self.stream).await
    }

    async fn handle_queue_bind(&mut self, channel: u16, _method: &MethodFrame) -> io::Result<()> {
        // Parse arguments: reserved, queue, exchange, routing_key, arguments
        tracing::debug!("Queue bind received");

        // Send Queue.Bind-Ok
        let mut payload = Vec::new();
        payload.extend_from_slice(&class_id::QUEUE.to_be_bytes());
        payload.extend_from_slice(&method_id::QUEUE_BIND_OK.to_be_bytes());

        let response = Frame {
            frame_type: FrameType::Method,
            channel,
            payload,
        };
        response.write_to_stream(&mut self.stream).await
    }

    // Basic methods
    async fn handle_basic_publish(&mut self, channel: u16, method: &MethodFrame) -> io::Result<()> {
        // Parse routing key and other publish parameters
        let mut offset = 0;
        // Skip reserved1 (u16)
        offset += 2;
        // Exchange name (short string)
        if offset + 1 > method.arguments.len() {
            return Ok(());
        }
        let exchange_len = method.arguments[offset] as usize;
        offset += 1 + exchange_len;
        // Routing key (short string)
        if offset + 1 > method.arguments.len() {
            return Ok(());
        }
        let routing_key_len = method.arguments[offset] as usize;
        offset += 1;
        if offset + routing_key_len > method.arguments.len() {
            return Ok(());
        }
        let routing_key =
            String::from_utf8_lossy(&method.arguments[offset..offset + routing_key_len])
                .to_string();

        tracing::debug!("Basic publish: routing_key={}", routing_key);

        // Handle publisher confirms
        if let Some(ch) = self.channels.get_mut(&channel) {
            if ch.publisher_confirms {
                let delivery_tag = ch.next_delivery_tag;
                ch.next_delivery_tag += 1;
                ch.unconfirmed_messages.insert(delivery_tag, routing_key.clone());

                // Send Basic.Ack for publisher confirms
                let mut payload = Vec::new();
                payload.extend_from_slice(&class_id::BASIC.to_be_bytes());
                payload.extend_from_slice(&method_id::BASIC_ACK.to_be_bytes());
                payload.extend_from_slice(&delivery_tag.to_be_bytes()); // delivery_tag
                payload.push(0); // multiple flag

                let response = Frame {
                    frame_type: FrameType::Method,
                    channel,
                    payload,
                };
                response.write_to_stream(&mut self.stream).await?;
            }
        }

        Ok(())
    }

    async fn handle_basic_consume(
        &mut self,
        channel: u16,
        _method: &MethodFrame,
    ) -> io::Result<()> {
        // Parse consumer tag
        let consumer_tag = format!("ctag-{}", self.next_consumer_tag);
        self.next_consumer_tag += 1;

        if let Some(ch) = self.channels.get_mut(&channel) {
            ch.consumer_tag = Some(consumer_tag.clone());
        }

        tracing::debug!("Basic consume: consumer_tag={}", consumer_tag);

        // Send Basic.Consume-Ok
        let mut payload = Vec::new();
        payload.extend_from_slice(&class_id::BASIC.to_be_bytes());
        payload.extend_from_slice(&method_id::BASIC_CONSUME_OK.to_be_bytes());
        // Consumer tag
        payload.push(consumer_tag.len() as u8);
        payload.extend_from_slice(consumer_tag.as_bytes());

        let response = Frame {
            frame_type: FrameType::Method,
            channel,
            payload,
        };
        response.write_to_stream(&mut self.stream).await
    }

    async fn handle_basic_ack(&mut self, _channel: u16, _method: &MethodFrame) -> io::Result<()> {
        // Handle message acknowledgment
        tracing::debug!("Basic ack received");
        Ok(())
    }

    async fn handle_basic_qos(&mut self, channel: u16, method: &MethodFrame) -> io::Result<()> {
        // Parse prefetch count
        if method.arguments.len() >= 6 {
            let prefetch_count = u16::from_be_bytes([method.arguments[4], method.arguments[5]]);
            if let Some(ch) = self.channels.get_mut(&channel) {
                ch.prefetch_count = prefetch_count;
            }
        }

        // Send Basic.Qos-Ok
        let mut payload = Vec::new();
        payload.extend_from_slice(&class_id::BASIC.to_be_bytes());
        payload.extend_from_slice(&method_id::BASIC_QOS_OK.to_be_bytes());

        let response = Frame {
            frame_type: FrameType::Method,
            channel,
            payload,
        };
        response.write_to_stream(&mut self.stream).await
    }

    async fn handle_basic_get(&mut self, channel: u16, method: &MethodFrame) -> io::Result<()> {
        // Parse queue name
        let mut offset = 0;
        // Skip reserved1 (u16)
        offset += 2;
        // Queue name (short string)
        if offset + 1 > method.arguments.len() {
            return Ok(());
        }
        let queue_len = method.arguments[offset] as usize;
        offset += 1;
        if offset + queue_len > method.arguments.len() {
            return Ok(());
        }
        let queue_name = String::from_utf8_lossy(&method.arguments[offset..offset + queue_len]);

        tracing::debug!("Basic get from queue: {}", queue_name);

        // For now, send Basic.Get-Empty (no messages available)
        let mut payload = Vec::new();
        payload.extend_from_slice(&class_id::BASIC.to_be_bytes());
        payload.extend_from_slice(&method_id::BASIC_GET_EMPTY.to_be_bytes());
        // Cluster ID (empty string)
        payload.extend_from_slice(&(0u8 as u32).to_be_bytes());

        let response = Frame {
            frame_type: FrameType::Method,
            channel,
            payload,
        };
        response.write_to_stream(&mut self.stream).await
    }

    // Transaction methods
    async fn handle_tx_select(&mut self, channel: u16) -> io::Result<()> {
        if let Some(ch) = self.channels.get_mut(&channel) {
            ch.transaction_mode = true;
        }

        // Send Tx.Select-Ok
        let mut payload = Vec::new();
        payload.extend_from_slice(&class_id::TX.to_be_bytes());
        payload.extend_from_slice(&method_id::TX_SELECT_OK.to_be_bytes());

        let response = Frame {
            frame_type: FrameType::Method,
            channel,
            payload,
        };
        response.write_to_stream(&mut self.stream).await
    }

    async fn handle_tx_commit(&mut self, channel: u16) -> io::Result<()> {
        // In a full implementation, this would commit the transaction
        tracing::debug!("Transaction commit on channel {}", channel);

        // Send Tx.Commit-Ok
        let mut payload = Vec::new();
        payload.extend_from_slice(&class_id::TX.to_be_bytes());
        payload.extend_from_slice(&method_id::TX_COMMIT_OK.to_be_bytes());

        let response = Frame {
            frame_type: FrameType::Method,
            channel,
            payload,
        };
        response.write_to_stream(&mut self.stream).await
    }

    async fn handle_tx_rollback(&mut self, channel: u16) -> io::Result<()> {
        // In a full implementation, this would rollback the transaction
        tracing::debug!("Transaction rollback on channel {}", channel);

        // Send Tx.Rollback-Ok
        let mut payload = Vec::new();
        payload.extend_from_slice(&class_id::TX.to_be_bytes());
        payload.extend_from_slice(&method_id::TX_ROLLBACK_OK.to_be_bytes());

        let response = Frame {
            frame_type: FrameType::Method,
            channel,
            payload,
        };
        response.write_to_stream(&mut self.stream).await
    }

    // Confirm methods
    async fn handle_confirm_select(&mut self, channel: u16) -> io::Result<()> {
        if let Some(ch) = self.channels.get_mut(&channel) {
            ch.publisher_confirms = true;
        }

        tracing::debug!("Publisher confirms enabled on channel {}", channel);

        // Send Confirm.Select-Ok
        let mut payload = Vec::new();
        payload.extend_from_slice(&class_id::CONFIRM.to_be_bytes());
        payload.extend_from_slice(&method_id::CONFIRM_SELECT_OK.to_be_bytes());

        let response = Frame {
            frame_type: FrameType::Method,
            channel,
            payload,
        };
        response.write_to_stream(&mut self.stream).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== Class ID Tests ====================

    #[test]
    fn test_class_ids_are_correct() {
        assert_eq!(class_id::CONNECTION, 10);
        assert_eq!(class_id::CHANNEL, 20);
        assert_eq!(class_id::EXCHANGE, 40);
        assert_eq!(class_id::QUEUE, 50);
        assert_eq!(class_id::BASIC, 60);
        assert_eq!(class_id::TX, 90);
        assert_eq!(class_id::CONFIRM, 85);
    }

    // ==================== Connection Method ID Tests ====================

    #[test]
    fn test_connection_method_ids() {
        assert_eq!(method_id::CONNECTION_START, 10);
        assert_eq!(method_id::CONNECTION_START_OK, 11);
        assert_eq!(method_id::CONNECTION_TUNE, 30);
        assert_eq!(method_id::CONNECTION_TUNE_OK, 31);
        assert_eq!(method_id::CONNECTION_OPEN, 40);
        assert_eq!(method_id::CONNECTION_OPEN_OK, 41);
        assert_eq!(method_id::CONNECTION_CLOSE, 50);
        assert_eq!(method_id::CONNECTION_CLOSE_OK, 51);
    }

    // ==================== Channel Method ID Tests ====================

    #[test]
    fn test_channel_method_ids() {
        assert_eq!(method_id::CHANNEL_OPEN, 10);
        assert_eq!(method_id::CHANNEL_OPEN_OK, 11);
        assert_eq!(method_id::CHANNEL_CLOSE, 40);
        assert_eq!(method_id::CHANNEL_CLOSE_OK, 41);
    }

    // ==================== Exchange Method ID Tests ====================

    #[test]
    fn test_exchange_method_ids() {
        assert_eq!(method_id::EXCHANGE_DECLARE, 10);
        assert_eq!(method_id::EXCHANGE_DECLARE_OK, 11);
        assert_eq!(method_id::EXCHANGE_DELETE, 20);
        assert_eq!(method_id::EXCHANGE_DELETE_OK, 21);
    }

    // ==================== Queue Method ID Tests ====================

    #[test]
    fn test_queue_method_ids() {
        assert_eq!(method_id::QUEUE_DECLARE, 10);
        assert_eq!(method_id::QUEUE_DECLARE_OK, 11);
        assert_eq!(method_id::QUEUE_BIND, 20);
        assert_eq!(method_id::QUEUE_BIND_OK, 21);
        assert_eq!(method_id::QUEUE_UNBIND, 50);
        assert_eq!(method_id::QUEUE_UNBIND_OK, 51);
        assert_eq!(method_id::QUEUE_DELETE, 40);
        assert_eq!(method_id::QUEUE_DELETE_OK, 41);
    }

    // ==================== Basic Method ID Tests ====================

    #[test]
    fn test_basic_method_ids() {
        assert_eq!(method_id::BASIC_PUBLISH, 40);
        assert_eq!(method_id::BASIC_CONSUME, 20);
        assert_eq!(method_id::BASIC_CONSUME_OK, 21);
        assert_eq!(method_id::BASIC_DELIVER, 60);
        assert_eq!(method_id::BASIC_ACK, 80);
        assert_eq!(method_id::BASIC_NACK, 120);
        assert_eq!(method_id::BASIC_QOS, 10);
        assert_eq!(method_id::BASIC_QOS_OK, 11);
        assert_eq!(method_id::BASIC_GET, 70);
        assert_eq!(method_id::BASIC_GET_OK, 71);
        assert_eq!(method_id::BASIC_GET_EMPTY, 72);
    }

    // ==================== Transaction Method ID Tests ====================

    #[test]
    fn test_tx_method_ids() {
        assert_eq!(method_id::TX_SELECT, 10);
        assert_eq!(method_id::TX_SELECT_OK, 11);
        assert_eq!(method_id::TX_COMMIT, 20);
        assert_eq!(method_id::TX_COMMIT_OK, 21);
        assert_eq!(method_id::TX_ROLLBACK, 30);
        assert_eq!(method_id::TX_ROLLBACK_OK, 31);
    }

    // ==================== Confirm Method ID Tests ====================

    #[test]
    fn test_confirm_method_ids() {
        assert_eq!(method_id::CONFIRM_SELECT, 10);
        assert_eq!(method_id::CONFIRM_SELECT_OK, 11);
    }

    // ==================== MethodFrame Tests ====================

    #[test]
    fn test_method_frame_creation() {
        let frame = MethodFrame {
            class_id: class_id::CONNECTION,
            method_id: method_id::CONNECTION_START,
            arguments: vec![0x01, 0x02, 0x03],
        };

        assert_eq!(frame.class_id, 10);
        assert_eq!(frame.method_id, 10);
        assert_eq!(frame.arguments.len(), 3);
    }

    #[test]
    fn test_method_frame_empty_arguments() {
        let frame = MethodFrame {
            class_id: class_id::CHANNEL,
            method_id: method_id::CHANNEL_OPEN,
            arguments: vec![],
        };

        assert_eq!(frame.class_id, 20);
        assert_eq!(frame.method_id, 10);
        assert!(frame.arguments.is_empty());
    }

    // ==================== ChannelState Tests ====================

    #[test]
    fn test_channel_state_debug() {
        let closed = ChannelState::Closed;
        let opening = ChannelState::Opening;
        let open = ChannelState::Open;
        let closing = ChannelState::Closing;

        assert_eq!(format!("{:?}", closed), "Closed");
        assert_eq!(format!("{:?}", opening), "Opening");
        assert_eq!(format!("{:?}", open), "Open");
        assert_eq!(format!("{:?}", closing), "Closing");
    }

    #[test]
    fn test_channel_state_clone() {
        let state = ChannelState::Open;
        let cloned = state.clone();
        // Both should be Open
        assert!(matches!(cloned, ChannelState::Open));
    }

    // ==================== Channel Tests ====================

    #[test]
    fn test_channel_creation() {
        let channel = Channel {
            id: 1,
            state: ChannelState::Open,
            consumer_tag: None,
            prefetch_count: 10,
            prefetch_size: 0,
            publisher_confirms: false,
            transaction_mode: false,
            next_delivery_tag: 1,
            unconfirmed_messages: HashMap::new(),
        };

        assert_eq!(channel.id, 1);
        assert!(matches!(channel.state, ChannelState::Open));
        assert!(channel.consumer_tag.is_none());
        assert_eq!(channel.prefetch_count, 10);
        assert!(!channel.publisher_confirms);
        assert!(!channel.transaction_mode);
    }

    #[test]
    fn test_channel_with_consumer() {
        let channel = Channel {
            id: 2,
            state: ChannelState::Open,
            consumer_tag: Some("ctag-1".to_string()),
            prefetch_count: 5,
            prefetch_size: 65536,
            publisher_confirms: true,
            transaction_mode: false,
            next_delivery_tag: 100,
            unconfirmed_messages: HashMap::new(),
        };

        assert_eq!(channel.consumer_tag, Some("ctag-1".to_string()));
        assert!(channel.publisher_confirms);
        assert_eq!(channel.next_delivery_tag, 100);
    }

    #[test]
    fn test_channel_unconfirmed_messages() {
        let mut channel = Channel {
            id: 1,
            state: ChannelState::Open,
            consumer_tag: None,
            prefetch_count: 0,
            prefetch_size: 0,
            publisher_confirms: true,
            transaction_mode: false,
            next_delivery_tag: 1,
            unconfirmed_messages: HashMap::new(),
        };

        // Simulate publisher confirms tracking
        channel.unconfirmed_messages.insert(1, "test.routing.key".to_string());
        channel.unconfirmed_messages.insert(2, "another.key".to_string());
        channel.next_delivery_tag = 3;

        assert_eq!(channel.unconfirmed_messages.len(), 2);
        assert_eq!(channel.unconfirmed_messages.get(&1), Some(&"test.routing.key".to_string()));
        assert_eq!(channel.next_delivery_tag, 3);
    }

    // ==================== Frame Tests ====================

    #[test]
    fn test_frame_creation_method() {
        let frame = Frame {
            frame_type: FrameType::Method,
            channel: 1,
            payload: vec![0x00, 0x0A, 0x00, 0x0A], // Connection.Start
        };

        assert!(matches!(frame.frame_type, FrameType::Method));
        assert_eq!(frame.channel, 1);
        assert_eq!(frame.payload.len(), 4);
    }

    #[test]
    fn test_frame_creation_heartbeat() {
        let frame = Frame {
            frame_type: FrameType::Heartbeat,
            channel: 0,
            payload: vec![],
        };

        assert!(matches!(frame.frame_type, FrameType::Heartbeat));
        assert_eq!(frame.channel, 0);
        assert!(frame.payload.is_empty());
    }

    #[test]
    fn test_frame_creation_body() {
        let body_data = b"Hello, AMQP!".to_vec();
        let frame = Frame {
            frame_type: FrameType::Body,
            channel: 1,
            payload: body_data.clone(),
        };

        assert!(matches!(frame.frame_type, FrameType::Body));
        assert_eq!(frame.payload, body_data);
    }

    #[test]
    fn test_frame_creation_header() {
        let frame = Frame {
            frame_type: FrameType::Header,
            channel: 1,
            payload: vec![0x00, 0x3C, 0x00, 0x00], // Basic class properties
        };

        assert!(matches!(frame.frame_type, FrameType::Header));
        assert_eq!(frame.channel, 1);
    }

    // ==================== FrameType Tests ====================

    #[test]
    fn test_frame_type_values() {
        assert_eq!(FrameType::Method as u8, 1);
        assert_eq!(FrameType::Header as u8, 2);
        assert_eq!(FrameType::Body as u8, 3);
        assert_eq!(FrameType::Heartbeat as u8, 8);
    }

    #[test]
    fn test_frame_type_debug() {
        assert_eq!(format!("{:?}", FrameType::Method), "Method");
        assert_eq!(format!("{:?}", FrameType::Header), "Header");
        assert_eq!(format!("{:?}", FrameType::Body), "Body");
        assert_eq!(format!("{:?}", FrameType::Heartbeat), "Heartbeat");
    }

    // ==================== Method Frame Parsing Helper Tests ====================

    #[test]
    fn test_parse_class_method_from_payload() {
        // Simulate parsing a method frame payload
        let payload: Vec<u8> = vec![
            0x00, 0x0A, // class_id: 10 (Connection)
            0x00, 0x0A, // method_id: 10 (Start)
            0x01, 0x02, 0x03, // arguments
        ];

        let class_id = u16::from_be_bytes([payload[0], payload[1]]);
        let method_id = u16::from_be_bytes([payload[2], payload[3]]);
        let arguments = payload[4..].to_vec();

        assert_eq!(class_id, 10);
        assert_eq!(method_id, 10);
        assert_eq!(arguments, vec![0x01, 0x02, 0x03]);
    }

    #[test]
    fn test_parse_channel_open_payload() {
        // Channel.Open method frame
        let payload: Vec<u8> = vec![
            0x00, 0x14, // class_id: 20 (Channel)
            0x00, 0x0A, // method_id: 10 (Open)
        ];

        let class_id = u16::from_be_bytes([payload[0], payload[1]]);
        let method_id = u16::from_be_bytes([payload[2], payload[3]]);

        assert_eq!(class_id, class_id::CHANNEL);
        assert_eq!(method_id, method_id::CHANNEL_OPEN);
    }

    #[test]
    fn test_parse_queue_declare_payload() {
        // Queue.Declare method frame
        let payload: Vec<u8> = vec![
            0x00, 0x32, // class_id: 50 (Queue)
            0x00, 0x0A, // method_id: 10 (Declare)
            0x00, 0x00, // reserved1
            0x04, // queue name length
            b't', b'e', b's', b't', // queue name: "test"
        ];

        let class_id = u16::from_be_bytes([payload[0], payload[1]]);
        let method_id = u16::from_be_bytes([payload[2], payload[3]]);

        assert_eq!(class_id, class_id::QUEUE);
        assert_eq!(method_id, method_id::QUEUE_DECLARE);

        // Parse queue name
        let name_len = payload[6] as usize;
        let queue_name = String::from_utf8_lossy(&payload[7..7 + name_len]);
        assert_eq!(queue_name, "test");
    }

    #[test]
    fn test_parse_basic_qos_payload() {
        // Basic.Qos method frame
        let payload: Vec<u8> = vec![
            0x00, 0x3C, // class_id: 60 (Basic)
            0x00, 0x0A, // method_id: 10 (Qos)
            0x00, 0x00, 0x00, 0x00, // prefetch_size: 0
            0x00, 0x0A, // prefetch_count: 10
            0x00, // global: false
        ];

        let class_id = u16::from_be_bytes([payload[0], payload[1]]);
        let method_id = u16::from_be_bytes([payload[2], payload[3]]);

        assert_eq!(class_id, class_id::BASIC);
        assert_eq!(method_id, method_id::BASIC_QOS);

        // Parse prefetch_count (offset 8-9)
        let prefetch_count = u16::from_be_bytes([payload[8], payload[9]]);
        assert_eq!(prefetch_count, 10);
    }

    // ==================== Frame Serialization Tests ====================

    #[test]
    fn test_frame_header_bytes() {
        let frame = Frame {
            frame_type: FrameType::Method,
            channel: 1,
            payload: vec![0x00, 0x0A, 0x00, 0x0A],
        };

        // Verify frame type byte
        let frame_type_byte: u8 = match frame.frame_type {
            FrameType::Method => 1,
            FrameType::Header => 2,
            FrameType::Body => 3,
            FrameType::Heartbeat => 8,
        };
        assert_eq!(frame_type_byte, 1);

        // Verify channel bytes
        let channel_bytes = frame.channel.to_be_bytes();
        assert_eq!(channel_bytes, [0x00, 0x01]);

        // Verify size bytes
        let size = frame.payload.len() as u32;
        let size_bytes = size.to_be_bytes();
        assert_eq!(size_bytes, [0x00, 0x00, 0x00, 0x04]);
    }

    #[test]
    fn test_frame_end_marker() {
        // AMQP frame end marker should be 0xCE
        let frame_end: u8 = 0xCE;
        assert_eq!(frame_end, 206);
    }

    // ==================== AMQP Protocol Header Tests ====================

    #[test]
    fn test_amqp_protocol_header() {
        // AMQP 0.9.1 protocol header
        let header = b"AMQP\x00\x00\x09\x01";

        assert_eq!(header[0], b'A');
        assert_eq!(header[1], b'M');
        assert_eq!(header[2], b'Q');
        assert_eq!(header[3], b'P');
        assert_eq!(header[4], 0x00); // Protocol ID
        assert_eq!(header[5], 0x00); // Major version
        assert_eq!(header[6], 0x09); // Minor version
        assert_eq!(header[7], 0x01); // Revision
    }

    // ==================== Transaction Mode Tests ====================

    #[test]
    fn test_channel_transaction_mode() {
        let mut channel = Channel {
            id: 1,
            state: ChannelState::Open,
            consumer_tag: None,
            prefetch_count: 0,
            prefetch_size: 0,
            publisher_confirms: false,
            transaction_mode: false,
            next_delivery_tag: 1,
            unconfirmed_messages: HashMap::new(),
        };

        assert!(!channel.transaction_mode);

        // Enable transaction mode (simulating Tx.Select)
        channel.transaction_mode = true;
        assert!(channel.transaction_mode);
    }

    #[test]
    fn test_channel_publisher_confirms() {
        let mut channel = Channel {
            id: 1,
            state: ChannelState::Open,
            consumer_tag: None,
            prefetch_count: 0,
            prefetch_size: 0,
            publisher_confirms: false,
            transaction_mode: false,
            next_delivery_tag: 1,
            unconfirmed_messages: HashMap::new(),
        };

        assert!(!channel.publisher_confirms);

        // Enable publisher confirms (simulating Confirm.Select)
        channel.publisher_confirms = true;
        assert!(channel.publisher_confirms);
    }
}
