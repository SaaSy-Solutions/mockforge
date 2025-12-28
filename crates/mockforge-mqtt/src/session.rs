//! MQTT Session Management
//!
//! This module handles client session tracking, subscription management,
//! and QoS message delivery state for the MQTT broker.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::io::AsyncWriteExt;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info, warn};

use crate::metrics::MqttMetrics;
use crate::protocol::{
    ConnackCode, ConnackPacket, Packet, PacketEncoder, PubackPacket, PubcompPacket, PublishPacket,
    PubrecPacket, PubrelPacket, QoS, SubackPacket, SubackReturnCode, UnsubackPacket,
};
use crate::topics::TopicTree;

/// Message to be delivered to a client
#[derive(Debug, Clone)]
pub struct PendingMessage {
    pub packet_id: u16,
    pub topic: String,
    pub payload: Vec<u8>,
    pub qos: QoS,
    pub retain: bool,
    pub timestamp: u64,
    pub retry_count: u8,
}

/// QoS 2 message state tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Qos2State {
    /// PUBLISH received, waiting to send PUBREC
    PendingPubrec,
    /// PUBREC sent, waiting for PUBREL
    WaitingPubrel,
    /// PUBREL received, waiting to send PUBCOMP
    PendingPubcomp,
}

/// State of a QoS 2 outbound message
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Qos2OutboundState {
    /// PUBLISH sent, waiting for PUBREC
    WaitingPubrec,
    /// PUBREC received, PUBREL sent, waiting for PUBCOMP
    WaitingPubcomp,
}

/// Client session state
#[derive(Debug)]
pub struct ClientSession {
    /// Client identifier
    pub client_id: String,
    /// Clean session flag from CONNECT
    pub clean_session: bool,
    /// Keep-alive interval in seconds
    pub keep_alive: u16,
    /// Topic subscriptions with QoS level
    pub subscriptions: HashMap<String, QoS>,
    /// Outbound messages pending acknowledgment (QoS 1)
    pub pending_qos1_out: HashMap<u16, PendingMessage>,
    /// Outbound QoS 2 message states
    pub pending_qos2_out: HashMap<u16, Qos2OutboundState>,
    /// Inbound QoS 2 message states (for duplicate detection)
    pub pending_qos2_in: HashMap<u16, Qos2State>,
    /// Last activity timestamp
    pub last_activity: u64,
    /// Connection timestamp
    pub connected_at: u64,
    /// Next packet ID for this session
    next_packet_id: u16,
    /// Username if authenticated
    pub username: Option<String>,
}

impl ClientSession {
    /// Create a new client session
    pub fn new(client_id: String, clean_session: bool, keep_alive: u16) -> Self {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();

        Self {
            client_id,
            clean_session,
            keep_alive,
            subscriptions: HashMap::new(),
            pending_qos1_out: HashMap::new(),
            pending_qos2_out: HashMap::new(),
            pending_qos2_in: HashMap::new(),
            last_activity: now,
            connected_at: now,
            next_packet_id: 1,
            username: None,
        }
    }

    /// Generate next packet ID for this session
    pub fn next_packet_id(&mut self) -> u16 {
        let id = self.next_packet_id;
        self.next_packet_id = self.next_packet_id.wrapping_add(1);
        if self.next_packet_id == 0 {
            self.next_packet_id = 1; // Skip 0
        }
        id
    }

    /// Update last activity timestamp
    pub fn touch(&mut self) {
        self.last_activity =
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    }

    /// Check if session has timed out
    pub fn is_expired(&self) -> bool {
        if self.keep_alive == 0 {
            return false; // No timeout
        }

        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();

        // MQTT spec says server should wait 1.5x keep_alive before disconnecting
        let timeout = (self.keep_alive as u64) * 3 / 2;
        now - self.last_activity > timeout
    }

    /// Add a subscription
    pub fn subscribe(&mut self, topic_filter: String, qos: QoS) {
        self.subscriptions.insert(topic_filter, qos);
    }

    /// Remove a subscription
    pub fn unsubscribe(&mut self, topic_filter: &str) -> bool {
        self.subscriptions.remove(topic_filter).is_some()
    }

