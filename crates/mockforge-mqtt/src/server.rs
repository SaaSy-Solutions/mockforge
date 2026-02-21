//! MQTT server implementation using tokio TCP listener
//!
//! This module provides a complete MQTT 3.1.1 broker implementation that:
//! - Accepts TCP connections from MQTT clients
//! - Parses and handles all MQTT control packets
//! - Manages client sessions and subscriptions
//! - Routes messages between publishers and subscribers
//! - Supports QoS 0, 1, and 2 message delivery

use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_rustls::server::TlsStream;
use tracing::{debug, error, info, warn};

use crate::broker::MqttConfig;
use crate::metrics::MqttMetrics;
use crate::protocol::{ConnackCode, Packet, PacketDecoder, PacketEncoder, QoS};
use crate::session::{
    build_connack, build_puback, build_pubcomp, build_pubrec, build_pubrel, build_suback,
    build_unsuback, SessionManager,
};
use crate::tls::{create_tls_acceptor_with_client_auth, TlsError};

/// Maximum buffer size for reading packets
const READ_BUFFER_SIZE: usize = 64 * 1024; // 64KB
/// Channel capacity for outgoing packets per client
const CLIENT_CHANNEL_CAPACITY: usize = 256;
/// Session cleanup interval in seconds
const CLEANUP_INTERVAL_SECS: u64 = 30;

/// Stream type that can be either plain TCP or TLS-encrypted
pub enum MqttStream {
    /// Plain TCP stream
    Plain(TcpStream),
    /// TLS-encrypted stream
    Tls(TlsStream<TcpStream>),
}

impl AsyncRead for MqttStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        match self.get_mut() {
            MqttStream::Plain(stream) => Pin::new(stream).poll_read(cx, buf),
            MqttStream::Tls(stream) => Pin::new(stream).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for MqttStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        match self.get_mut() {
            MqttStream::Plain(stream) => Pin::new(stream).poll_write(cx, buf),
            MqttStream::Tls(stream) => Pin::new(stream).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            MqttStream::Plain(stream) => Pin::new(stream).poll_flush(cx),
            MqttStream::Tls(stream) => Pin::new(stream).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            MqttStream::Plain(stream) => Pin::new(stream).poll_shutdown(cx),
            MqttStream::Tls(stream) => Pin::new(stream).poll_shutdown(cx),
        }
    }
}

/// MQTT Server handle
pub struct MqttServer {
    session_manager: Arc<SessionManager>,
    metrics: Arc<MqttMetrics>,
}

impl MqttServer {
    /// Create a new MQTT server
    pub fn new(config: &MqttConfig, metrics: Arc<MqttMetrics>) -> Self {
        Self {
            session_manager: Arc::new(SessionManager::new(
                config.max_connections,
                Some(metrics.clone()),
            )),
            metrics,
        }
    }

    /// Get the session manager
    pub fn session_manager(&self) -> Arc<SessionManager> {
        self.session_manager.clone()
    }

    /// Get the metrics
    pub fn metrics(&self) -> Arc<MqttMetrics> {
        self.metrics.clone()
    }
}

/// Start an MQTT server using tokio TCP listener
///
/// This is the main entry point for the MQTT broker. It binds to the
/// configured address and handles client connections with full MQTT
/// protocol support.
pub async fn start_mqtt_server(
    config: MqttConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let metrics = Arc::new(MqttMetrics::new());
    start_mqtt_server_with_metrics(config, metrics).await
}

/// Start an MQTT server with custom metrics
pub async fn start_mqtt_server_with_metrics(
    config: MqttConfig,
    metrics: Arc<MqttMetrics>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = format!("{}:{}", config.host, config.port);

    info!(
        "Starting MQTT broker on {}:{} (MQTT {:?})",
        config.host, config.port, config.version
    );

    let listener = TcpListener::bind(&addr).await?;
    let session_manager =
        Arc::new(SessionManager::new(config.max_connections, Some(metrics.clone())));

    info!(
        "MQTT broker listening on {}:{} (MQTT {:?})",
        config.host, config.port, config.version
    );

    // Spawn session cleanup task
    let cleanup_manager = session_manager.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(CLEANUP_INTERVAL_SECS));
        loop {
            interval.tick().await;
            let expired = cleanup_manager.cleanup_expired_sessions().await;
            if !expired.is_empty() {
                debug!("Cleaned up {} expired sessions", expired.len());
            }
        }
    });

    // Accept connections in a loop
    loop {
        match listener.accept().await {
            Ok((socket, addr)) => {
                info!("New MQTT connection from {}", addr);

                let session_manager = session_manager.clone();
                let metrics = metrics.clone();
                let max_packet_size = config.max_packet_size;

                tokio::spawn(async move {
                    if let Err(e) =
                        handle_connection(socket, addr, session_manager, metrics, max_packet_size)
                            .await
                    {
                        warn!("Connection error from {}: {}", addr, e);
                    }
                });
            }
            Err(e) => {
                error!("Error accepting MQTT connection: {}", e);
            }
        }
    }
}

