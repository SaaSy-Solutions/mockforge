use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// MQTT Quality of Service levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QoS {
    AtMostOnce = 0,  // QoS 0
    AtLeastOnce = 1, // QoS 1
    ExactlyOnce = 2, // QoS 2
}

impl QoS {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(QoS::AtMostOnce),
            1 => Some(QoS::AtLeastOnce),
            2 => Some(QoS::ExactlyOnce),
            _ => None,
        }
    }

    pub fn as_u8(&self) -> u8 {
        *self as u8
    }
}

/// Message state for QoS handling
#[derive(Debug, Clone)]
pub struct MessageState {
    pub packet_id: u16,
    pub topic: String,
    pub payload: Vec<u8>,
    pub qos: QoS,
    pub retained: bool,
    pub timestamp: u64,
}

/// QoS 1 message awaiting acknowledgment
#[derive(Debug, Clone)]
struct PendingQoS1Message {
    message: MessageState,
    client_id: String,
    retry_count: u8,
}

/// QoS 2 message state
#[derive(Debug, Clone)]
enum QoS2State {
    Received, // PUBREC sent, waiting for PUBREL
    Released, // PUBREL received, PUBCOMP sent
}

/// QoS handler for managing message delivery guarantees
pub struct QoSHandler {
    qos1_pending: Arc<RwLock<HashMap<u16, PendingQoS1Message>>>,
    qos2_states: Arc<RwLock<HashMap<u16, QoS2State>>>,
    max_retries: u8,
}

impl QoSHandler {
    pub fn new() -> Self {
        Self {
            qos1_pending: Arc::new(RwLock::new(HashMap::new())),
            qos2_states: Arc::new(RwLock::new(HashMap::new())),
            max_retries: 3,
        }
    }

    /// Handle QoS 0: At most once delivery
    pub async fn handle_qo_s0(&self, message: MessageState) -> Result<()> {
        info!("QoS 0: Fire and forget delivery");
        // QoS 0 - no acknowledgment needed
        Ok(())
    }

    /// Handle QoS 1: At least once delivery
    pub async fn handle_qo_s1(&self, message: MessageState, client_id: &str) -> Result<()> {
        info!(
            "QoS 1: Storing message for at-least-once delivery, packet {}",
            message.packet_id
        );

        let pending = PendingQoS1Message {
            message: message.clone(),
            client_id: client_id.to_string(),
            retry_count: 0,
        };

        self.qos1_pending.write().await.insert(message.packet_id, pending);

        // Send PUBACK to client
        self.send_puback(client_id, message.packet_id).await?;

        Ok(())
    }

    /// Handle QoS 2: Exactly once delivery
    pub async fn handle_qo_s2(&self, message: MessageState, client_id: &str) -> Result<()> {
        info!("QoS 2: Starting exactly-once delivery handshake, packet {}", message.packet_id);

        // Store the message state
        self.qos2_states.write().await.insert(message.packet_id, QoS2State::Received);

        // Send PUBREC to client
        self.send_pubrec(client_id, message.packet_id).await?;

        Ok(())
    }

    /// Handle PUBACK (QoS 1 acknowledgment)
    pub async fn handle_puback(&self, packet_id: u16) -> Result<()> {
        if let Some(pending) = self.qos1_pending.write().await.remove(&packet_id) {
            info!("QoS 1: Received PUBACK for packet {}, delivery confirmed", packet_id);
        } else {
            warn!("QoS 1: Received PUBACK for unknown packet {}", packet_id);
        }
        Ok(())
    }

    /// Handle PUBREC (QoS 2 first acknowledgment)
    pub async fn handle_pubrec(&self, packet_id: u16, client_id: &str) -> Result<()> {
        let mut states = self.qos2_states.write().await;
        if let Some(state) = states.get_mut(&packet_id) {
            match state {
                QoS2State::Received => {
                    *state = QoS2State::Released;
                    info!("QoS 2: Received PUBREC for packet {}, sending PUBREL", packet_id);
                    // Send PUBREL to client
                    self.send_pubrel(client_id, packet_id).await?;
                }
                _ => {
                    warn!("QoS 2: Unexpected PUBREC for packet {} in state {:?}", packet_id, state);
                }
            }
        } else {
            warn!("QoS 2: Received PUBREC for unknown packet {}", packet_id);
        }
        Ok(())
    }