    /// Queue a message for QoS 1 delivery
    pub fn queue_qos1_message(&mut self, packet_id: u16, message: PendingMessage) {
        self.pending_qos1_out.insert(packet_id, message);
    }

    /// Handle PUBACK for QoS 1
    pub fn handle_puback(&mut self, packet_id: u16) -> Option<PendingMessage> {
        self.pending_qos1_out.remove(&packet_id)
    }

    /// Start QoS 2 outbound flow (PUBLISH sent)
    pub fn start_qos2_outbound(&mut self, packet_id: u16) {
        self.pending_qos2_out.insert(packet_id, Qos2OutboundState::WaitingPubrec);
    }

    /// Handle PUBREC for QoS 2 outbound
    pub fn handle_pubrec(&mut self, packet_id: u16) -> bool {
        if let Some(state) = self.pending_qos2_out.get_mut(&packet_id) {
            if *state == Qos2OutboundState::WaitingPubrec {
                *state = Qos2OutboundState::WaitingPubcomp;
                return true;
            }
        }
        false
    }

    /// Handle PUBCOMP for QoS 2 outbound (completes flow)
    pub fn handle_pubcomp(&mut self, packet_id: u16) -> bool {
        if let Some(state) = self.pending_qos2_out.get(&packet_id) {
            if *state == Qos2OutboundState::WaitingPubcomp {
                self.pending_qos2_out.remove(&packet_id);
                return true;
            }
        }
        false
    }

    /// Start QoS 2 inbound flow (PUBLISH received)
    pub fn start_qos2_inbound(&mut self, packet_id: u16) {
        self.pending_qos2_in.insert(packet_id, Qos2State::PendingPubrec);
    }

    /// Handle PUBREL for QoS 2 inbound
    pub fn handle_pubrel(&mut self, packet_id: u16) -> bool {
        if let Some(state) = self.pending_qos2_in.get_mut(&packet_id) {
            if *state == Qos2State::WaitingPubrel {
                *state = Qos2State::PendingPubcomp;
                return true;
            }
        }
        false
    }

    /// Complete QoS 2 inbound flow (PUBCOMP sent)
    pub fn complete_qos2_inbound(&mut self, packet_id: u16) {
        self.pending_qos2_in.remove(&packet_id);
    }

    /// Mark PUBREC as sent for QoS 2 inbound
    pub fn mark_pubrec_sent(&mut self, packet_id: u16) {
        if let Some(state) = self.pending_qos2_in.get_mut(&packet_id) {
            if *state == Qos2State::PendingPubrec {
                *state = Qos2State::WaitingPubrel;
            }
        }
    }
}

/// Channel for sending packets to a connected client
pub type ClientSender = mpsc::Sender<Packet>;

/// Active client connection state
pub struct ActiveClient {
    /// The client session
    pub session: ClientSession,
    /// Channel to send packets to the client
    pub sender: ClientSender,
}

/// Session manager for tracking all client sessions
pub struct SessionManager {
    /// Active connected clients
    active_clients: RwLock<HashMap<String, ActiveClient>>,
    /// Persistent sessions for reconnecting clients
    persistent_sessions: RwLock<HashMap<String, ClientSession>>,
    /// Topic subscription tree
    topics: RwLock<TopicTree>,
    /// Metrics collector
    metrics: Option<Arc<MqttMetrics>>,
    /// Maximum number of connections
    max_connections: usize,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new(max_connections: usize, metrics: Option<Arc<MqttMetrics>>) -> Self {
        Self {
            active_clients: RwLock::new(HashMap::new()),
            persistent_sessions: RwLock::new(HashMap::new()),
            topics: RwLock::new(TopicTree::new()),
            metrics,
            max_connections,
        }
    }