/// Start an MQTT server with TLS support
///
/// This starts an MQTTS listener on the configured TLS port (default 8883).
/// Requires TLS certificate and key to be configured.
pub async fn start_mqtt_tls_server(config: MqttConfig) -> Result<(), TlsError> {
    let metrics = Arc::new(MqttMetrics::new());
    start_mqtt_tls_server_with_metrics(config, metrics).await
}

/// Start an MQTT server with TLS and custom metrics
pub async fn start_mqtt_tls_server_with_metrics(
    config: MqttConfig,
    metrics: Arc<MqttMetrics>,
) -> Result<(), TlsError> {
    if !config.tls_enabled {
        return Err(TlsError::ConfigError("TLS is not enabled in configuration".to_string()));
    }

    let tls_acceptor = create_tls_acceptor_with_client_auth(&config)?;
    let addr = format!("{}:{}", config.host, config.tls_port);

    let listener = TcpListener::bind(&addr)
        .await
        .map_err(|e| TlsError::ConfigError(format!("Failed to bind to {}: {}", addr, e)))?;

    info!(
        "Starting MQTTS broker with TLS on {}:{} (MQTT {:?})",
        config.host, config.tls_port, config.version
    );

    let session_manager =
        Arc::new(SessionManager::new(config.max_connections, Some(metrics.clone())));

    // Spawn session cleanup task
    let cleanup_manager = session_manager.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(CLEANUP_INTERVAL_SECS));
        loop {
            interval.tick().await;
            let expired = cleanup_manager.cleanup_expired_sessions().await;
            if !expired.is_empty() {
                debug!("Cleaned up {} expired sessions", expired.len());
            }
        }
    });

    // Accept TLS connections
    loop {
        match listener.accept().await {
            Ok((socket, addr)) => {
                info!("New MQTTS connection from {}", addr);

                let tls_acceptor = tls_acceptor.clone();
                let session_manager = session_manager.clone();
                let metrics = metrics.clone();
                let max_packet_size = config.max_packet_size;

                tokio::spawn(async move {
                    match tls_acceptor.accept(socket).await {
                        Ok(tls_stream) => {
                            if let Err(e) = handle_tls_connection(
                                tls_stream,
                                addr,
                                session_manager,
                                metrics,
                                max_packet_size,
                            )
                            .await
                            {
                                warn!("TLS connection error from {}: {}", addr, e);
                            }
                        }
                        Err(e) => {
                            warn!("TLS handshake failed from {}: {}", addr, e);
                        }
                    }
                });
            }
            Err(e) => {
                error!("Error accepting MQTTS connection: {}", e);
            }
        }
    }
}

/// Start both plain and TLS listeners concurrently
pub async fn start_mqtt_dual_server(
    config: MqttConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let metrics = Arc::new(MqttMetrics::new());
    start_mqtt_dual_server_with_metrics(config, metrics).await
}

