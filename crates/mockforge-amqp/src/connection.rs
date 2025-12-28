//! AMQP Connection Management
//!
//! This module handles AMQP 0.9.1 connection lifecycle, channel management,
//! and message routing between clients and the broker.

use std::collections::HashMap;
use std::io;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tokio_rustls::server::TlsStream;

use crate::bindings::Binding;
use crate::consumers::Consumer;
use crate::exchanges::{ExchangeManager, ExchangeType};
use crate::messages::{Message, MessageProperties, QueuedMessage};
use crate::metrics::AmqpMetrics;
use crate::protocol::{class_id, method_id, Frame, FrameType};
use crate::queues::{QueueManager, QueueNotifyReceiver};

/// Simple UUID generator for queue/consumer names
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

fn generate_id() -> u64 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

/// Connection state machine
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    /// Initial state, waiting for protocol header
    Start,
    /// Protocol header received, waiting for Start-Ok
    Starting,
    /// Tune parameters being negotiated
    Tuning,
    /// Connection open, ready for operations
    Open,
    /// Connection closing
    Closing,
    /// Connection closed
    Closed,
}

/// Channel state
#[derive(Debug, Clone, PartialEq)]
pub enum ChannelState {
    Closed,
    Opening,
    Open,
    Closing,
}

/// Unacknowledged message tracking
#[derive(Debug, Clone)]
pub struct UnackedMessage {
    pub delivery_tag: u64,
    pub queue_name: String,
    pub message: Message,
    pub redelivered: bool,
}

/// Channel state and operations
#[derive(Debug)]
pub struct Channel {
    pub id: u16,
    pub state: ChannelState,
    pub consumers: HashMap<String, Consumer>,
    pub prefetch_count: u16,
    pub prefetch_size: u32,
    pub global_prefetch: bool,
    pub publisher_confirms: bool,
    pub transaction_mode: bool,
    pub next_delivery_tag: u64,
    pub next_consumer_tag: u32,
    /// Unacknowledged messages: delivery_tag -> UnackedMessage
    pub unacked_messages: HashMap<u64, UnackedMessage>,
    /// Transaction pending messages (uncommitted)
    pub tx_pending_publishes: Vec<(String, String, Message)>, // (exchange, routing_key, message)
    pub tx_pending_acks: Vec<u64>,
}

impl Channel {
    pub fn new(id: u16) -> Self {
        Self {
            id,
            state: ChannelState::Open,
            consumers: HashMap::new(),
            prefetch_count: 0,
            prefetch_size: 0,
            global_prefetch: false,
            publisher_confirms: false,
            transaction_mode: false,
            next_delivery_tag: 1,
            next_consumer_tag: 1,
            unacked_messages: HashMap::new(),
            tx_pending_publishes: Vec::new(),
            tx_pending_acks: Vec::new(),
        }
    }

    /// Generate and return the next delivery tag
    pub fn next_delivery_tag(&mut self) -> u64 {
        let tag = self.next_delivery_tag;
        self.next_delivery_tag += 1;
        tag
    }

    /// Generate and return the next consumer tag
    pub fn generate_consumer_tag(&mut self) -> String {
        let tag = format!("amq.ctag-{}.{}", self.id, self.next_consumer_tag);
        self.next_consumer_tag += 1;
        tag
    }
}

/// Content frame state for multi-frame messages
#[derive(Debug)]
pub struct ContentState {
    pub class_id: u16,
    pub body_size: u64,
    pub properties: MessageProperties,
    pub body: Vec<u8>,
    pub exchange: String,
    pub routing_key: String,
    pub mandatory: bool,
    pub immediate: bool,
}

/// Stream type that can be either plain TCP or TLS-encrypted
pub enum AmqpStream {
    /// Plain TCP stream
    Plain(TcpStream),
    /// TLS-encrypted stream
    Tls(TlsStream<TcpStream>),
}

impl AsyncRead for AmqpStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        match self.get_mut() {
            AmqpStream::Plain(stream) => Pin::new(stream).poll_read(cx, buf),
            AmqpStream::Tls(stream) => Pin::new(stream).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for AmqpStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        match self.get_mut() {
            AmqpStream::Plain(stream) => Pin::new(stream).poll_write(cx, buf),
            AmqpStream::Tls(stream) => Pin::new(stream).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            AmqpStream::Plain(stream) => Pin::new(stream).poll_flush(cx),
            AmqpStream::Tls(stream) => Pin::new(stream).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            AmqpStream::Plain(stream) => Pin::new(stream).poll_shutdown(cx),
            AmqpStream::Tls(stream) => Pin::new(stream).poll_shutdown(cx),
        }
    }
}

/// AMQP Connection Handler
///
/// Handles the full AMQP 0.9.1 protocol for a single client connection.
pub struct AmqpConnection {
    stream: AmqpStream,
    state: ConnectionState,
    channels: HashMap<u16, Channel>,
    exchanges: Arc<RwLock<ExchangeManager>>,
    queues: Arc<RwLock<QueueManager>>,
    metrics: Arc<AmqpMetrics>,
    /// Negotiated parameters
    channel_max: u16,
    frame_max: u32,
    heartbeat: u16,
    /// Current content frame state (for multi-frame messages)
    content_state: Option<(u16, ContentState)>, // (channel, state)
    /// Virtual host
    vhost: String,
    /// Queue notification receiver for consumer delivery
    queue_notify_rx: Option<QueueNotifyReceiver>,
}

impl AmqpConnection {
    /// Create a new AMQP connection handler for a plain TCP stream
    pub async fn new(
        stream: TcpStream,
        exchanges: Arc<RwLock<ExchangeManager>>,
        queues: Arc<RwLock<QueueManager>>,
        metrics: Arc<AmqpMetrics>,
    ) -> Self {
        Self::new_with_stream(AmqpStream::Plain(stream), exchanges, queues, metrics).await
    }

    /// Create a new AMQP connection handler for a TLS stream
    pub async fn new_tls(
        stream: TlsStream<TcpStream>,
        exchanges: Arc<RwLock<ExchangeManager>>,
        queues: Arc<RwLock<QueueManager>>,
        metrics: Arc<AmqpMetrics>,
    ) -> Self {
        Self::new_with_stream(AmqpStream::Tls(stream), exchanges, queues, metrics).await
    }

    /// Create a new AMQP connection handler with any stream type
    async fn new_with_stream(
        stream: AmqpStream,
        exchanges: Arc<RwLock<ExchangeManager>>,
        queues: Arc<RwLock<QueueManager>>,
        metrics: Arc<AmqpMetrics>,
    ) -> Self {
        let mut channels = HashMap::new();
        // Channel 0 is always open for connection-level operations
        channels.insert(0, Channel::new(0));

        // Subscribe to queue notifications for consumer delivery
        let queue_notify_rx = {
            let queues_guard = queues.read().await;
            Some(queues_guard.subscribe())
        };

        Self {
            stream,
            state: ConnectionState::Start,
            channels,
            exchanges,
            queues,
            metrics,
            channel_max: 2047,
            frame_max: 131072,
            heartbeat: 60,
            content_state: None,
            vhost: "/".to_string(),
            queue_notify_rx,
        }
    }

    /// Handle the AMQP connection lifecycle
    pub async fn handle(mut self) -> io::Result<()> {
        self.metrics.record_connection();

        // Wait for client protocol header
        let result = self.wait_for_protocol_header().await;
        if result.is_err() {
            self.metrics.record_connection_closed();
            return result;
        }

        // Send Connection.Start
        self.send_connection_start().await?;
        self.state = ConnectionState::Starting;

        // Main frame loop with consumer delivery support
        loop {
            // Take the notification receiver for use in select!
            let mut notify_rx = self.queue_notify_rx.take();

            tokio::select! {
                biased;

                // Handle incoming frames from client
                frame_result = self.read_and_handle_frame() => {
                    // Put the receiver back
                    self.queue_notify_rx = notify_rx;

                    match frame_result {
                        Ok(should_continue) => {
                            if !should_continue {
                                break;
                            }
                        }
                        Err(e) => {
                            if e.kind() == io::ErrorKind::UnexpectedEof {
                                tracing::debug!("Client disconnected");
                            } else {
                                tracing::error!("Connection error: {}", e);
                                self.metrics.record_error(&e.to_string());
                            }
                            break;
                        }
                    }
                }

                // Handle queue notifications for consumer delivery
                notification = async {
                    if let Some(ref mut rx) = notify_rx {
                        rx.recv().await
                    } else {
                        // No receiver, wait forever (will never complete)
                        std::future::pending::<Result<crate::queues::QueueNotification, tokio::sync::broadcast::error::RecvError>>().await
                    }
                } => {
                    // Put the receiver back
                    self.queue_notify_rx = notify_rx;

                    if let Ok(notification) = notification {
                        // Check if we have consumers for this queue and deliver messages
                        if let Err(e) = self.deliver_to_consumers(&notification.queue_name).await {
                            tracing::warn!("Failed to deliver to consumers: {}", e);
                        }
                    }
                }
            }
        }

        // Close all channels
        for (channel_id, _) in self.channels.iter() {
            if *channel_id != 0 {
                self.metrics.record_channel_closed();
            }
        }
        self.metrics.record_connection_closed();

        Ok(())
    }