    /// Handle a new client connection
    pub async fn connect(
        &self,
        client_id: String,
        clean_session: bool,
        keep_alive: u16,
        sender: ClientSender,
    ) -> Result<(bool, ConnackCode), ConnackCode> {
        let active = self.active_clients.read().await;
        if active.len() >= self.max_connections {
            return Err(ConnackCode::ServerUnavailable);
        }
        drop(active);

        // Check if client is already connected
        let mut active = self.active_clients.write().await;
        if let Some(existing) = active.remove(&client_id) {
            // Disconnect existing client
            info!("Disconnecting existing client {} for new connection", client_id);
            let _ = existing.sender.send(Packet::Disconnect).await;

            if let Some(metrics) = &self.metrics {
                metrics.record_connection_closed();
            }
        }

        // Check for persistent session
        let mut persistent = self.persistent_sessions.write().await;
        let (session, session_present) = if clean_session {
            // Remove any existing session
            persistent.remove(&client_id);
            // Remove subscriptions from topic tree
            let mut topics = self.topics.write().await;
            if let Some(old_session) = persistent.get(&client_id) {
                for filter in old_session.subscriptions.keys() {
                    topics.unsubscribe(filter, &client_id);
                }
            }
            (ClientSession::new(client_id.clone(), true, keep_alive), false)
        } else if let Some(mut session) = persistent.remove(&client_id) {
            // Restore persistent session
            session.keep_alive = keep_alive;
            session.touch();
            (session, true)
        } else {
            (ClientSession::new(client_id.clone(), false, keep_alive), false)
        };

        active.insert(client_id.clone(), ActiveClient { session, sender });

        if let Some(metrics) = &self.metrics {
            metrics.record_connection();
        }

        info!(
            "Client {} connected (clean_session={}, session_present={})",
            client_id, clean_session, session_present
        );

        Ok((session_present, ConnackCode::Accepted))
    }

    /// Handle client disconnect
    pub async fn disconnect(&self, client_id: &str) {
        let mut active = self.active_clients.write().await;
        if let Some(client) = active.remove(client_id) {
            if !client.session.clean_session {
                // Persist session for later reconnection
                let mut persistent = self.persistent_sessions.write().await;
                persistent.insert(client_id.to_string(), client.session);
                info!("Persisted session for client {}", client_id);
            } else {
                // Clean session - remove subscriptions
                let mut topics = self.topics.write().await;
                for filter in client.session.subscriptions.keys() {
                    topics.unsubscribe(filter, client_id);

                    if let Some(metrics) = &self.metrics {
                        metrics.record_unsubscription();
                    }
                }
                info!("Cleaned up session for client {}", client_id);
            }

            if let Some(metrics) = &self.metrics {
                metrics.record_connection_closed();
            }
        }
    }

    /// Handle SUBSCRIBE packet
    pub async fn subscribe(
        &self,
        client_id: &str,
        subscriptions: Vec<(String, QoS)>,
    ) -> Option<Vec<SubackReturnCode>> {
        let mut active = self.active_clients.write().await;
        let client = active.get_mut(client_id)?;

        let mut topics = self.topics.write().await;
        let mut return_codes = Vec::new();

        for (filter, requested_qos) in subscriptions {
            // Add to topic tree
            topics.subscribe(&filter, requested_qos as u8, client_id);

            // Add to session
            client.session.subscribe(filter.clone(), requested_qos);

            // Return granted QoS (we grant what was requested)
            return_codes.push(SubackReturnCode::success(requested_qos));

            if let Some(metrics) = &self.metrics {
                metrics.record_subscription();
            }

            debug!("Client {} subscribed to {} with QoS {:?}", client_id, filter, requested_qos);
        }

        Some(return_codes)
    }

    /// Handle UNSUBSCRIBE packet
    pub async fn unsubscribe(&self, client_id: &str, topic_filters: Vec<String>) -> bool {
        let mut active = self.active_clients.write().await;
        let client = active.get_mut(client_id);

        if client.is_none() {
            return false;
        }

        let client = client.unwrap();
        let mut topics = self.topics.write().await;

        for filter in topic_filters {
            topics.unsubscribe(&filter, client_id);
            client.session.unsubscribe(&filter);

            if let Some(metrics) = &self.metrics {
                metrics.record_unsubscription();
            }

            debug!("Client {} unsubscribed from {}", client_id, filter);
        }

        true
    }