/// Start both plain and TLS listeners with custom metrics
pub async fn start_mqtt_dual_server_with_metrics(
    config: MqttConfig,
    metrics: Arc<MqttMetrics>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let session_manager =
        Arc::new(SessionManager::new(config.max_connections, Some(metrics.clone())));

    // Spawn session cleanup task
    let cleanup_manager = session_manager.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(CLEANUP_INTERVAL_SECS));
        loop {
            interval.tick().await;
            let expired = cleanup_manager.cleanup_expired_sessions().await;
            if !expired.is_empty() {
                debug!("Cleaned up {} expired sessions", expired.len());
            }
        }
    });

    // Start plain TCP listener
    let plain_addr = format!("{}:{}", config.host, config.port);
    let plain_listener = TcpListener::bind(&plain_addr).await?;
    info!("Starting MQTT broker on {} (MQTT {:?})", plain_addr, config.version);

    let plain_session_manager = session_manager.clone();
    let plain_metrics = metrics.clone();
    let plain_max_packet_size = config.max_packet_size;

    tokio::spawn(async move {
        loop {
            match plain_listener.accept().await {
                Ok((socket, addr)) => {
                    info!("New MQTT connection from {}", addr);

                    let session_manager = plain_session_manager.clone();
                    let metrics = plain_metrics.clone();

                    tokio::spawn(async move {
                        if let Err(e) = handle_connection(
                            socket,
                            addr,
                            session_manager,
                            metrics,
                            plain_max_packet_size,
                        )
                        .await
                        {
                            warn!("Connection error from {}: {}", addr, e);
                        }
                    });
                }
                Err(e) => {
                    error!("Error accepting MQTT connection: {}", e);
                }
            }
        }
    });

    // Start TLS listener if enabled
    if config.tls_enabled {
        let tls_acceptor = create_tls_acceptor_with_client_auth(&config)?;
        let tls_addr = format!("{}:{}", config.host, config.tls_port);
        let tls_listener = TcpListener::bind(&tls_addr).await?;
        info!("Starting MQTTS broker with TLS on {}", tls_addr);

        let tls_session_manager = session_manager.clone();
        let tls_metrics = metrics.clone();
        let tls_max_packet_size = config.max_packet_size;

        tokio::spawn(async move {
            loop {
                match tls_listener.accept().await {
                    Ok((socket, addr)) => {
                        info!("New MQTTS connection from {}", addr);

                        let tls_acceptor = tls_acceptor.clone();
                        let session_manager = tls_session_manager.clone();
                        let metrics = tls_metrics.clone();

                        tokio::spawn(async move {
                            match tls_acceptor.accept(socket).await {
                                Ok(tls_stream) => {
                                    if let Err(e) = handle_tls_connection(
                                        tls_stream,
                                        addr,
                                        session_manager,
                                        metrics,
                                        tls_max_packet_size,
                                    )
                                    .await
                                    {
                                        warn!("TLS connection error from {}: {}", addr, e);
                                    }
                                }
                                Err(e) => {
                                    warn!("TLS handshake failed from {}: {}", addr, e);
                                }
                            }
                        });
                    }
                    Err(e) => {
                        error!("Error accepting MQTTS connection: {}", e);
                    }
                }
            }
        });
    }

    // Keep the main task running
    loop {
        tokio::time::sleep(Duration::from_secs(3600)).await;
    }
}

/// Handle a single TLS client connection
async fn handle_tls_connection(
    stream: TlsStream<TcpStream>,
    addr: std::net::SocketAddr,
    session_manager: Arc<SessionManager>,
    metrics: Arc<MqttMetrics>,
    max_packet_size: usize,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (read_half, write_half) = tokio::io::split(stream);
    let mut reader = tokio::io::BufReader::new(read_half);
    let mut writer = write_half;

    // Buffer for reading packets
    let mut buffer = vec![0u8; READ_BUFFER_SIZE.min(max_packet_size)];
    let mut buf_len = 0usize;

    // Client state
    let mut _client_id: Option<String> = None;
    let mut packet_rx: Option<mpsc::Receiver<Packet>> = None;

    // Read first packet - must be CONNECT
    let connect_timeout = Duration::from_secs(10);
    let first_read = tokio::time::timeout(connect_timeout, reader.read(&mut buffer[buf_len..]))
        .await
        .map_err(|_| "Connection timeout waiting for CONNECT")?;

    match first_read {
        Ok(0) => {
            debug!("TLS client {} closed connection before CONNECT", addr);
            return Ok(());
        }
        Ok(n) => buf_len += n,
        Err(e) => return Err(e.into()),
    }

    // Parse CONNECT packet
    let (connect_packet, consumed) = match PacketDecoder::decode(&buffer[..buf_len])? {
        Some((Packet::Connect(connect), consumed)) => (connect, consumed),
        Some((_, _)) => {
            warn!("First packet from TLS client {} was not CONNECT", addr);
            let connack = build_connack(false, ConnackCode::NotAuthorized);
            let bytes = PacketEncoder::encode(&connack)?;
            writer.write_all(&bytes).await?;
            return Err("Expected CONNECT packet".into());
        }
        None => {
            return Err("Incomplete CONNECT packet".into());
        }
    };

    // Shift buffer
    buffer.copy_within(consumed..buf_len, 0);
    buf_len -= consumed;

    // Validate CONNECT packet
    let cid = if connect_packet.client_id.is_empty() {
        if connect_packet.clean_session {
            format!("auto-tls-{}", uuid::Uuid::new_v4())
        } else {
            let connack = build_connack(false, ConnackCode::IdentifierRejected);
            let bytes = PacketEncoder::encode(&connack)?;
            writer.write_all(&bytes).await?;
            return Err("Empty client ID with clean_session=false".into());
        }
    } else {
        connect_packet.client_id.clone()
    };

    info!(
        "TLS CONNECT from {} (client_id={}, clean_session={})",
        addr, cid, connect_packet.clean_session
    );

    // Create channel for sending packets to this client
    let (tx, rx) = mpsc::channel(CLIENT_CHANNEL_CAPACITY);
    packet_rx = Some(rx);

    // Register with session manager
    let connect_result = session_manager
        .connect(cid.clone(), connect_packet.clean_session, connect_packet.keep_alive, tx)
        .await;

    let session_present = match connect_result {
        Ok((session_present, code)) => {
            let connack = build_connack(session_present, code);
            let bytes = PacketEncoder::encode(&connack)?;
            writer.write_all(&bytes).await?;
            session_present
        }
        Err(code) => {
            let connack = build_connack(false, code);
            let bytes = PacketEncoder::encode(&connack)?;
            writer.write_all(&bytes).await?;
            return Err(format!("Connection rejected: {:?}", code).into());
        }
    };

    _client_id = Some(cid.clone());

    info!("TLS client {} connected (session_present={})", cid, session_present);

    // Send retained messages for existing subscriptions (if session was restored)
    if session_present {
        let subscriptions = session_manager.get_client_subscriptions(&cid).await;

        for (filter, _sub_qos) in subscriptions {
            let retained = session_manager.get_retained_messages(&filter).await;
            for (topic, mut publish) in retained {
                if publish.qos != QoS::AtMostOnce {
                    if let Some(id) = session_manager.assign_packet_id(&cid).await {
                        publish.packet_id = Some(id);
                    }
                }

                let bytes = PacketEncoder::encode(&Packet::Publish(publish))?;
                writer.write_all(&bytes).await?;

                debug!(
                    "Delivered retained message for topic {} to restored TLS session {}",
                    topic, cid
                );
            }
        }
    }

    // Main connection loop
    let mut rx = packet_rx.take().unwrap();
    let result = handle_tls_client_loop(
        &mut reader,
        &mut writer,
        &mut rx,
        &cid,
        &session_manager,
        &metrics,
        &mut buffer,
        &mut buf_len,
        max_packet_size,
    )
    .await;

    // Clean up on disconnect
    session_manager.disconnect(&cid).await;
    info!("TLS client {} disconnected", cid);

    result
}