    /// Wait for and validate the AMQP protocol header from client
    async fn wait_for_protocol_header(&mut self) -> io::Result<()> {
        let mut header = [0u8; 8];
        self.stream.read_exact(&mut header).await?;

        // Validate AMQP 0.9.1 header
        if &header[0..4] != b"AMQP" {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid AMQP protocol header"));
        }

        // Check version (0-9-1)
        if header[4] != 0 || header[5] != 0 || header[6] != 9 || header[7] != 1 {
            // Send our protocol header back to indicate supported version
            self.stream.write_all(b"AMQP\x00\x00\x09\x01").await?;
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Unsupported AMQP version: {}.{}.{}.{}",
                    header[4], header[5], header[6], header[7]
                ),
            ));
        }

        Ok(())
    }

    /// Read a frame and handle it, returns false if connection should close
    async fn read_and_handle_frame(&mut self) -> io::Result<bool> {
        let frame = Frame::read_from_stream(&mut self.stream).await?;

        match frame.frame_type {
            FrameType::Method => self.handle_method_frame(frame.channel, &frame.payload).await,
            FrameType::Header => self.handle_header_frame(frame.channel, &frame.payload).await,
            FrameType::Body => self.handle_body_frame(frame.channel, &frame.payload).await,
            FrameType::Heartbeat => {
                // Respond to heartbeat with heartbeat
                self.send_heartbeat().await?;
                Ok(true)
            }
        }
    }

    /// Send Connection.Start frame
    async fn send_connection_start(&mut self) -> io::Result<()> {
        let mut payload = Vec::new();

        // Class ID and Method ID
        payload.extend_from_slice(&class_id::CONNECTION.to_be_bytes());
        payload.extend_from_slice(&method_id::CONNECTION_START.to_be_bytes());

        // Version (0-9)
        payload.push(0); // major
        payload.push(9); // minor

        // Server properties (field table)
        let server_props = self.build_server_properties();
        payload.extend_from_slice(&server_props);

        // Mechanisms (long string)
        let mechanisms = b"PLAIN AMQPLAIN";
        payload.extend_from_slice(&(mechanisms.len() as u32).to_be_bytes());
        payload.extend_from_slice(mechanisms);

        // Locales (long string)
        let locales = b"en_US";
        payload.extend_from_slice(&(locales.len() as u32).to_be_bytes());
        payload.extend_from_slice(locales);

        self.send_method_frame(0, &payload).await
    }

    /// Build server properties field table
    fn build_server_properties(&self) -> Vec<u8> {
        let mut table = Vec::new();

        // Add properties
        self.add_field_table_string(&mut table, "product", "MockForge AMQP");
        self.add_field_table_string(&mut table, "version", "0.1.0");
        self.add_field_table_string(&mut table, "platform", "Rust");
        self.add_field_table_string(&mut table, "copyright", "MIT");
        self.add_field_table_string(&mut table, "information", "https://github.com/mockforge");

        // Capabilities table
        let mut caps = Vec::new();
        self.add_field_table_bool(&mut caps, "publisher_confirms", true);
        self.add_field_table_bool(&mut caps, "consumer_cancel_notify", true);
        self.add_field_table_bool(&mut caps, "exchange_exchange_bindings", true);
        self.add_field_table_bool(&mut caps, "basic.nack", true);
        self.add_field_table_bool(&mut caps, "connection.blocked", true);
        self.add_field_table_bool(&mut caps, "authentication_failure_close", true);
        self.add_field_table_bool(&mut caps, "per_consumer_qos", true);

        // Add capabilities as nested table
        self.add_field_table_table(&mut table, "capabilities", &caps);

        // Return with length prefix
        let mut result = Vec::new();
        result.extend_from_slice(&(table.len() as u32).to_be_bytes());
        result.extend_from_slice(&table);
        result
    }

    fn add_field_table_string(&self, table: &mut Vec<u8>, key: &str, value: &str) {
        table.push(key.len() as u8);
        table.extend_from_slice(key.as_bytes());
        table.push(b'S'); // String type
        table.extend_from_slice(&(value.len() as u32).to_be_bytes());
        table.extend_from_slice(value.as_bytes());
    }

    fn add_field_table_bool(&self, table: &mut Vec<u8>, key: &str, value: bool) {
        table.push(key.len() as u8);
        table.extend_from_slice(key.as_bytes());
        table.push(b't'); // Boolean type
        table.push(if value { 1 } else { 0 });
    }

    fn add_field_table_table(&self, table: &mut Vec<u8>, key: &str, nested: &[u8]) {
        table.push(key.len() as u8);
        table.extend_from_slice(key.as_bytes());
        table.push(b'F'); // Table type
        table.extend_from_slice(&(nested.len() as u32).to_be_bytes());
        table.extend_from_slice(nested);
    }

    /// Handle a method frame
    async fn handle_method_frame(&mut self, channel: u16, payload: &[u8]) -> io::Result<bool> {
        if payload.len() < 4 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Method frame too short"));
        }

        let class_id = u16::from_be_bytes([payload[0], payload[1]]);
        let method_id = u16::from_be_bytes([payload[2], payload[3]]);
        let arguments = &payload[4..];

        tracing::debug!(
            "Method frame: class={}, method={}, channel={}",
            class_id,
            method_id,
            channel
        );

        match class_id {
            class_id::CONNECTION => self.handle_connection_method(method_id, arguments).await,
            class_id::CHANNEL => self.handle_channel_method(channel, method_id, arguments).await,
            class_id::EXCHANGE => self.handle_exchange_method(channel, method_id, arguments).await,
            class_id::QUEUE => self.handle_queue_method(channel, method_id, arguments).await,
            class_id::BASIC => self.handle_basic_method(channel, method_id, arguments).await,
            class_id::TX => self.handle_tx_method(channel, method_id, arguments).await,
            class_id::CONFIRM => self.handle_confirm_method(channel, method_id, arguments).await,
            _ => {
                tracing::warn!("Unknown class ID: {}", class_id);
                Ok(true)
            }
        }
    }

    /// Handle connection class methods
    async fn handle_connection_method(
        &mut self,
        method_id: u16,
        arguments: &[u8],
    ) -> io::Result<bool> {
        match method_id {
            method_id::CONNECTION_START_OK => self.handle_connection_start_ok(arguments).await,
            method_id::CONNECTION_TUNE_OK => self.handle_connection_tune_ok(arguments).await,
            method_id::CONNECTION_OPEN => self.handle_connection_open(arguments).await,
            method_id::CONNECTION_CLOSE => self.handle_connection_close(arguments).await,
            method_id::CONNECTION_CLOSE_OK => {
                self.state = ConnectionState::Closed;
                Ok(false) // Connection closed
            }
            _ => {
                tracing::warn!("Unknown connection method: {}", method_id);
                Ok(true)
            }
        }
    }

    async fn handle_connection_start_ok(&mut self, _arguments: &[u8]) -> io::Result<bool> {
        // Parse client properties, mechanism, response, locale
        // For now, just accept any authentication

        // Send Connection.Tune
        self.send_connection_tune().await?;
        self.state = ConnectionState::Tuning;
        Ok(true)
    }

    async fn send_connection_tune(&mut self) -> io::Result<()> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&class_id::CONNECTION.to_be_bytes());
        payload.extend_from_slice(&method_id::CONNECTION_TUNE.to_be_bytes());
        payload.extend_from_slice(&self.channel_max.to_be_bytes());
        payload.extend_from_slice(&self.frame_max.to_be_bytes());
        payload.extend_from_slice(&self.heartbeat.to_be_bytes());

        self.send_method_frame(0, &payload).await
    }

    async fn handle_connection_tune_ok(&mut self, arguments: &[u8]) -> io::Result<bool> {
        if arguments.len() >= 8 {
            let client_channel_max = u16::from_be_bytes([arguments[0], arguments[1]]);
            let client_frame_max =
                u32::from_be_bytes([arguments[2], arguments[3], arguments[4], arguments[5]]);
            let client_heartbeat = u16::from_be_bytes([arguments[6], arguments[7]]);

            // Use client's values if they're more restrictive
            if client_channel_max > 0 && client_channel_max < self.channel_max {
                self.channel_max = client_channel_max;
            }
            if client_frame_max > 0 && client_frame_max < self.frame_max {
                self.frame_max = client_frame_max;
            }
            if client_heartbeat > 0 {
                self.heartbeat = client_heartbeat;
            }
        }
        Ok(true)
    }

    async fn handle_connection_open(&mut self, arguments: &[u8]) -> io::Result<bool> {
        // Parse virtual host (short string)
        if !arguments.is_empty() {
            let vhost_len = arguments[0] as usize;
            if arguments.len() > vhost_len {
                self.vhost = String::from_utf8_lossy(&arguments[1..1 + vhost_len]).to_string();
            }
        }

        // Send Connection.Open-Ok
        let mut payload = Vec::new();
        payload.extend_from_slice(&class_id::CONNECTION.to_be_bytes());
        payload.extend_from_slice(&method_id::CONNECTION_OPEN_OK.to_be_bytes());
        // Reserved (short string - empty)
        payload.push(0);

        self.send_method_frame(0, &payload).await?;
        self.state = ConnectionState::Open;
        Ok(true)
    }

    async fn handle_connection_close(&mut self, _arguments: &[u8]) -> io::Result<bool> {
        // Send Connection.Close-Ok
        let mut payload = Vec::new();
        payload.extend_from_slice(&class_id::CONNECTION.to_be_bytes());
        payload.extend_from_slice(&method_id::CONNECTION_CLOSE_OK.to_be_bytes());

        self.send_method_frame(0, &payload).await?;
        self.state = ConnectionState::Closed;
        Ok(false)
    }

    /// Handle channel class methods
    async fn handle_channel_method(
        &mut self,
        channel: u16,
        method_id: u16,
        _arguments: &[u8],
    ) -> io::Result<bool> {
        match method_id {
            method_id::CHANNEL_OPEN => {
                // Create new channel
                self.channels.insert(channel, Channel::new(channel));
                self.metrics.record_channel_opened();

                // Send Channel.Open-Ok
                let mut payload = Vec::new();
                payload.extend_from_slice(&class_id::CHANNEL.to_be_bytes());
                payload.extend_from_slice(&method_id::CHANNEL_OPEN_OK.to_be_bytes());
                // Reserved (long string - empty)
                payload.extend_from_slice(&0u32.to_be_bytes());

                self.send_method_frame(channel, &payload).await?;
                Ok(true)
            }
            method_id::CHANNEL_CLOSE => {
                if let Some(ch) = self.channels.get_mut(&channel) {
                    ch.state = ChannelState::Closed;
                }
                self.metrics.record_channel_closed();

                // Send Channel.Close-Ok
                let mut payload = Vec::new();
                payload.extend_from_slice(&class_id::CHANNEL.to_be_bytes());
                payload.extend_from_slice(&method_id::CHANNEL_CLOSE_OK.to_be_bytes());

                self.send_method_frame(channel, &payload).await?;
                Ok(true)
            }
            method_id::CHANNEL_CLOSE_OK => {
                if let Some(ch) = self.channels.get_mut(&channel) {
                    ch.state = ChannelState::Closed;
                }
                Ok(true)
            }
            0x14 /* Flow */ => {
                // Channel.Flow - just acknowledge for now
                let mut payload = Vec::new();
                payload.extend_from_slice(&class_id::CHANNEL.to_be_bytes());
                payload.extend_from_slice(&0x15u16.to_be_bytes()); // Flow-Ok
                payload.push(1); // active = true

                self.send_method_frame(channel, &payload).await?;
                Ok(true)
            }
            _ => {
                tracing::warn!("Unknown channel method: {}", method_id);
                Ok(true)
            }
        }
    }

    /// Handle exchange class methods
    async fn handle_exchange_method(
        &mut self,
        channel: u16,
        method_id: u16,
        arguments: &[u8],
    ) -> io::Result<bool> {
        match method_id {
            method_id::EXCHANGE_DECLARE => {
                self.handle_exchange_declare(channel, arguments).await
            }
            method_id::EXCHANGE_DELETE => {
                self.handle_exchange_delete(channel, arguments).await
            }
            0x1E /* Bind */ => {
                self.handle_exchange_bind(channel, arguments).await
            }
            0x28 /* Unbind */ => {
                self.handle_exchange_unbind(channel, arguments).await
            }
            _ => {
                tracing::warn!("Unknown exchange method: {}", method_id);
                Ok(true)
            }
        }
    }

    async fn handle_exchange_declare(
        &mut self,
        channel: u16,
        arguments: &[u8],
    ) -> io::Result<bool> {
        let mut offset = 2; // Skip reserved

        // Exchange name
        let name_len = arguments.get(offset).copied().unwrap_or(0) as usize;
        offset += 1;
        let exchange_name = if offset + name_len <= arguments.len() {
            String::from_utf8_lossy(&arguments[offset..offset + name_len]).to_string()
        } else {
            String::new()
        };
        offset += name_len;

        // Exchange type
        let type_len = arguments.get(offset).copied().unwrap_or(0) as usize;
        offset += 1;
        let exchange_type_str = if offset + type_len <= arguments.len() {
            String::from_utf8_lossy(&arguments[offset..offset + type_len]).to_string()
        } else {
            "direct".to_string()
        };
        offset += type_len;

        // Flags
        let flags = arguments.get(offset).copied().unwrap_or(0);
        let _passive = flags & 0x01 != 0;
        let durable = flags & 0x02 != 0;
        let auto_delete = flags & 0x04 != 0;
        let _internal = flags & 0x08 != 0;
        let no_wait = flags & 0x10 != 0;

        let exchange_type = match exchange_type_str.as_str() {
            "fanout" => ExchangeType::Fanout,
            "topic" => ExchangeType::Topic,
            "headers" => ExchangeType::Headers,
            _ => ExchangeType::Direct,
        };

        tracing::debug!("Exchange declare: name={}, type={:?}", exchange_name, exchange_type);

        // Declare the exchange
        if !exchange_name.is_empty() {
            let mut exchanges = self.exchanges.write().await;
            exchanges.declare_exchange(exchange_name, exchange_type, durable, auto_delete);
            self.metrics.record_exchange_declared();
        }

        // Send Exchange.Declare-Ok (unless no_wait)
        if !no_wait {
            let mut payload = Vec::new();
            payload.extend_from_slice(&class_id::EXCHANGE.to_be_bytes());
            payload.extend_from_slice(&method_id::EXCHANGE_DECLARE_OK.to_be_bytes());
            self.send_method_frame(channel, &payload).await?;
        }

        Ok(true)
    }

    async fn handle_exchange_delete(&mut self, channel: u16, arguments: &[u8]) -> io::Result<bool> {
        let mut offset = 2; // Skip reserved

        let name_len = arguments.get(offset).copied().unwrap_or(0) as usize;
        offset += 1;
        let exchange_name = if offset + name_len <= arguments.len() {
            String::from_utf8_lossy(&arguments[offset..offset + name_len]).to_string()
        } else {
            String::new()
        };
        offset += name_len;

        let flags = arguments.get(offset).copied().unwrap_or(0);
        let no_wait = flags & 0x02 != 0;

        tracing::debug!("Exchange delete: {}", exchange_name);

        {
            let mut exchanges = self.exchanges.write().await;
            exchanges.delete_exchange(&exchange_name);
        }
        self.metrics.record_exchange_deleted();

        if !no_wait {
            let mut payload = Vec::new();
            payload.extend_from_slice(&class_id::EXCHANGE.to_be_bytes());
            payload.extend_from_slice(&method_id::EXCHANGE_DELETE_OK.to_be_bytes());
            self.send_method_frame(channel, &payload).await?;
        }

        Ok(true)
    }

    async fn handle_exchange_bind(&mut self, channel: u16, _arguments: &[u8]) -> io::Result<bool> {
        // Exchange-to-exchange binding (simplified)
        let mut payload = Vec::new();
        payload.extend_from_slice(&class_id::EXCHANGE.to_be_bytes());
        payload.extend_from_slice(&0x1Fu16.to_be_bytes()); // Bind-Ok
        self.send_method_frame(channel, &payload).await?;
        Ok(true)
    }

    async fn handle_exchange_unbind(
        &mut self,
        channel: u16,
        _arguments: &[u8],
    ) -> io::Result<bool> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&class_id::EXCHANGE.to_be_bytes());
        payload.extend_from_slice(&0x33u16.to_be_bytes()); // Unbind-Ok
        self.send_method_frame(channel, &payload).await?;
        Ok(true)
    }

    /// Handle queue class methods
    async fn handle_queue_method(
        &mut self,
        channel: u16,
        method_id: u16,
        arguments: &[u8],
    ) -> io::Result<bool> {
        match method_id {
            method_id::QUEUE_DECLARE => self.handle_queue_declare(channel, arguments).await,
            method_id::QUEUE_BIND => self.handle_queue_bind(channel, arguments).await,
            method_id::QUEUE_UNBIND => self.handle_queue_unbind(channel, arguments).await,
            method_id::QUEUE_DELETE => self.handle_queue_delete(channel, arguments).await,
            0x1E /* Purge */ => self.handle_queue_purge(channel, arguments).await,
            _ => {
                tracing::warn!("Unknown queue method: {}", method_id);
                Ok(true)
            }
        }
    }

    async fn handle_queue_declare(&mut self, channel: u16, arguments: &[u8]) -> io::Result<bool> {
        let mut offset = 2; // Skip reserved

        // Queue name
        let name_len = arguments.get(offset).copied().unwrap_or(0) as usize;
        offset += 1;
        let mut queue_name = if offset + name_len <= arguments.len() {
            String::from_utf8_lossy(&arguments[offset..offset + name_len]).to_string()
        } else {
            String::new()
        };
        offset += name_len;

        // Flags
        let flags = arguments.get(offset).copied().unwrap_or(0);
        let _passive = flags & 0x01 != 0;
        let durable = flags & 0x02 != 0;
        let exclusive = flags & 0x04 != 0;
        let auto_delete = flags & 0x08 != 0;
        let no_wait = flags & 0x10 != 0;

        // Generate queue name if empty
        if queue_name.is_empty() {
            queue_name = format!("amq.gen-{}", generate_id());
        }

        tracing::debug!("Queue declare: {}", queue_name);

        // Declare the queue
        let mut queues = self.queues.write().await;
        queues.declare_queue(queue_name.clone(), durable, exclusive, auto_delete);
        self.metrics.record_queue_declared();
        drop(queues);

        // Send Queue.Declare-Ok
        if !no_wait {
            let queues = self.queues.read().await;
            let (message_count, consumer_count) = if let Some(q) = queues.get_queue(&queue_name) {
                (q.messages.len() as u32, q.consumers.len() as u32)
            } else {
                (0, 0)
            };
            drop(queues);

            let mut payload = Vec::new();
            payload.extend_from_slice(&class_id::QUEUE.to_be_bytes());
            payload.extend_from_slice(&method_id::QUEUE_DECLARE_OK.to_be_bytes());
            // Queue name
            payload.push(queue_name.len() as u8);
            payload.extend_from_slice(queue_name.as_bytes());
            // Message count
            payload.extend_from_slice(&message_count.to_be_bytes());
            // Consumer count
            payload.extend_from_slice(&consumer_count.to_be_bytes());

            self.send_method_frame(channel, &payload).await?;
        }

        Ok(true)
    }

    async fn handle_queue_bind(&mut self, channel: u16, arguments: &[u8]) -> io::Result<bool> {
        let mut offset = 2; // Skip reserved

        // Queue name
        let queue_len = arguments.get(offset).copied().unwrap_or(0) as usize;
        offset += 1;
        let queue_name = if offset + queue_len <= arguments.len() {
            String::from_utf8_lossy(&arguments[offset..offset + queue_len]).to_string()
        } else {
            String::new()
        };
        offset += queue_len;

        // Exchange name
        let exchange_len = arguments.get(offset).copied().unwrap_or(0) as usize;
        offset += 1;
        let exchange_name = if offset + exchange_len <= arguments.len() {
            String::from_utf8_lossy(&arguments[offset..offset + exchange_len]).to_string()
        } else {
            String::new()
        };
        offset += exchange_len;

        // Routing key
        let routing_key_len = arguments.get(offset).copied().unwrap_or(0) as usize;
        offset += 1;
        let routing_key = if offset + routing_key_len <= arguments.len() {
            String::from_utf8_lossy(&arguments[offset..offset + routing_key_len]).to_string()
        } else {
            String::new()
        };
        offset += routing_key_len;

        let flags = arguments.get(offset).copied().unwrap_or(0);
        let no_wait = flags & 0x01 != 0;

        tracing::debug!(
            "Queue bind: queue={}, exchange={}, routing_key={}",
            queue_name,
            exchange_name,
            routing_key
        );

        // Create the binding
        let binding = Binding::new(exchange_name.clone(), queue_name.clone(), routing_key);

        // Add binding to exchange
        {
            let mut exchanges = self.exchanges.write().await;
            // We need to add a method to ExchangeManager for adding bindings
            // For now, we'll handle this through the exchange directly
            if let Some(exchange) = exchanges.get_exchange_mut(&exchange_name) {
                exchange.bindings.push(binding);
                self.metrics.record_binding();
            }
        }

        if !no_wait {
            let mut payload = Vec::new();
            payload.extend_from_slice(&class_id::QUEUE.to_be_bytes());
            payload.extend_from_slice(&method_id::QUEUE_BIND_OK.to_be_bytes());
            self.send_method_frame(channel, &payload).await?;
        }

        Ok(true)
    }

    async fn handle_queue_unbind(&mut self, channel: u16, arguments: &[u8]) -> io::Result<bool> {
        let mut offset = 2; // Skip reserved

        // Parse queue, exchange, routing_key (similar to bind)
        let queue_len = arguments.get(offset).copied().unwrap_or(0) as usize;
        offset += 1 + queue_len;
        let exchange_len = arguments.get(offset).copied().unwrap_or(0) as usize;
        offset += 1 + exchange_len;
        let _routing_key_len = arguments.get(offset).copied().unwrap_or(0) as usize;

        // Remove binding (simplified)
        let mut payload = Vec::new();
        payload.extend_from_slice(&class_id::QUEUE.to_be_bytes());
        payload.extend_from_slice(&method_id::QUEUE_UNBIND_OK.to_be_bytes());
        self.send_method_frame(channel, &payload).await?;

        Ok(true)
    }

    async fn handle_queue_delete(&mut self, channel: u16, arguments: &[u8]) -> io::Result<bool> {
        let mut offset = 2; // Skip reserved

        let name_len = arguments.get(offset).copied().unwrap_or(0) as usize;
        offset += 1;
        let queue_name = if offset + name_len <= arguments.len() {
            String::from_utf8_lossy(&arguments[offset..offset + name_len]).to_string()
        } else {
            String::new()
        };
        offset += name_len;

        let flags = arguments.get(offset).copied().unwrap_or(0);
        let no_wait = flags & 0x04 != 0;

        tracing::debug!("Queue delete: {}", queue_name);

        let message_count = {
            let mut queues = self.queues.write().await;
            let count = queues.get_queue(&queue_name).map(|q| q.messages.len() as u32).unwrap_or(0);
            queues.delete_queue(&queue_name);
            self.metrics.record_queue_deleted();
            count
        };

        if !no_wait {
            let mut payload = Vec::new();
            payload.extend_from_slice(&class_id::QUEUE.to_be_bytes());
            payload.extend_from_slice(&method_id::QUEUE_DELETE_OK.to_be_bytes());
            payload.extend_from_slice(&message_count.to_be_bytes());
            self.send_method_frame(channel, &payload).await?;
        }

        Ok(true)
    }

    async fn handle_queue_purge(&mut self, channel: u16, arguments: &[u8]) -> io::Result<bool> {
        let mut offset = 2;

        let name_len = arguments.get(offset).copied().unwrap_or(0) as usize;
        offset += 1;
        let queue_name = if offset + name_len <= arguments.len() {
            String::from_utf8_lossy(&arguments[offset..offset + name_len]).to_string()
        } else {
            String::new()
        };
        offset += name_len;

        let flags = arguments.get(offset).copied().unwrap_or(0);
        let no_wait = flags & 0x01 != 0;

        let message_count = {
            let mut queues = self.queues.write().await;
            if let Some(queue) = queues.get_queue_mut(&queue_name) {
                let count = queue.messages.len() as u32;
                queue.messages.clear();
                count
            } else {
                0
            }
        };

        if !no_wait {
            let mut payload = Vec::new();
            payload.extend_from_slice(&class_id::QUEUE.to_be_bytes());
            payload.extend_from_slice(&0x1Fu16.to_be_bytes()); // Purge-Ok
            payload.extend_from_slice(&message_count.to_be_bytes());
            self.send_method_frame(channel, &payload).await?;
        }

        Ok(true)
    }

    /// Handle basic class methods
    async fn handle_basic_method(
        &mut self,
        channel: u16,
        method_id: u16,
        arguments: &[u8],
    ) -> io::Result<bool> {
        match method_id {
            method_id::BASIC_QOS => self.handle_basic_qos(channel, arguments).await,
            method_id::BASIC_CONSUME => self.handle_basic_consume(channel, arguments).await,
            0x1E /* Cancel */ => self.handle_basic_cancel(channel, arguments).await,
            method_id::BASIC_PUBLISH => self.handle_basic_publish(channel, arguments).await,
            method_id::BASIC_GET => self.handle_basic_get(channel, arguments).await,
            method_id::BASIC_ACK => self.handle_basic_ack(channel, arguments).await,
            0x5A /* Reject */ => self.handle_basic_reject(channel, arguments).await,
            method_id::BASIC_NACK => self.handle_basic_nack(channel, arguments).await,
            0x6E /* Recover */ => self.handle_basic_recover(channel, arguments).await,
            _ => {
                tracing::warn!("Unknown basic method: {}", method_id);
                Ok(true)
            }
        }
    }

    async fn handle_basic_qos(&mut self, channel: u16, arguments: &[u8]) -> io::Result<bool> {
        let prefetch_size = if arguments.len() >= 4 {
            u32::from_be_bytes([arguments[0], arguments[1], arguments[2], arguments[3]])
        } else {
            0
        };

        let prefetch_count = if arguments.len() >= 6 {
            u16::from_be_bytes([arguments[4], arguments[5]])
        } else {
            0
        };

        let global = arguments.get(6).map(|&b| b != 0).unwrap_or(false);

        if let Some(ch) = self.channels.get_mut(&channel) {
            ch.prefetch_size = prefetch_size;
            ch.prefetch_count = prefetch_count;
            ch.global_prefetch = global;
        }

        let mut payload = Vec::new();
        payload.extend_from_slice(&class_id::BASIC.to_be_bytes());
        payload.extend_from_slice(&method_id::BASIC_QOS_OK.to_be_bytes());
        self.send_method_frame(channel, &payload).await?;

        Ok(true)
    }

    async fn handle_basic_consume(&mut self, channel: u16, arguments: &[u8]) -> io::Result<bool> {
        let mut offset = 2; // Skip reserved

        // Queue name
        let queue_len = arguments.get(offset).copied().unwrap_or(0) as usize;
        offset += 1;
        let queue_name = if offset + queue_len <= arguments.len() {
            String::from_utf8_lossy(&arguments[offset..offset + queue_len]).to_string()
        } else {
            String::new()
        };
        offset += queue_len;

        // Consumer tag
        let tag_len = arguments.get(offset).copied().unwrap_or(0) as usize;
        offset += 1;
        let consumer_tag = if tag_len > 0 && offset + tag_len <= arguments.len() {
            String::from_utf8_lossy(&arguments[offset..offset + tag_len]).to_string()
        } else {
            // Generate consumer tag
            self.channels
                .get_mut(&channel)
                .map(|ch| ch.generate_consumer_tag())
                .unwrap_or_else(|| format!("amq.ctag-{}", generate_id()))
        };
        offset += tag_len;

        // Flags
        let flags = arguments.get(offset).copied().unwrap_or(0);
        let _no_local = flags & 0x01 != 0;
        let no_ack = flags & 0x02 != 0;
        let _exclusive = flags & 0x04 != 0;
        let no_wait = flags & 0x08 != 0;

        tracing::debug!(
            "Basic.Consume: queue={}, tag={}, no_ack={}",
            queue_name,
            consumer_tag,
            no_ack
        );

        // Get prefetch from channel
        let prefetch_count = self.channels.get(&channel).map(|ch| ch.prefetch_count).unwrap_or(0);

        // Create consumer
        let consumer =
            Consumer::new(consumer_tag.clone(), queue_name.clone(), no_ack, prefetch_count);

        // Add consumer to channel
        if let Some(ch) = self.channels.get_mut(&channel) {
            ch.consumers.insert(consumer_tag.clone(), consumer);
        }

        // Add consumer tag to queue
        {
            let mut queues = self.queues.write().await;
            if let Some(queue) = queues.get_queue_mut(&queue_name) {
                queue.consumers.push(consumer_tag.clone());
            }
        }

        // Send Basic.Consume-Ok
        if !no_wait {
            let mut payload = Vec::new();
            payload.extend_from_slice(&class_id::BASIC.to_be_bytes());
            payload.extend_from_slice(&method_id::BASIC_CONSUME_OK.to_be_bytes());
            payload.push(consumer_tag.len() as u8);
            payload.extend_from_slice(consumer_tag.as_bytes());
            self.send_method_frame(channel, &payload).await?;
        }

        // Immediately try to deliver any pending messages to this consumer
        if let Err(e) = self.deliver_to_consumers(&queue_name).await {
            tracing::warn!("Failed to deliver initial messages to consumer: {}", e);
        }

        Ok(true)
    }

    async fn handle_basic_cancel(&mut self, channel: u16, arguments: &[u8]) -> io::Result<bool> {
        let mut offset = 0;

        let tag_len = arguments.get(offset).copied().unwrap_or(0) as usize;
        offset += 1;
        let consumer_tag = if offset + tag_len <= arguments.len() {
            String::from_utf8_lossy(&arguments[offset..offset + tag_len]).to_string()
        } else {
            String::new()
        };
        offset += tag_len;

        let flags = arguments.get(offset).copied().unwrap_or(0);
        let no_wait = flags & 0x01 != 0;

        // Remove consumer from channel
        if let Some(ch) = self.channels.get_mut(&channel) {
            ch.consumers.remove(&consumer_tag);
        }

        if !no_wait {
            let mut payload = Vec::new();
            payload.extend_from_slice(&class_id::BASIC.to_be_bytes());
            payload.extend_from_slice(&0x1Fu16.to_be_bytes()); // Cancel-Ok
            payload.push(consumer_tag.len() as u8);
            payload.extend_from_slice(consumer_tag.as_bytes());
            self.send_method_frame(channel, &payload).await?;
        }

        Ok(true)
    }

    async fn handle_basic_publish(&mut self, channel: u16, arguments: &[u8]) -> io::Result<bool> {
        let mut offset = 2; // Skip reserved

        // Exchange name
        let exchange_len = arguments.get(offset).copied().unwrap_or(0) as usize;
        offset += 1;
        let exchange = if offset + exchange_len <= arguments.len() {
            String::from_utf8_lossy(&arguments[offset..offset + exchange_len]).to_string()
        } else {
            String::new()
        };
        offset += exchange_len;

        // Routing key
        let routing_key_len = arguments.get(offset).copied().unwrap_or(0) as usize;
        offset += 1;
        let routing_key = if offset + routing_key_len <= arguments.len() {
            String::from_utf8_lossy(&arguments[offset..offset + routing_key_len]).to_string()
        } else {
            String::new()
        };
        offset += routing_key_len;

        // Flags
        let flags = arguments.get(offset).copied().unwrap_or(0);
        let mandatory = flags & 0x01 != 0;
        let immediate = flags & 0x02 != 0;

        tracing::debug!("Basic.Publish: exchange={}, routing_key={}", exchange, routing_key);

        // Set up content state for header and body frames
        self.content_state = Some((
            channel,
            ContentState {
                class_id: class_id::BASIC,
                body_size: 0,
                properties: MessageProperties::default(),
                body: Vec::new(),
                exchange,
                routing_key,
                mandatory,
                immediate,
            },
        ));

        Ok(true)
    }

    async fn handle_basic_get(&mut self, channel: u16, arguments: &[u8]) -> io::Result<bool> {
        let mut offset = 2; // Skip reserved

        let queue_len = arguments.get(offset).copied().unwrap_or(0) as usize;
        offset += 1;
        let queue_name = if offset + queue_len <= arguments.len() {
            String::from_utf8_lossy(&arguments[offset..offset + queue_len]).to_string()
        } else {
            String::new()
        };
        offset += queue_len;

        let flags = arguments.get(offset).copied().unwrap_or(0);
        let no_ack = flags & 0x01 != 0;

        tracing::debug!("Basic.Get: queue={}, no_ack={}", queue_name, no_ack);

        // Try to get a message from the queue
        let message_opt = {
            let mut queues = self.queues.write().await;
            queues.get_queue_mut(&queue_name).and_then(|q| q.dequeue())
        };

        if let Some(queued_msg) = message_opt {
            // Get delivery tag
            let delivery_tag =
                self.channels.get_mut(&channel).map(|ch| ch.next_delivery_tag()).unwrap_or(1);

            // Track unacked message if not no_ack
            if !no_ack {
                if let Some(ch) = self.channels.get_mut(&channel) {
                    ch.unacked_messages.insert(
                        delivery_tag,
                        UnackedMessage {
                            delivery_tag,
                            queue_name: queue_name.clone(),
                            message: queued_msg.message.clone(),
                            redelivered: queued_msg.delivery_count > 0,
                        },
                    );
                }
            }

            // Get remaining message count
            let message_count = {
                let queues = self.queues.read().await;
                queues.get_queue(&queue_name).map(|q| q.messages.len() as u32).unwrap_or(0)
            };

            // Send Basic.Get-Ok
            let mut payload = Vec::new();
            payload.extend_from_slice(&class_id::BASIC.to_be_bytes());
            payload.extend_from_slice(&method_id::BASIC_GET_OK.to_be_bytes());
            payload.extend_from_slice(&delivery_tag.to_be_bytes());
            payload.push(if queued_msg.delivery_count > 0 { 1 } else { 0 }); // redelivered
                                                                             // Exchange (short string)
            payload.push(0); // empty exchange
                             // Routing key
            payload.push(queued_msg.message.routing_key.len() as u8);
            payload.extend_from_slice(queued_msg.message.routing_key.as_bytes());
            // Message count
            payload.extend_from_slice(&message_count.to_be_bytes());

            self.send_method_frame(channel, &payload).await?;

            // Send content header and body
            self.send_content(channel, &queued_msg.message).await?;

            self.metrics.record_consume();
        } else {
            // Send Basic.Get-Empty
            let mut payload = Vec::new();
            payload.extend_from_slice(&class_id::BASIC.to_be_bytes());
            payload.extend_from_slice(&method_id::BASIC_GET_EMPTY.to_be_bytes());
            // Cluster-id (deprecated, empty string)
            payload.push(0);

            self.send_method_frame(channel, &payload).await?;
        }

        Ok(true)
    }

    async fn handle_basic_ack(&mut self, channel: u16, arguments: &[u8]) -> io::Result<bool> {
        let delivery_tag = if arguments.len() >= 8 {
            u64::from_be_bytes([
                arguments[0],
                arguments[1],
                arguments[2],
                arguments[3],
                arguments[4],
                arguments[5],
                arguments[6],
                arguments[7],
            ])
        } else {
            return Ok(true);
        };

        let multiple = arguments.get(8).map(|&b| b & 0x01 != 0).unwrap_or(false);

        tracing::debug!("Basic.Ack: delivery_tag={}, multiple={}", delivery_tag, multiple);

        if let Some(ch) = self.channels.get_mut(&channel) {
            if ch.transaction_mode {
                ch.tx_pending_acks.push(delivery_tag);
            } else if multiple {
                ch.unacked_messages.retain(|&tag, _| tag > delivery_tag);
            } else {
                ch.unacked_messages.remove(&delivery_tag);
            }
        }

        self.metrics.record_ack();
        Ok(true)
    }

    async fn handle_basic_reject(&mut self, channel: u16, arguments: &[u8]) -> io::Result<bool> {
        let delivery_tag = if arguments.len() >= 8 {
            u64::from_be_bytes([
                arguments[0],
                arguments[1],
                arguments[2],
                arguments[3],
                arguments[4],
                arguments[5],
                arguments[6],
                arguments[7],
            ])
        } else {
            return Ok(true);
        };

        let requeue = arguments.get(8).map(|&b| b & 0x01 != 0).unwrap_or(false);

        tracing::debug!("Basic.Reject: delivery_tag={}, requeue={}", delivery_tag, requeue);

        // Handle requeue if requested
        if requeue {
            if let Some(ch) = self.channels.get_mut(&channel) {
                if let Some(unacked) = ch.unacked_messages.remove(&delivery_tag) {
                    let mut queues = self.queues.write().await;
                    if let Some(queue) = queues.get_queue_mut(&unacked.queue_name) {
                        let msg = unacked.message;
                        // Mark as redelivered somehow - we'd need to track this
                        queue.messages.push_front(QueuedMessage::new(msg));
                    }
                }
            }
        } else if let Some(ch) = self.channels.get_mut(&channel) {
            ch.unacked_messages.remove(&delivery_tag);
        }

        self.metrics.record_reject();
        Ok(true)
    }

    async fn handle_basic_nack(&mut self, channel: u16, arguments: &[u8]) -> io::Result<bool> {
        let delivery_tag = if arguments.len() >= 8 {
            u64::from_be_bytes([
                arguments[0],
                arguments[1],
                arguments[2],
                arguments[3],
                arguments[4],
                arguments[5],
                arguments[6],
                arguments[7],
            ])
        } else {
            return Ok(true);
        };

        let flags = arguments.get(8).copied().unwrap_or(0);
        let multiple = flags & 0x01 != 0;
        let requeue = flags & 0x02 != 0;

        tracing::debug!(
            "Basic.Nack: delivery_tag={}, multiple={}, requeue={}",
            delivery_tag,
            multiple,
            requeue
        );

        // Similar to reject but can be multiple
        if let Some(ch) = self.channels.get_mut(&channel) {
            let tags_to_nack: Vec<u64> = if multiple {
                ch.unacked_messages
                    .keys()
                    .filter(|&&tag| tag <= delivery_tag)
                    .copied()
                    .collect()
            } else {
                vec![delivery_tag]
            };

            for tag in tags_to_nack {
                if let Some(unacked) = ch.unacked_messages.remove(&tag) {
                    if requeue {
                        let mut queues = self.queues.write().await;
                        if let Some(queue) = queues.get_queue_mut(&unacked.queue_name) {
                            queue.messages.push_front(QueuedMessage::new(unacked.message));
                        }
                    }
                }
                self.metrics.record_reject();
            }
        }

        Ok(true)
    }

    async fn handle_basic_recover(&mut self, channel: u16, arguments: &[u8]) -> io::Result<bool> {
        let requeue = arguments.first().map(|&b| b & 0x01 != 0).unwrap_or(true);

        tracing::debug!("Basic.Recover: requeue={}", requeue);

        // Redeliver all unacked messages
        if let Some(ch) = self.channels.get_mut(&channel) {
            if requeue {
                let messages: Vec<_> = ch.unacked_messages.drain().collect();
                for (_, unacked) in messages {
                    let mut queues = self.queues.write().await;
                    if let Some(queue) = queues.get_queue_mut(&unacked.queue_name) {
                        queue.messages.push_front(QueuedMessage::new(unacked.message));
                    }
                }
            }
        }

        // Send Recover-Ok
        let mut payload = Vec::new();
        payload.extend_from_slice(&class_id::BASIC.to_be_bytes());
        payload.extend_from_slice(&0x6Fu16.to_be_bytes()); // Recover-Ok
        self.send_method_frame(channel, &payload).await?;

        Ok(true)
    }

    /// Handle tx class methods
    async fn handle_tx_method(
        &mut self,
        channel: u16,
        method_id: u16,
        _arguments: &[u8],
    ) -> io::Result<bool> {
        match method_id {
            method_id::TX_SELECT => {
                if let Some(ch) = self.channels.get_mut(&channel) {
                    ch.transaction_mode = true;
                }

                let mut payload = Vec::new();
                payload.extend_from_slice(&class_id::TX.to_be_bytes());
                payload.extend_from_slice(&method_id::TX_SELECT_OK.to_be_bytes());
                self.send_method_frame(channel, &payload).await?;
                Ok(true)
            }
            method_id::TX_COMMIT => {
                // Extract pending publishes and acks before routing
                let (pending_publishes, pending_acks) =
                    if let Some(ch) = self.channels.get_mut(&channel) {
                        let publishes: Vec<_> = ch.tx_pending_publishes.drain(..).collect();
                        let acks: Vec<_> = ch.tx_pending_acks.drain(..).collect();
                        (publishes, acks)
                    } else {
                        (Vec::new(), Vec::new())
                    };

                // Route pending messages
                for (exchange, routing_key, message) in pending_publishes {
                    self.route_message(&exchange, &routing_key, message).await;
                }

                // Process pending acks
                if let Some(ch) = self.channels.get_mut(&channel) {
                    for tag in pending_acks {
                        ch.unacked_messages.remove(&tag);
                    }
                }

                let mut payload = Vec::new();
                payload.extend_from_slice(&class_id::TX.to_be_bytes());
                payload.extend_from_slice(&method_id::TX_COMMIT_OK.to_be_bytes());
                self.send_method_frame(channel, &payload).await?;
                Ok(true)
            }
            method_id::TX_ROLLBACK => {
                if let Some(ch) = self.channels.get_mut(&channel) {
                    // Discard pending publishes
                    ch.tx_pending_publishes.clear();

                    // Discard pending acks (messages stay unacked)
                    ch.tx_pending_acks.clear();
                }

                let mut payload = Vec::new();
                payload.extend_from_slice(&class_id::TX.to_be_bytes());
                payload.extend_from_slice(&method_id::TX_ROLLBACK_OK.to_be_bytes());
                self.send_method_frame(channel, &payload).await?;
                Ok(true)
            }
            _ => {
                tracing::warn!("Unknown tx method: {}", method_id);
                Ok(true)
            }
        }
    }

    /// Handle confirm class methods
    async fn handle_confirm_method(
        &mut self,
        channel: u16,
        method_id: u16,
        _arguments: &[u8],
    ) -> io::Result<bool> {
        match method_id {
            method_id::CONFIRM_SELECT => {
                if let Some(ch) = self.channels.get_mut(&channel) {
                    ch.publisher_confirms = true;
                }

                let mut payload = Vec::new();
                payload.extend_from_slice(&class_id::CONFIRM.to_be_bytes());
                payload.extend_from_slice(&method_id::CONFIRM_SELECT_OK.to_be_bytes());
                self.send_method_frame(channel, &payload).await?;
                Ok(true)
            }
            _ => {
                tracing::warn!("Unknown confirm method: {}", method_id);
                Ok(true)
            }
        }
    }

    /// Handle content header frame
    async fn handle_header_frame(&mut self, channel: u16, payload: &[u8]) -> io::Result<bool> {
        if payload.len() < 14 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Header frame too short"));
        }

        let class_id = u16::from_be_bytes([payload[0], payload[1]]);
        // weight is deprecated, skip it (bytes 2-3)
        let body_size = u64::from_be_bytes([
            payload[4],
            payload[5],
            payload[6],
            payload[7],
            payload[8],
            payload[9],
            payload[10],
            payload[11],
        ]);

        // Parse property flags and values
        let properties = self.parse_basic_properties(&payload[12..]);

        if let Some((ch, state)) = &mut self.content_state {
            if *ch == channel && state.class_id == class_id {
                state.body_size = body_size;
                state.properties = properties;
                state.body.reserve(body_size as usize);
            }
        }

        Ok(true)
    }

    /// Parse Basic content properties
    fn parse_basic_properties(&self, payload: &[u8]) -> MessageProperties {
        let mut props = MessageProperties::default();

        if payload.len() < 2 {
            return props;
        }

        let flags = u16::from_be_bytes([payload[0], payload[1]]);
        let mut offset = 2;

        // Content-type (bit 15)
        if flags & 0x8000 != 0 {
            if let Some((s, new_offset)) = self.read_short_string(payload, offset) {
                props.content_type = Some(s);
                offset = new_offset;
            }
        }

        // Content-encoding (bit 14)
        if flags & 0x4000 != 0 {
            if let Some((s, new_offset)) = self.read_short_string(payload, offset) {
                props.content_encoding = Some(s);
                offset = new_offset;
            }
        }

        // Headers (bit 13) - field table
        if flags & 0x2000 != 0 {
            if offset + 4 <= payload.len() {
                let table_len = u32::from_be_bytes([
                    payload[offset],
                    payload[offset + 1],
                    payload[offset + 2],
                    payload[offset + 3],
                ]) as usize;
                offset += 4;
                // Parse headers (simplified - just skip for now)
                offset += table_len;
            }
        }

        // Delivery-mode (bit 12)
        if flags & 0x1000 != 0 {
            if offset < payload.len() {
                let mode = payload[offset];
                props.delivery_mode = if mode == 2 {
                    crate::messages::DeliveryMode::Persistent
                } else {
                    crate::messages::DeliveryMode::NonPersistent
                };
                offset += 1;
            }
        }

        // Priority (bit 11)
        if flags & 0x0800 != 0 {
            if offset < payload.len() {
                props.priority = payload[offset];
                offset += 1;
            }
        }

        // Correlation-id (bit 10)
        if flags & 0x0400 != 0 {
            if let Some((s, new_offset)) = self.read_short_string(payload, offset) {
                props.correlation_id = Some(s);
                offset = new_offset;
            }
        }

        // Reply-to (bit 9)
        if flags & 0x0200 != 0 {
            if let Some((s, new_offset)) = self.read_short_string(payload, offset) {
                props.reply_to = Some(s);
                offset = new_offset;
            }
        }

        // Expiration (bit 8)
        if flags & 0x0100 != 0 {
            if let Some((s, new_offset)) = self.read_short_string(payload, offset) {
                props.expiration = Some(s);
                offset = new_offset;
            }
        }

        // Message-id (bit 7)
        if flags & 0x0080 != 0 {
            if let Some((s, new_offset)) = self.read_short_string(payload, offset) {
                props.message_id = Some(s);
                offset = new_offset;
            }
        }

        // Timestamp (bit 6)
        if flags & 0x0040 != 0 {
            if offset + 8 <= payload.len() {
                props.timestamp = Some(i64::from_be_bytes([
                    payload[offset],
                    payload[offset + 1],
                    payload[offset + 2],
                    payload[offset + 3],
                    payload[offset + 4],
                    payload[offset + 5],
                    payload[offset + 6],
                    payload[offset + 7],
                ]));
                offset += 8;
            }
        }

        // Type (bit 5)
        if flags & 0x0020 != 0 {
            if let Some((s, new_offset)) = self.read_short_string(payload, offset) {
                props.type_field = Some(s);
                offset = new_offset;
            }
        }

        // User-id (bit 4)
        if flags & 0x0010 != 0 {
            if let Some((s, new_offset)) = self.read_short_string(payload, offset) {
                props.user_id = Some(s);
                offset = new_offset;
            }
        }

        // App-id (bit 3)
        if flags & 0x0008 != 0 {
            if let Some((s, _)) = self.read_short_string(payload, offset) {
                props.app_id = Some(s);
            }
        }

        props
    }

    fn read_short_string(&self, payload: &[u8], offset: usize) -> Option<(String, usize)> {
        if offset >= payload.len() {
            return None;
        }
        let len = payload[offset] as usize;
        if offset + 1 + len > payload.len() {
            return None;
        }
        let s = String::from_utf8_lossy(&payload[offset + 1..offset + 1 + len]).to_string();
        Some((s, offset + 1 + len))
    }

    /// Handle content body frame
    async fn handle_body_frame(&mut self, channel: u16, payload: &[u8]) -> io::Result<bool> {
        let should_publish = if let Some((ch, state)) = &mut self.content_state {
            if *ch == channel {
                state.body.extend_from_slice(payload);
                state.body.len() as u64 >= state.body_size
            } else {
                false
            }
        } else {
            false
        };

        if should_publish {
            if let Some((ch, state)) = self.content_state.take() {
                let message = Message {
                    properties: state.properties,
                    body: state.body,
                    routing_key: state.routing_key.clone(),
                };

                // Check if in transaction mode
                let in_tx = self.channels.get(&ch).map(|c| c.transaction_mode).unwrap_or(false);

                if in_tx {
                    if let Some(channel_state) = self.channels.get_mut(&ch) {
                        channel_state.tx_pending_publishes.push((
                            state.exchange.clone(),
                            state.routing_key.clone(),
                            message,
                        ));
                    }
                } else {
                    // Route immediately
                    self.route_message(&state.exchange, &state.routing_key, message.clone()).await;

                    // Send publisher confirm if enabled
                    if let Some(channel_state) = self.channels.get_mut(&ch) {
                        if channel_state.publisher_confirms {
                            let delivery_tag = channel_state.next_delivery_tag();
                            self.send_basic_ack(ch, delivery_tag, false).await?;
                        }
                    }
                }

                self.metrics.record_publish();
            }
        }

        Ok(true)
    }

    /// Route a message through exchanges to queues
    async fn route_message(&mut self, exchange_name: &str, routing_key: &str, message: Message) {
        let target_queues = {
            let exchanges = self.exchanges.read().await;

            if exchange_name.is_empty() {
                // Default exchange - route directly to queue named by routing_key
                vec![routing_key.to_string()]
            } else if let Some(exchange) = exchanges.get_exchange(exchange_name) {
                exchange.route_message(&message, routing_key)
            } else {
                vec![]
            }
        };

        // Deliver to queues and notify consumers
        let mut queues = self.queues.write().await;
        for queue_name in target_queues {
            let queued = QueuedMessage::new(message.clone());
            if let Err(e) = queues.enqueue_and_notify(&queue_name, queued) {
                tracing::warn!("Failed to enqueue message to {}: {}", queue_name, e);
            }
        }
    }

    /// Send a Basic.Ack frame (for publisher confirms)
    async fn send_basic_ack(
        &mut self,
        channel: u16,
        delivery_tag: u64,
        multiple: bool,
    ) -> io::Result<()> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&class_id::BASIC.to_be_bytes());
        payload.extend_from_slice(&method_id::BASIC_ACK.to_be_bytes());
        payload.extend_from_slice(&delivery_tag.to_be_bytes());
        payload.push(if multiple { 1 } else { 0 });

        self.send_method_frame(channel, &payload).await
    }

    /// Send content (header + body frames) for a message
    async fn send_content(&mut self, channel: u16, message: &Message) -> io::Result<()> {
        // Send header frame
        let header_payload = self.build_content_header(message);
        let header_frame = Frame {
            frame_type: FrameType::Header,
            channel,
            payload: header_payload,
        };
        header_frame.write_to_stream(&mut self.stream).await?;

        // Send body frame(s)
        let max_body_size = (self.frame_max - 8) as usize; // Frame overhead
        for chunk in message.body.chunks(max_body_size) {
            let body_frame = Frame {
                frame_type: FrameType::Body,
                channel,
                payload: chunk.to_vec(),
            };
            body_frame.write_to_stream(&mut self.stream).await?;
        }

        Ok(())
    }

    /// Deliver messages from a queue to all consumers on this connection
    async fn deliver_to_consumers(&mut self, queue_name: &str) -> io::Result<()> {
        // Collect consumers interested in this queue
        let mut deliveries: Vec<(u16, String, bool, u16)> = Vec::new(); // (channel, consumer_tag, no_ack, prefetch)

        for (channel_id, channel) in &self.channels {
            if *channel_id == 0 {
                continue; // Skip channel 0
            }
            for (consumer_tag, consumer) in &channel.consumers {
                if consumer.queue == queue_name {
                    deliveries.push((
                        *channel_id,
                        consumer_tag.clone(),
                        consumer.no_ack,
                        consumer.prefetch_count,
                    ));
                }
            }
        }

        if deliveries.is_empty() {
            return Ok(());
        }

        // For each consumer, try to deliver messages
        for (channel_id, consumer_tag, no_ack, prefetch_count) in deliveries {
            // Check prefetch limit
            let unacked_count =
                self.channels.get(&channel_id).map(|ch| ch.unacked_messages.len()).unwrap_or(0);

            if prefetch_count > 0 && unacked_count >= prefetch_count as usize {
                tracing::debug!(
                    "Consumer {} on channel {} at prefetch limit",
                    consumer_tag,
                    channel_id
                );
                continue;
            }

            // Try to get a message from the queue
            let message_opt = {
                let mut queues = self.queues.write().await;
                queues.get_queue_mut(queue_name).and_then(|q| q.dequeue())
            };

            if let Some(queued_msg) = message_opt {
                // Get delivery tag
                let delivery_tag = self
                    .channels
                    .get_mut(&channel_id)
                    .map(|ch| ch.next_delivery_tag())
                    .unwrap_or(1);

                // Track unacked message if not no_ack
                if !no_ack {
                    if let Some(ch) = self.channels.get_mut(&channel_id) {
                        ch.unacked_messages.insert(
                            delivery_tag,
                            UnackedMessage {
                                delivery_tag,
                                queue_name: queue_name.to_string(),
                                message: queued_msg.message.clone(),
                                redelivered: queued_msg.delivery_count > 0,
                            },
                        );
                    }
                }

                // Send Basic.Deliver
                let mut payload = Vec::new();
                payload.extend_from_slice(&class_id::BASIC.to_be_bytes());
                payload.extend_from_slice(&method_id::BASIC_DELIVER.to_be_bytes());
                // Consumer tag (short string)
                payload.push(consumer_tag.len() as u8);
                payload.extend_from_slice(consumer_tag.as_bytes());
                // Delivery tag (8 bytes)
                payload.extend_from_slice(&delivery_tag.to_be_bytes());
                // Redelivered flag
                payload.push(if queued_msg.delivery_count > 0 { 1 } else { 0 });
                // Exchange (short string) - use empty for default
                payload.push(0);
                // Routing key (short string)
                payload.push(queued_msg.message.routing_key.len() as u8);
                payload.extend_from_slice(queued_msg.message.routing_key.as_bytes());

                self.send_method_frame(channel_id, &payload).await?;

                // Send content header and body
                self.send_content(channel_id, &queued_msg.message).await?;

                self.metrics.record_consume();

                tracing::debug!(
                    "Delivered message to consumer {} on channel {} with delivery_tag={}",
                    consumer_tag,
                    channel_id,
                    delivery_tag
                );
            }
        }

        Ok(())
    }

    fn build_content_header(&self, message: &Message) -> Vec<u8> {
        let mut payload = Vec::new();

        // Class ID (Basic = 60)
        payload.extend_from_slice(&class_id::BASIC.to_be_bytes());
        // Weight (deprecated)
        payload.extend_from_slice(&0u16.to_be_bytes());
        // Body size
        payload.extend_from_slice(&(message.body.len() as u64).to_be_bytes());

        // Property flags and values
        let mut flags: u16 = 0;
        let mut props_data = Vec::new();

        if message.properties.content_type.is_some() {
            flags |= 0x8000;
            let ct = message.properties.content_type.as_ref().unwrap();
            props_data.push(ct.len() as u8);
            props_data.extend_from_slice(ct.as_bytes());
        }

        if message.properties.content_encoding.is_some() {
            flags |= 0x4000;
            let ce = message.properties.content_encoding.as_ref().unwrap();
            props_data.push(ce.len() as u8);
            props_data.extend_from_slice(ce.as_bytes());
        }

        // Headers (bit 13) - skip for now
        if !message.properties.headers.is_empty() {
            flags |= 0x2000;
            // Empty table for now
            props_data.extend_from_slice(&0u32.to_be_bytes());
        }

        // Delivery mode
        flags |= 0x1000;
        props_data.push(message.properties.delivery_mode.clone() as u8);

        // Priority
        if message.properties.priority > 0 {
            flags |= 0x0800;
            props_data.push(message.properties.priority);
        }

        if let Some(ref cid) = message.properties.correlation_id {
            flags |= 0x0400;
            props_data.push(cid.len() as u8);
            props_data.extend_from_slice(cid.as_bytes());
        }

        if let Some(ref rt) = message.properties.reply_to {
            flags |= 0x0200;
            props_data.push(rt.len() as u8);
            props_data.extend_from_slice(rt.as_bytes());
        }

        if let Some(ref exp) = message.properties.expiration {
            flags |= 0x0100;
            props_data.push(exp.len() as u8);
            props_data.extend_from_slice(exp.as_bytes());
        }

        if let Some(ref mid) = message.properties.message_id {
            flags |= 0x0080;
            props_data.push(mid.len() as u8);
            props_data.extend_from_slice(mid.as_bytes());
        }

        if let Some(ts) = message.properties.timestamp {
            flags |= 0x0040;
            props_data.extend_from_slice(&ts.to_be_bytes());
        }

        if let Some(ref t) = message.properties.type_field {
            flags |= 0x0020;
            props_data.push(t.len() as u8);
            props_data.extend_from_slice(t.as_bytes());
        }

        if let Some(ref uid) = message.properties.user_id {
            flags |= 0x0010;
            props_data.push(uid.len() as u8);
            props_data.extend_from_slice(uid.as_bytes());
        }

        if let Some(ref aid) = message.properties.app_id {
            flags |= 0x0008;
            props_data.push(aid.len() as u8);
            props_data.extend_from_slice(aid.as_bytes());
        }

        payload.extend_from_slice(&flags.to_be_bytes());
        payload.extend_from_slice(&props_data);

        payload
    }

    /// Send a method frame
    async fn send_method_frame(&mut self, channel: u16, payload: &[u8]) -> io::Result<()> {
        let frame = Frame {
            frame_type: FrameType::Method,
            channel,
            payload: payload.to_vec(),
        };
        frame.write_to_stream(&mut self.stream).await
    }

    /// Send a heartbeat frame
    async fn send_heartbeat(&mut self) -> io::Result<()> {
        let frame = Frame {
            frame_type: FrameType::Heartbeat,
            channel: 0,
            payload: vec![],
        };
        frame.write_to_stream(&mut self.stream).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_state_transitions() {
        assert_eq!(ConnectionState::Start, ConnectionState::Start);
        assert_ne!(ConnectionState::Start, ConnectionState::Open);
    }

    #[test]
    fn test_channel_state() {
        assert_eq!(ChannelState::Closed, ChannelState::Closed);
        assert_ne!(ChannelState::Closed, ChannelState::Open);
    }

    #[test]
    fn test_channel_new() {
        let channel = Channel::new(1);
        assert_eq!(channel.id, 1);
        assert_eq!(channel.state, ChannelState::Open);
        assert!(channel.consumers.is_empty());
        assert_eq!(channel.prefetch_count, 0);
        assert!(!channel.publisher_confirms);
        assert!(!channel.transaction_mode);
    }

    #[test]
    fn test_channel_delivery_tag() {
        let mut channel = Channel::new(1);
        assert_eq!(channel.next_delivery_tag(), 1);
        assert_eq!(channel.next_delivery_tag(), 2);
        assert_eq!(channel.next_delivery_tag(), 3);
    }

    #[test]
    fn test_channel_consumer_tag() {
        let mut channel = Channel::new(5);
        let tag1 = channel.generate_consumer_tag();
        let tag2 = channel.generate_consumer_tag();
        assert!(tag1.contains("5"));
        assert!(tag2.contains("5"));
        assert_ne!(tag1, tag2);
    }

    #[test]
    fn test_unacked_message() {
        let msg = UnackedMessage {
            delivery_tag: 42,
            queue_name: "test-queue".to_string(),
            message: Message {
                properties: MessageProperties::default(),
                body: vec![1, 2, 3],
                routing_key: "test.key".to_string(),
            },
            redelivered: false,
        };

        assert_eq!(msg.delivery_tag, 42);
        assert_eq!(msg.queue_name, "test-queue");
        assert!(!msg.redelivered);
    }
}