    /// Handle PUBLISH packet - route to subscribers
    pub async fn publish(&self, publisher_id: &str, publish: &PublishPacket) {
        // Update publisher's last activity
        {
            let mut active = self.active_clients.write().await;
            if let Some(client) = active.get_mut(publisher_id) {
                client.session.touch();
            }
        }

        if let Some(metrics) = &self.metrics {
            metrics.record_publish(publish.qos as u8);
        }

        // Handle retained messages
        if publish.retain {
            let mut topics = self.topics.write().await;
            topics.retain_message(&publish.topic, publish.payload.clone(), publish.qos as u8);

            if let Some(metrics) = &self.metrics {
                metrics.record_retained_message();
            }
        }

        // Find matching subscribers
        let topics = self.topics.read().await;
        let subscribers = topics.match_topic(&publish.topic);

        // Deliver to each subscriber
        let active = self.active_clients.read().await;
        for sub in subscribers {
            if sub.client_id == publisher_id {
                continue; // Don't send to self
            }

            if let Some(client) = active.get(&sub.client_id) {
                // Determine delivery QoS (minimum of publish and subscription QoS)
                let delivery_qos = std::cmp::min(publish.qos as u8, sub.qos);
                let delivery_qos = QoS::try_from(delivery_qos).unwrap_or(QoS::AtMostOnce);

                let packet = Packet::Publish(PublishPacket {
                    dup: false,
                    qos: delivery_qos,
                    retain: false, // Only first delivery can have retain
                    topic: publish.topic.clone(),
                    packet_id: if delivery_qos != QoS::AtMostOnce {
                        Some(0) // Will be assigned by receiver
                    } else {
                        None
                    },
                    payload: publish.payload.clone(),
                });

                if client.sender.send(packet).await.is_ok() {
                    if let Some(metrics) = &self.metrics {
                        metrics.record_delivery();
                    }
                    debug!("Delivered message to {} on topic {}", sub.client_id, publish.topic);
                }
            }
        }
    }

    /// Handle PUBACK from client
    pub async fn handle_puback(&self, client_id: &str, packet_id: u16) {
        let mut active = self.active_clients.write().await;
        if let Some(client) = active.get_mut(client_id) {
            client.session.touch();
            if client.session.handle_puback(packet_id).is_some() {
                debug!("QoS 1 delivery confirmed for client {}, packet {}", client_id, packet_id);
            }
        }
    }

    /// Handle PUBREC from client (QoS 2 step 1)
    pub async fn handle_pubrec(&self, client_id: &str, packet_id: u16) -> bool {
        let mut active = self.active_clients.write().await;
        if let Some(client) = active.get_mut(client_id) {
            client.session.touch();
            if client.session.handle_pubrec(packet_id) {
                debug!("QoS 2 PUBREC received for client {}, packet {}", client_id, packet_id);
                return true;
            }
        }
        false
    }

    /// Handle PUBREL from client (QoS 2 step 2)
    pub async fn handle_pubrel(&self, client_id: &str, packet_id: u16) -> bool {
        let mut active = self.active_clients.write().await;
        if let Some(client) = active.get_mut(client_id) {
            client.session.touch();
            if client.session.handle_pubrel(packet_id) {
                debug!("QoS 2 PUBREL received for client {}, packet {}", client_id, packet_id);
                return true;
            }
        }
        false
    }

    /// Handle PUBCOMP from client (QoS 2 step 3)
    pub async fn handle_pubcomp(&self, client_id: &str, packet_id: u16) {
        let mut active = self.active_clients.write().await;
        if let Some(client) = active.get_mut(client_id) {
            client.session.touch();
            if client.session.handle_pubcomp(packet_id) {
                debug!("QoS 2 delivery completed for client {}, packet {}", client_id, packet_id);
            }
        }
    }

    /// Update client activity timestamp (for PINGREQ)
    pub async fn touch(&self, client_id: &str) {
        let mut active = self.active_clients.write().await;
        if let Some(client) = active.get_mut(client_id) {
            client.session.touch();
        }
    }

    /// Get retained messages matching a topic filter
    pub async fn get_retained_messages(&self, filter: &str) -> Vec<(String, PublishPacket)> {
        let topics = self.topics.read().await;
        topics
            .get_retained_for_filter(filter)
            .into_iter()
            .map(|(topic, msg)| {
                (
                    topic.to_string(),
                    PublishPacket {
                        dup: false,
                        qos: QoS::try_from(msg.qos).unwrap_or(QoS::AtMostOnce),
                        retain: true,
                        topic: topic.to_string(),
                        packet_id: None,
                        payload: msg.payload.clone(),
                    },
                )
            })
            .collect()
    }