/// Handle the main TLS client message loop
async fn handle_tls_client_loop<R, W>(
    reader: &mut tokio::io::BufReader<R>,
    writer: &mut W,
    packet_rx: &mut mpsc::Receiver<Packet>,
    client_id: &str,
    session_manager: &Arc<SessionManager>,
    metrics: &Arc<MqttMetrics>,
    buffer: &mut Vec<u8>,
    buf_len: &mut usize,
    max_packet_size: usize,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    loop {
        tokio::select! {
            // Handle incoming packets from client
            read_result = reader.read(&mut buffer[*buf_len..]) => {
                match read_result {
                    Ok(0) => {
                        debug!("TLS client {} closed connection", client_id);
                        return Ok(());
                    }
                    Ok(n) => {
                        *buf_len += n;

                        // Check for oversized packet
                        if *buf_len > max_packet_size {
                            warn!("TLS client {} sent oversized packet", client_id);
                            metrics.record_error("oversized_packet");
                            return Err("Packet too large".into());
                        }

                        // Parse and handle packets
                        while let Some((packet, consumed)) = PacketDecoder::decode(&buffer[..*buf_len])? {
                            // Shift buffer
                            buffer.copy_within(consumed..*buf_len, 0);
                            *buf_len -= consumed;

                            // Handle the packet
                            match handle_tls_packet(
                                client_id,
                                packet,
                                writer,
                                session_manager,
                                metrics,
                            ).await {
                                Ok(true) => continue,
                                Ok(false) => return Ok(()), // Disconnect requested
                                Err(e) => {
                                    warn!("Error handling packet from TLS client {}: {}", client_id, e);
                                    metrics.record_error(&e.to_string());
                                }
                            }
                        }
                    }
                    Err(e) => {
                        return Err(e.into());
                    }
                }
            }

            // Handle outgoing packets to client
            packet = packet_rx.recv() => {
                match packet {
                    Some(Packet::Disconnect) => {
                        debug!("Sending disconnect to TLS client {}", client_id);
                        return Ok(());
                    }
                    Some(mut packet) => {
                        // Assign packet ID for QoS > 0 publish packets
                        if let Packet::Publish(ref mut publish) = packet {
                            if publish.qos != QoS::AtMostOnce && publish.packet_id.is_none() {
                                if let Some(id) = session_manager.assign_packet_id(client_id).await {
                                    publish.packet_id = Some(id);
                                }
                            }
                        }

                        let bytes = PacketEncoder::encode(&packet)?;
                        writer.write_all(&bytes).await?;
                    }
                    None => {
                        debug!("Channel closed for TLS client {}", client_id);
                        return Ok(());
                    }
                }
            }
        }
    }
}