    /// Handle PUBREL (QoS 2 release)
    pub async fn handle_pubrel(&self, packet_id: u16, client_id: &str) -> Result<()> {
        let mut states = self.qos2_states.write().await;
        if let Some(state) = states.get_mut(&packet_id) {
            match state {
                QoS2State::Released => {
                    states.remove(&packet_id);
                    info!("QoS 2: Received PUBREL for packet {}, sending PUBCOMP", packet_id);
                    // Send PUBCOMP to client
                    self.send_pubcomp(client_id, packet_id).await?;
                }
                _ => {
                    warn!("QoS 2: Unexpected PUBREL for packet {} in state {:?}", packet_id, state);
                }
            }
        } else {
            warn!("QoS 2: Received PUBREL for unknown packet {}", packet_id);
        }
        Ok(())
    }

    /// Handle PUBCOMP (QoS 2 completion)
    pub async fn handle_pubcomp(&self, packet_id: u16) -> Result<()> {
        if self.qos2_states.write().await.remove(&packet_id).is_some() {
            info!("QoS 2: Received PUBCOMP for packet {}, delivery completed", packet_id);
        } else {
            warn!("QoS 2: Received PUBCOMP for unknown packet {}", packet_id);
        }
        Ok(())
    }

    /// Send PUBACK packet to client (QoS 1 acknowledgment)
    async fn send_puback(&self, client_id: &str, packet_id: u16) -> Result<()> {
        info!("QoS 1: Sending PUBACK for packet {} to client {}", packet_id, client_id);
        // In a real implementation, this would send the actual MQTT PUBACK packet
        // For the management layer, we simulate the send
        Ok(())
    }

    /// Send PUBREC packet to client (QoS 2 first acknowledgment)
    async fn send_pubrec(&self, client_id: &str, packet_id: u16) -> Result<()> {
        info!("QoS 2: Sending PUBREC for packet {} to client {}", packet_id, client_id);
        // In a real implementation, this would send the actual MQTT PUBREC packet
        // For the management layer, we simulate the send
        Ok(())
    }

    /// Send PUBREL packet to client (QoS 2 release)
    async fn send_pubrel(&self, client_id: &str, packet_id: u16) -> Result<()> {
        info!("QoS 2: Sending PUBREL for packet {} to client {}", packet_id, client_id);
        // In a real implementation, this would send the actual MQTT PUBREL packet
        // For the management layer, we simulate the send
        Ok(())
    }

    /// Send PUBCOMP packet to client (QoS 2 completion)
    async fn send_pubcomp(&self, client_id: &str, packet_id: u16) -> Result<()> {
        info!("QoS 2: Sending PUBCOMP for packet {} to client {}", packet_id, client_id);
        // In a real implementation, this would send the actual MQTT PUBCOMP packet
        // For the management layer, we simulate the send
        Ok(())
    }

    /// Retry pending QoS 1 messages
    pub async fn retry_pending_messages(&self) -> Result<()> {
        let mut pending = self.qos1_pending.write().await;
        let mut to_retry = Vec::new();

        for (packet_id, message) in pending.iter_mut() {
            if message.retry_count < self.max_retries {
                message.retry_count += 1;
                to_retry.push((*packet_id, message.client_id.clone()));
                info!(
                    "Retrying QoS 1 message for packet {} (attempt {})",
                    packet_id, message.retry_count
                );
            } else {
                warn!("QoS 1 message for packet {} exceeded max retries", packet_id);
            }
        }

        // Resend the messages
        for (packet_id, client_id) in to_retry {
            info!("Resending QoS 1 message for packet {} to client {}", packet_id, client_id);
            // In a real implementation, this would resend the PUBLISH packet
        }

        Ok(())
    }
}