    /// Get sender for a specific client
    pub async fn get_sender(&self, client_id: &str) -> Option<ClientSender> {
        let active = self.active_clients.read().await;
        active.get(client_id).map(|c| c.sender.clone())
    }

    /// Get list of connected client IDs
    pub async fn get_connected_clients(&self) -> Vec<String> {
        let active = self.active_clients.read().await;
        active.keys().cloned().collect()
    }

    /// Get count of active connections
    pub async fn connection_count(&self) -> usize {
        let active = self.active_clients.read().await;
        active.len()
    }

    /// Check and disconnect expired sessions
    pub async fn cleanup_expired_sessions(&self) -> Vec<String> {
        let mut expired = Vec::new();
        let active = self.active_clients.read().await;

        for (client_id, client) in active.iter() {
            if client.session.is_expired() {
                expired.push(client_id.clone());
            }
        }
        drop(active);

        for client_id in &expired {
            warn!("Disconnecting expired session: {}", client_id);
            self.disconnect(client_id).await;
        }

        expired
    }

    /// Assign a packet ID for outgoing QoS > 0 message
    pub async fn assign_packet_id(&self, client_id: &str) -> Option<u16> {
        let mut active = self.active_clients.write().await;
        active.get_mut(client_id).map(|c| c.session.next_packet_id())
    }