/// Handle a single MQTT packet from TLS client
async fn handle_tls_packet<W>(
    client_id: &str,
    packet: Packet,
    writer: &mut W,
    session_manager: &Arc<SessionManager>,
    metrics: &Arc<MqttMetrics>,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>>
where
    W: AsyncWrite + Unpin,
{
    match packet {
        Packet::Connect(_) => {
            warn!("TLS client {} sent second CONNECT packet", client_id);
            return Ok(false);
        }

        Packet::Publish(publish) => {
            debug!(
                "PUBLISH from TLS client {} to topic {} (QoS {:?})",
                client_id, publish.topic, publish.qos
            );

            match publish.qos {
                QoS::AtMostOnce => {}
                QoS::AtLeastOnce => {
                    if let Some(packet_id) = publish.packet_id {
                        let puback = build_puback(packet_id);
                        let bytes = PacketEncoder::encode(&puback)?;
                        writer.write_all(&bytes).await?;
                    }
                }
                QoS::ExactlyOnce => {
                    if let Some(packet_id) = publish.packet_id {
                        session_manager.start_qos2_inbound(client_id, packet_id).await;
                        let pubrec = build_pubrec(packet_id);
                        let bytes = PacketEncoder::encode(&pubrec)?;
                        writer.write_all(&bytes).await?;
                        session_manager.mark_pubrec_sent(client_id, packet_id).await;
                    }
                }
            }

            session_manager.publish(client_id, &publish).await;
        }

        Packet::Puback(puback) => {
            debug!("PUBACK from TLS client {} for packet {}", client_id, puback.packet_id);
            session_manager.handle_puback(client_id, puback.packet_id).await;
        }

        Packet::Pubrec(pubrec) => {
            debug!("PUBREC from TLS client {} for packet {}", client_id, pubrec.packet_id);
            if session_manager.handle_pubrec(client_id, pubrec.packet_id).await {
                let pubrel = build_pubrel(pubrec.packet_id);
                let bytes = PacketEncoder::encode(&pubrel)?;
                writer.write_all(&bytes).await?;
            }
        }

        Packet::Pubrel(pubrel) => {
            debug!("PUBREL from TLS client {} for packet {}", client_id, pubrel.packet_id);
            if session_manager.handle_pubrel(client_id, pubrel.packet_id).await {
                let pubcomp = build_pubcomp(pubrel.packet_id);
                let bytes = PacketEncoder::encode(&pubcomp)?;
                writer.write_all(&bytes).await?;
                session_manager.complete_qos2_inbound(client_id, pubrel.packet_id).await;
            }
        }

        Packet::Pubcomp(pubcomp) => {
            debug!("PUBCOMP from TLS client {} for packet {}", client_id, pubcomp.packet_id);
            session_manager.handle_pubcomp(client_id, pubcomp.packet_id).await;
        }

        Packet::Subscribe(subscribe) => {
            debug!(
                "SUBSCRIBE from TLS client {} for {} topics",
                client_id,
                subscribe.subscriptions.len()
            );

            if let Some(return_codes) =
                session_manager.subscribe(client_id, subscribe.subscriptions.clone()).await
            {
                let suback = build_suback(subscribe.packet_id, return_codes);
                let bytes = PacketEncoder::encode(&suback)?;
                writer.write_all(&bytes).await?;

                for (filter, _) in &subscribe.subscriptions {
                    let retained = session_manager.get_retained_messages(filter).await;
                    for (topic, mut publish) in retained {
                        if publish.qos != QoS::AtMostOnce {
                            if let Some(id) = session_manager.assign_packet_id(client_id).await {
                                publish.packet_id = Some(id);
                            }
                        }
                        let bytes = PacketEncoder::encode(&Packet::Publish(publish))?;
                        writer.write_all(&bytes).await?;
                        debug!(
                            "Sent retained message for topic {} to TLS client {}",
                            topic, client_id
                        );
                    }
                }
            }
        }

        Packet::Unsubscribe(unsubscribe) => {
            debug!(
                "UNSUBSCRIBE from TLS client {} for {} topics",
                client_id,
                unsubscribe.topics.len()
            );

            session_manager.unsubscribe(client_id, unsubscribe.topics).await;

            let unsuback = build_unsuback(unsubscribe.packet_id);
            let bytes = PacketEncoder::encode(&unsuback)?;
            writer.write_all(&bytes).await?;
        }

        Packet::Pingreq => {
            debug!("PINGREQ from TLS client {}", client_id);
            session_manager.touch(client_id).await;

            let pingresp = Packet::Pingresp;
            let bytes = PacketEncoder::encode(&pingresp)?;
            writer.write_all(&bytes).await?;
        }

        Packet::Disconnect => {
            info!("DISCONNECT from TLS client {}", client_id);
            return Ok(false);
        }

        Packet::Connack(_) | Packet::Suback(_) | Packet::Unsuback(_) | Packet::Pingresp => {
            warn!("TLS client {} sent unexpected packet type: {:?}", client_id, packet);
            metrics.record_error("unexpected_packet_type");
        }
    }

    Ok(true)
}

/// Handle a single client connection
async fn handle_connection(
    socket: TcpStream,
    addr: std::net::SocketAddr,
    session_manager: Arc<SessionManager>,
    metrics: Arc<MqttMetrics>,
    max_packet_size: usize,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (read_half, mut write_half) = socket.into_split();
    let mut reader = tokio::io::BufReader::new(read_half);

    // Buffer for reading packets
    let mut buffer = vec![0u8; READ_BUFFER_SIZE.min(max_packet_size)];
    let mut buf_len = 0usize;

    // Client state
    let mut _client_id: Option<String> = None;
    let mut packet_rx: Option<mpsc::Receiver<Packet>> = None;

    // Read first packet - must be CONNECT
    let connect_timeout = Duration::from_secs(10);
    let first_read = tokio::time::timeout(connect_timeout, reader.read(&mut buffer[buf_len..]))
        .await
        .map_err(|_| "Connection timeout waiting for CONNECT")?;

    match first_read {
        Ok(0) => {
            debug!("Client {} closed connection before CONNECT", addr);
            return Ok(());
        }
        Ok(n) => buf_len += n,
        Err(e) => return Err(e.into()),
    }

    // Parse CONNECT packet
    let (connect_packet, consumed) = match PacketDecoder::decode(&buffer[..buf_len])? {
        Some((Packet::Connect(connect), consumed)) => (connect, consumed),
        Some((_, _)) => {
            warn!("First packet from {} was not CONNECT", addr);
            let connack = build_connack(false, ConnackCode::NotAuthorized);
            let bytes = PacketEncoder::encode(&connack)?;
            write_half.write_all(&bytes).await?;
            return Err("Expected CONNECT packet".into());
        }
        None => {
            return Err("Incomplete CONNECT packet".into());
        }
    };

    // Shift buffer
    buffer.copy_within(consumed..buf_len, 0);
    buf_len -= consumed;

    // Validate CONNECT packet
    let cid = if connect_packet.client_id.is_empty() {
        // Generate client ID for empty client ID with clean session
        if connect_packet.clean_session {
            format!("auto-{}", uuid::Uuid::new_v4())
        } else {
            let connack = build_connack(false, ConnackCode::IdentifierRejected);
            let bytes = PacketEncoder::encode(&connack)?;
            write_half.write_all(&bytes).await?;
            return Err("Empty client ID with clean_session=false".into());
        }
    } else {
        connect_packet.client_id.clone()
    };

    info!(
        "CONNECT from {} (client_id={}, clean_session={})",
        addr, cid, connect_packet.clean_session
    );

    // Create channel for sending packets to this client
    let (tx, rx) = mpsc::channel(CLIENT_CHANNEL_CAPACITY);
    packet_rx = Some(rx);

    // Register with session manager
    let connect_result = session_manager
        .connect(cid.clone(), connect_packet.clean_session, connect_packet.keep_alive, tx)
        .await;

    let session_present = match connect_result {
        Ok((session_present, code)) => {
            let connack = build_connack(session_present, code);
            let bytes = PacketEncoder::encode(&connack)?;
            write_half.write_all(&bytes).await?;
            session_present
        }
        Err(code) => {
            let connack = build_connack(false, code);
            let bytes = PacketEncoder::encode(&connack)?;
            write_half.write_all(&bytes).await?;
            return Err(format!("Connection rejected: {:?}", code).into());
        }
    };

    _client_id = Some(cid.clone());

    info!("Client {} connected (session_present={})", cid, session_present);

    // Send retained messages for existing subscriptions (if session was restored)
    if session_present {
        // Get the client's restored subscriptions
        let subscriptions = session_manager.get_client_subscriptions(&cid).await;

        for (filter, _sub_qos) in subscriptions {
            let retained = session_manager.get_retained_messages(&filter).await;
            for (topic, mut publish) in retained {
                // Assign packet ID if needed for QoS > 0
                if publish.qos != QoS::AtMostOnce {
                    if let Some(id) = session_manager.assign_packet_id(&cid).await {
                        publish.packet_id = Some(id);
                    }
                }

                let bytes = PacketEncoder::encode(&Packet::Publish(publish))?;
                write_half.write_all(&bytes).await?;

                debug!(
                    "Delivered retained message for topic {} to restored session {}",
                    topic, cid
                );
            }
        }
    }

    // Main connection loop
    let mut rx = packet_rx.take().unwrap();
    let result = handle_client_loop(
        &mut reader,
        &mut write_half,
        &mut rx,
        &cid,
        &session_manager,
        &metrics,
        &mut buffer,
        &mut buf_len,
        max_packet_size,
    )
    .await;

    // Clean up on disconnect
    session_manager.disconnect(&cid).await;
    info!("Client {} disconnected", cid);

    result
}

/// Handle the main client message loop
async fn handle_client_loop(
    reader: &mut tokio::io::BufReader<tokio::net::tcp::OwnedReadHalf>,
    writer: &mut tokio::net::tcp::OwnedWriteHalf,
    packet_rx: &mut mpsc::Receiver<Packet>,
    client_id: &str,
    session_manager: &Arc<SessionManager>,
    metrics: &Arc<MqttMetrics>,
    buffer: &mut Vec<u8>,
    buf_len: &mut usize,
    max_packet_size: usize,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    loop {
        tokio::select! {
            // Handle incoming packets from client
            read_result = reader.read(&mut buffer[*buf_len..]) => {
                match read_result {
                    Ok(0) => {
                        debug!("Client {} closed connection", client_id);
                        return Ok(());
                    }
                    Ok(n) => {
                        *buf_len += n;

                        // Check for oversized packet
                        if *buf_len > max_packet_size {
                            warn!("Client {} sent oversized packet", client_id);
                            metrics.record_error("oversized_packet");
                            return Err("Packet too large".into());
                        }

                        // Parse and handle packets
                        while let Some((packet, consumed)) = PacketDecoder::decode(&buffer[..*buf_len])? {
                            // Shift buffer
                            buffer.copy_within(consumed..*buf_len, 0);
                            *buf_len -= consumed;

                            // Handle the packet
                            match handle_packet(
                                client_id,
                                packet,
                                writer,
                                session_manager,
                                metrics,
                            ).await {
                                Ok(true) => continue,
                                Ok(false) => return Ok(()), // Disconnect requested
                                Err(e) => {
                                    warn!("Error handling packet from {}: {}", client_id, e);
                                    metrics.record_error(&e.to_string());
                                }
                            }
                        }
                    }
                    Err(e) => {
                        return Err(e.into());
                    }
                }
            }

            // Handle outgoing packets to client
            packet = packet_rx.recv() => {
                match packet {
                    Some(Packet::Disconnect) => {
                        debug!("Sending disconnect to {}", client_id);
                        return Ok(());
                    }
                    Some(mut packet) => {
                        // Assign packet ID for QoS > 0 publish packets
                        if let Packet::Publish(ref mut publish) = packet {
                            if publish.qos != QoS::AtMostOnce && publish.packet_id.is_none() {
                                if let Some(id) = session_manager.assign_packet_id(client_id).await {
                                    publish.packet_id = Some(id);
                                }
                            }
                        }

                        let bytes = PacketEncoder::encode(&packet)?;
                        writer.write_all(&bytes).await?;
                    }
                    None => {
                        debug!("Channel closed for {}", client_id);
                        return Ok(());
                    }
                }
            }
        }
    }
}

/// Handle a single MQTT packet
async fn handle_packet(
    client_id: &str,
    packet: Packet,
    writer: &mut tokio::net::tcp::OwnedWriteHalf,
    session_manager: &Arc<SessionManager>,
    metrics: &Arc<MqttMetrics>,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    match packet {
        Packet::Connect(_) => {
            // Second CONNECT is a protocol error
            warn!("Client {} sent second CONNECT packet", client_id);
            return Ok(false);
        }

        Packet::Publish(publish) => {
            debug!("PUBLISH from {} to topic {} (QoS {:?})", client_id, publish.topic, publish.qos);

            // Handle QoS acknowledgments
            match publish.qos {
                QoS::AtMostOnce => {
                    // No acknowledgment needed
                }
                QoS::AtLeastOnce => {
                    // Send PUBACK
                    if let Some(packet_id) = publish.packet_id {
                        let puback = build_puback(packet_id);
                        let bytes = PacketEncoder::encode(&puback)?;
                        writer.write_all(&bytes).await?;
                    }
                }
                QoS::ExactlyOnce => {
                    // Start QoS 2 flow
                    if let Some(packet_id) = publish.packet_id {
                        session_manager.start_qos2_inbound(client_id, packet_id).await;
                        let pubrec = build_pubrec(packet_id);
                        let bytes = PacketEncoder::encode(&pubrec)?;
                        writer.write_all(&bytes).await?;
                        session_manager.mark_pubrec_sent(client_id, packet_id).await;
                    }
                }
            }

            // Route message to subscribers
            session_manager.publish(client_id, &publish).await;
        }

        Packet::Puback(puback) => {
            debug!("PUBACK from {} for packet {}", client_id, puback.packet_id);
            session_manager.handle_puback(client_id, puback.packet_id).await;
        }

        Packet::Pubrec(pubrec) => {
            debug!("PUBREC from {} for packet {}", client_id, pubrec.packet_id);
            if session_manager.handle_pubrec(client_id, pubrec.packet_id).await {
                let pubrel = build_pubrel(pubrec.packet_id);
                let bytes = PacketEncoder::encode(&pubrel)?;
                writer.write_all(&bytes).await?;
            }
        }

        Packet::Pubrel(pubrel) => {
            debug!("PUBREL from {} for packet {}", client_id, pubrel.packet_id);
            if session_manager.handle_pubrel(client_id, pubrel.packet_id).await {
                let pubcomp = build_pubcomp(pubrel.packet_id);
                let bytes = PacketEncoder::encode(&pubcomp)?;
                writer.write_all(&bytes).await?;
                session_manager.complete_qos2_inbound(client_id, pubrel.packet_id).await;
            }
        }

        Packet::Pubcomp(pubcomp) => {
            debug!("PUBCOMP from {} for packet {}", client_id, pubcomp.packet_id);
            session_manager.handle_pubcomp(client_id, pubcomp.packet_id).await;
        }

        Packet::Subscribe(subscribe) => {
            debug!("SUBSCRIBE from {} for {} topics", client_id, subscribe.subscriptions.len());

            if let Some(return_codes) =
                session_manager.subscribe(client_id, subscribe.subscriptions.clone()).await
            {
                // Send SUBACK
                let suback = build_suback(subscribe.packet_id, return_codes);
                let bytes = PacketEncoder::encode(&suback)?;
                writer.write_all(&bytes).await?;

                // Send retained messages for new subscriptions
                for (filter, _) in &subscribe.subscriptions {
                    let retained = session_manager.get_retained_messages(filter).await;
                    for (topic, mut publish) in retained {
                        // Assign packet ID if needed
                        if publish.qos != QoS::AtMostOnce {
                            if let Some(id) = session_manager.assign_packet_id(client_id).await {
                                publish.packet_id = Some(id);
                            }
                        }
                        let bytes = PacketEncoder::encode(&Packet::Publish(publish))?;
                        writer.write_all(&bytes).await?;
                        debug!("Sent retained message for topic {} to {}", topic, client_id);
                    }
                }
            }
        }

        Packet::Unsubscribe(unsubscribe) => {
            debug!("UNSUBSCRIBE from {} for {} topics", client_id, unsubscribe.topics.len());

            session_manager.unsubscribe(client_id, unsubscribe.topics).await;

            // Send UNSUBACK
            let unsuback = build_unsuback(unsubscribe.packet_id);
            let bytes = PacketEncoder::encode(&unsuback)?;
            writer.write_all(&bytes).await?;
        }

        Packet::Pingreq => {
            debug!("PINGREQ from {}", client_id);
            session_manager.touch(client_id).await;

            // Send PINGRESP
            let pingresp = Packet::Pingresp;
            let bytes = PacketEncoder::encode(&pingresp)?;
            writer.write_all(&bytes).await?;
        }

        Packet::Disconnect => {
            info!("DISCONNECT from {}", client_id);
            return Ok(false);
        }

        // These are server-to-client packets, shouldn't receive from client
        Packet::Connack(_) | Packet::Suback(_) | Packet::Unsuback(_) | Packet::Pingresp => {
            warn!("Client {} sent unexpected packet type: {:?}", client_id, packet);
            metrics.record_error("unexpected_packet_type");
        }
    }

    Ok(true)
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
            ..Default::default()
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

    #[tokio::test]
    async fn test_mqtt_server_creation() {
        let config = MqttConfig::default();
        let metrics = Arc::new(MqttMetrics::new());
        let server = MqttServer::new(&config, metrics.clone());

        assert_eq!(server.session_manager().connection_count().await, 0);
    }

    #[tokio::test]
    async fn test_session_manager_integration() {
        let config = MqttConfig {
            max_connections: 10,
            ..Default::default()
        };
        let metrics = Arc::new(MqttMetrics::new());
        let server = MqttServer::new(&config, metrics);

        let (tx, _rx) = mpsc::channel(10);
        let result =
            server.session_manager().connect("test-client".to_string(), true, 60, tx).await;

        assert!(result.is_ok());
        assert_eq!(server.session_manager().connection_count().await, 1);

        let clients = server.session_manager().get_connected_clients().await;
        assert!(clients.contains(&"test-client".to_string()));
    }
}