    /// Get a client's subscriptions
    pub async fn get_client_subscriptions(&self, client_id: &str) -> Vec<(String, QoS)> {
        let active = self.active_clients.read().await;
        if let Some(client) = active.get(client_id) {
            client
                .session
                .subscriptions
                .iter()
                .map(|(filter, qos)| (filter.clone(), *qos))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Start QoS 2 inbound tracking
    pub async fn start_qos2_inbound(&self, client_id: &str, packet_id: u16) {
        let mut active = self.active_clients.write().await;
        if let Some(client) = active.get_mut(client_id) {
            client.session.start_qos2_inbound(packet_id);
        }
    }

    /// Mark PUBREC sent for QoS 2 inbound
    pub async fn mark_pubrec_sent(&self, client_id: &str, packet_id: u16) {
        let mut active = self.active_clients.write().await;
        if let Some(client) = active.get_mut(client_id) {
            client.session.mark_pubrec_sent(packet_id);
        }
    }

    /// Complete QoS 2 inbound (PUBCOMP sent)
    pub async fn complete_qos2_inbound(&self, client_id: &str, packet_id: u16) {
        let mut active = self.active_clients.write().await;
        if let Some(client) = active.get_mut(client_id) {
            client.session.complete_qos2_inbound(packet_id);
        }
    }
}

/// Helper to write a packet to a TCP stream
pub async fn write_packet(
    writer: &mut OwnedWriteHalf,
    packet: &Packet,
) -> Result<(), std::io::Error> {
    let bytes = PacketEncoder::encode(packet)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
    writer.write_all(&bytes).await?;
    writer.flush().await?;
    Ok(())
}

/// Build a CONNACK response packet
pub fn build_connack(session_present: bool, code: ConnackCode) -> Packet {
    Packet::Connack(ConnackPacket {
        session_present,
        return_code: code,
    })
}

/// Build a SUBACK response packet
pub fn build_suback(packet_id: u16, return_codes: Vec<SubackReturnCode>) -> Packet {
    Packet::Suback(SubackPacket {
        packet_id,
        return_codes,
    })
}

/// Build an UNSUBACK response packet
pub fn build_unsuback(packet_id: u16) -> Packet {
    Packet::Unsuback(UnsubackPacket { packet_id })
}

/// Build a PUBACK response packet
pub fn build_puback(packet_id: u16) -> Packet {
    Packet::Puback(PubackPacket { packet_id })
}

/// Build a PUBREC response packet
pub fn build_pubrec(packet_id: u16) -> Packet {
    Packet::Pubrec(PubrecPacket { packet_id })
}

/// Build a PUBREL packet
pub fn build_pubrel(packet_id: u16) -> Packet {
    Packet::Pubrel(PubrelPacket { packet_id })
}

/// Build a PUBCOMP response packet
pub fn build_pubcomp(packet_id: u16) -> Packet {
    Packet::Pubcomp(PubcompPacket { packet_id })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_session_new() {
        let session = ClientSession::new("test-client".to_string(), true, 60);
        assert_eq!(session.client_id, "test-client");
        assert!(session.clean_session);
        assert_eq!(session.keep_alive, 60);
        assert!(session.subscriptions.is_empty());
    }

    #[test]
    fn test_client_session_packet_id() {
        let mut session = ClientSession::new("test".to_string(), true, 60);
        assert_eq!(session.next_packet_id(), 1);
        assert_eq!(session.next_packet_id(), 2);
        assert_eq!(session.next_packet_id(), 3);
    }

    #[test]
    fn test_client_session_packet_id_wrap() {
        let mut session = ClientSession::new("test".to_string(), true, 60);
        session.next_packet_id = 65535;
        assert_eq!(session.next_packet_id(), 65535);
        assert_eq!(session.next_packet_id(), 1); // Wrapped, skipped 0
    }

    #[test]
    fn test_client_session_subscribe() {
        let mut session = ClientSession::new("test".to_string(), true, 60);
        session.subscribe("topic/a".to_string(), QoS::AtLeastOnce);
        session.subscribe("topic/b".to_string(), QoS::ExactlyOnce);

        assert_eq!(session.subscriptions.len(), 2);
        assert_eq!(session.subscriptions.get("topic/a"), Some(&QoS::AtLeastOnce));
    }

    #[test]
    fn test_client_session_unsubscribe() {
        let mut session = ClientSession::new("test".to_string(), true, 60);
        session.subscribe("topic/a".to_string(), QoS::AtLeastOnce);
        assert!(session.unsubscribe("topic/a"));
        assert!(!session.unsubscribe("topic/a")); // Already removed
    }

    #[test]
    fn test_client_session_qos1_flow() {
        let mut session = ClientSession::new("test".to_string(), true, 60);

        let msg = PendingMessage {
            packet_id: 100,
            topic: "test".to_string(),
            payload: vec![1, 2, 3],
            qos: QoS::AtLeastOnce,
            retain: false,
            timestamp: 0,
            retry_count: 0,
        };

        session.queue_qos1_message(100, msg);
        assert!(session.pending_qos1_out.contains_key(&100));

        let removed = session.handle_puback(100);
        assert!(removed.is_some());
        assert!(!session.pending_qos1_out.contains_key(&100));
    }

    #[test]
    fn test_client_session_qos2_outbound_flow() {
        let mut session = ClientSession::new("test".to_string(), true, 60);

        // Start QoS 2 flow
        session.start_qos2_outbound(200);
        assert!(session.pending_qos2_out.contains_key(&200));
        assert_eq!(session.pending_qos2_out.get(&200), Some(&Qos2OutboundState::WaitingPubrec));

        // Receive PUBREC
        assert!(session.handle_pubrec(200));
        assert_eq!(session.pending_qos2_out.get(&200), Some(&Qos2OutboundState::WaitingPubcomp));

        // Receive PUBCOMP
        assert!(session.handle_pubcomp(200));
        assert!(!session.pending_qos2_out.contains_key(&200));
    }

    #[test]
    fn test_client_session_qos2_inbound_flow() {
        let mut session = ClientSession::new("test".to_string(), true, 60);

        // Start QoS 2 inbound
        session.start_qos2_inbound(300);
        assert!(session.pending_qos2_in.contains_key(&300));

        // Send PUBREC
        session.mark_pubrec_sent(300);
        assert_eq!(session.pending_qos2_in.get(&300), Some(&Qos2State::WaitingPubrel));

        // Receive PUBREL
        assert!(session.handle_pubrel(300));
        assert_eq!(session.pending_qos2_in.get(&300), Some(&Qos2State::PendingPubcomp));

        // Send PUBCOMP
        session.complete_qos2_inbound(300);
        assert!(!session.pending_qos2_in.contains_key(&300));
    }

    #[tokio::test]
    async fn test_session_manager_connect() {
        let manager = SessionManager::new(100, None);
        let (tx, _rx) = mpsc::channel(10);

        let result = manager.connect("client-1".to_string(), true, 60, tx).await;
        assert!(result.is_ok());
        let (session_present, code) = result.unwrap();
        assert!(!session_present);
        assert_eq!(code, ConnackCode::Accepted);

        assert_eq!(manager.connection_count().await, 1);
    }

    #[tokio::test]
    async fn test_session_manager_disconnect() {
        let manager = SessionManager::new(100, None);
        let (tx, _rx) = mpsc::channel(10);

        manager.connect("client-1".to_string(), true, 60, tx).await.unwrap();
        manager.disconnect("client-1").await;

        assert_eq!(manager.connection_count().await, 0);
    }

    #[tokio::test]
    async fn test_session_manager_persistent_session() {
        let manager = SessionManager::new(100, None);

        // First connection with clean_session=false
        let (tx1, _rx1) = mpsc::channel(10);
        manager.connect("client-1".to_string(), false, 60, tx1).await.unwrap();

        // Subscribe
        manager
            .subscribe("client-1", vec![("topic/a".to_string(), QoS::AtLeastOnce)])
            .await;

        // Disconnect
        manager.disconnect("client-1").await;

        // Reconnect - should have session
        let (tx2, _rx2) = mpsc::channel(10);
        let result = manager.connect("client-1".to_string(), false, 60, tx2).await;
        let (session_present, _) = result.unwrap();
        assert!(session_present);
    }

    #[tokio::test]
    async fn test_session_manager_subscribe() {
        let manager = SessionManager::new(100, None);
        let (tx, _rx) = mpsc::channel(10);

        manager.connect("client-1".to_string(), true, 60, tx).await.unwrap();

        let result = manager
            .subscribe(
                "client-1",
                vec![
                    ("topic/a".to_string(), QoS::AtMostOnce),
                    ("topic/b".to_string(), QoS::AtLeastOnce),
                ],
            )
            .await;

        assert!(result.is_some());
        let codes = result.unwrap();
        assert_eq!(codes.len(), 2);
        assert_eq!(codes[0], SubackReturnCode::SuccessQoS0);
        assert_eq!(codes[1], SubackReturnCode::SuccessQoS1);
    }

    #[tokio::test]
    async fn test_session_manager_unsubscribe() {
        let manager = SessionManager::new(100, None);
        let (tx, _rx) = mpsc::channel(10);

        manager.connect("client-1".to_string(), true, 60, tx).await.unwrap();

        manager
            .subscribe("client-1", vec![("topic/a".to_string(), QoS::AtMostOnce)])
            .await;

        let result = manager.unsubscribe("client-1", vec!["topic/a".to_string()]).await;
        assert!(result);
    }

    #[tokio::test]
    async fn test_session_manager_max_connections() {
        let manager = SessionManager::new(2, None);

        let (tx1, _rx1) = mpsc::channel(10);
        let (tx2, _rx2) = mpsc::channel(10);
        let (tx3, _rx3) = mpsc::channel(10);

        manager.connect("client-1".to_string(), true, 60, tx1).await.unwrap();
        manager.connect("client-2".to_string(), true, 60, tx2).await.unwrap();

        let result = manager.connect("client-3".to_string(), true, 60, tx3).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ConnackCode::ServerUnavailable);
    }

    #[test]
    fn test_build_connack() {
        let packet = build_connack(true, ConnackCode::Accepted);
        if let Packet::Connack(connack) = packet {
            assert!(connack.session_present);
            assert_eq!(connack.return_code, ConnackCode::Accepted);
        } else {
            panic!("Expected Connack packet");
        }
    }

    #[test]
    fn test_build_suback() {
        let packet =
            build_suback(100, vec![SubackReturnCode::SuccessQoS0, SubackReturnCode::SuccessQoS1]);
        if let Packet::Suback(suback) = packet {
            assert_eq!(suback.packet_id, 100);
            assert_eq!(suback.return_codes.len(), 2);
        } else {
            panic!("Expected Suback packet");
        }
    }

    #[test]
    fn test_suback_return_code_success() {
        assert_eq!(SubackReturnCode::success(QoS::AtMostOnce), SubackReturnCode::SuccessQoS0);
        assert_eq!(SubackReturnCode::success(QoS::AtLeastOnce), SubackReturnCode::SuccessQoS1);
        assert_eq!(SubackReturnCode::success(QoS::ExactlyOnce), SubackReturnCode::SuccessQoS2);
    }
}
