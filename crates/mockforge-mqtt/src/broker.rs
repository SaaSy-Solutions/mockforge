use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use crate::topics::TopicTree;
use crate::spec_registry::MqttSpecRegistry;
use crate::qos::QoSHandler;

/// MQTT protocol version
#[derive(Debug, Clone, Copy)]
pub enum MqttVersion {
    V3_1_1,
    V5_0,
}

impl Default for MqttVersion {
    fn default() -> Self {
        MqttVersion::V5_0 // Default to v5.0 for better features
    }
}

/// MQTT broker configuration
#[derive(Debug, Clone)]
pub struct MqttConfig {
    pub port: u16,
    pub host: String,
    pub max_connections: usize,
    pub max_packet_size: usize,
    pub keep_alive_secs: u16,
    pub version: MqttVersion,
}

impl Default for MqttConfig {
    fn default() -> Self {
        Self {
            port: 1883,
            host: "0.0.0.0".to_string(),
            max_connections: 1000,
            max_packet_size: 1024 * 1024, // 1MB
            keep_alive_secs: 60,
            version: MqttVersion::default(),
        }
    }
}

/// Client session state
#[derive(Debug, Clone)]
pub struct ClientSession {
    pub client_id: String,
    pub subscriptions: HashMap<String, u8>, // topic_filter -> qos
    pub clean_session: bool,
    pub connected_at: u64,
    pub last_seen: u64,
}

/// Client state for session management
#[derive(Debug)]
pub struct ClientState {
    pub session: ClientSession,
    pub pending_messages: Vec<crate::qos::MessageState>, // Messages to send when client reconnects
}

/// MQTT broker implementation
pub struct MqttBroker {
    config: MqttConfig,
    topics: Arc<RwLock<TopicTree>>,
    clients: Arc<RwLock<HashMap<String, ClientState>>>,
    session_store: Arc<RwLock<HashMap<String, ClientSession>>>,
    qos_handler: QoSHandler,
    fixture_registry: Arc<RwLock<MqttFixtureRegistry>>,
    next_packet_id: Arc<RwLock<u16>>,
    metrics_registry: Option<Arc<mockforge_observability::prometheus::MetricsRegistry>>,
}

impl MqttBroker {
    pub fn new(config: MqttConfig, spec_registry: Arc<MqttSpecRegistry>) -> Self {
        Self {
            config,
            topics: Arc::new(RwLock::new(TopicTree::new())),
            clients: Arc::new(RwLock::new(HashMap::new())),
            session_store: Arc::new(RwLock::new(HashMap::new())),
            qos_handler: QoSHandler::new(),
            fixture_registry: Arc::new(RwLock::new(MqttFixtureRegistry::new())),
            next_packet_id: Arc::new(RwLock::new(1)),
            metrics_registry: None,
        }
    }

    pub fn with_metrics(mut self, metrics_registry: Arc<mockforge_observability::prometheus::MetricsRegistry>) -> Self {
        self.metrics_registry = Some(metrics_registry);
        self
    }

    /// Handle client connection with session management
    pub async fn client_connect(&self, client_id: &str, clean_session: bool) -> Result<(), Box<dyn std::error::Error>> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();

        let mut clients = self.clients.write().await;
        let mut sessions = self.session_store.write().await;

        if let Some(existing_client) = clients.get(client_id) {
            // Client already connected - this shouldn't happen in normal operation
            info!("Client {} already connected, updating session", client_id);
        }

        let session = if clean_session {
            // Clean session: create new session
            sessions.remove(client_id); // Remove any existing persistent session
            ClientSession {
                client_id: client_id.to_string(),
                subscriptions: HashMap::new(),
                clean_session: true,
                connected_at: now,
                last_seen: now,
            }
        } else {
            // Persistent session: restore or create
            if let Some(persistent_session) = sessions.get(client_id) {
                let mut restored_session = persistent_session.clone();
                restored_session.connected_at = now;
                restored_session.last_seen = now;
                restored_session.clean_session = false;
                restored_session
            } else {
                ClientSession {
                    client_id: client_id.to_string(),
                    subscriptions: HashMap::new(),
                    clean_session: false,
                    connected_at: now,
                    last_seen: now,
                }
            }
        };

        let client_state = ClientState {
            session: session.clone(),
            pending_messages: Vec::new(),
        };

        clients.insert(client_id.to_string(), client_state);

        // Record metrics
        if let Some(metrics) = &self.metrics_registry {
            metrics.mqtt_connections_active.inc();
            metrics.mqtt_connections_total.inc();
        }

        info!("Client {} connected with clean_session: {}", client_id, clean_session);
        Ok(())
    }

    /// Handle client disconnection with session persistence
    pub async fn client_disconnect(&self, client_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();

        let mut clients = self.clients.write().await;
        let mut sessions = self.session_store.write().await;

        if let Some(client_state) = clients.remove(client_id) {
            let session = client_state.session;

            if !session.clean_session {
                // Persist session for non-clean sessions
                let mut persistent_session = session.clone();
                persistent_session.last_seen = now;
                sessions.insert(client_id.to_string(), persistent_session);

                info!("Persisted session for client {}", client_id);
            } else {
                // Clean up subscriptions for clean sessions
                let mut topics = self.topics.write().await;
                for filter in session.subscriptions.keys() {
                    topics.unsubscribe(filter, client_id);
                }

                info!("Cleaned up session for client {}", client_id);
            }
        }

        // Record metrics
        if let Some(metrics) = &self.metrics_registry {
            metrics.mqtt_connections_active.dec();
        }

        Ok(())
    }

    /// Subscribe client to topics with session persistence
    pub async fn client_subscribe(&self, client_id: &str, topics: Vec<(String, u8)>) -> Result<(), Box<dyn std::error::Error>> {
        let mut clients = self.clients.write().await;
        let mut broker_topics = self.topics.write().await;

        if let Some(client_state) = clients.get_mut(client_id) {
            for (filter, qos) in topics {
                broker_topics.subscribe(&filter, qos, client_id);
                client_state.session.subscriptions.insert(filter.clone(), qos);

                // Send retained messages for new subscriptions
                let retained_messages = broker_topics.get_retained_for_filter(&filter);
                for (topic, message) in retained_messages {
                    info!("Sending retained message for topic {} to client {}", topic, client_id);
                    let qos_level = crate::qos::QoS::from_u8(message.qos).unwrap_or(crate::qos::QoS::AtMostOnce);
                    if let Err(e) = self.route_message_to_client(client_id, topic, &message.payload, qos_level).await {
                        warn!("Failed to deliver retained message to client {}: {}", client_id, e);
                    }
                }
            }

            // Update persistent session if not clean
            if !client_state.session.clean_session {
                let mut sessions = self.session_store.write().await;
                if let Some(session) = sessions.get_mut(client_id) {
                    session.subscriptions.clone_from(&client_state.session.subscriptions);
                }
            }
        }

        Ok(())
    }

    /// Unsubscribe client from topics
    pub async fn client_unsubscribe(&self, client_id: &str, filters: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
        let mut clients = self.clients.write().await;
        let mut broker_topics = self.topics.write().await;

        if let Some(client_state) = clients.get_mut(client_id) {
            for filter in filters {
                broker_topics.unsubscribe(&filter, client_id);
                client_state.session.subscriptions.remove(&filter);
            }

            // Update persistent session if not clean
            if !client_state.session.clean_session {
                let mut sessions = self.session_store.write().await;
                if let Some(session) = sessions.get_mut(client_id) {
                    session.subscriptions.clone_from(&client_state.session.subscriptions);
                }
            }
        }

        Ok(())
    }

    /// Get broker configuration (for testing)
    pub fn config(&self) -> &MqttConfig {
        &self.config
    }

    /// Get list of active topics (subscription filters and retained topics)
    pub async fn get_active_topics(&self) -> Vec<String> {
        let topics = self.topics.read().await;
        let mut all_topics = topics.get_all_topic_filters();
        all_topics.extend(topics.get_all_retained_topics());
        all_topics.sort();
        all_topics.dedup();
        all_topics
    }

    /// Get list of connected clients
    pub async fn get_connected_clients(&self) -> Vec<String> {
        let clients = self.clients.read().await;
        clients.keys().cloned().collect()
    }

    /// Get client information
    pub async fn get_client_info(&self, client_id: &str) -> Option<ClientSession> {
        let clients = self.clients.read().await;
        clients.get(client_id).map(|state| state.session.clone())
    }

    /// Disconnect a client
    pub async fn disconnect_client(&self, client_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.client_disconnect(client_id).await
    }

    /// Get topic statistics
    pub async fn get_topic_stats(&self) -> crate::topics::TopicStats {
        let topics = self.topics.read().await;
        topics.stats()
    }

    /// Generate next packet ID
    pub async fn next_packet_id(&self) -> u16 {
        let mut packet_id = self.next_packet_id.write().await;
        let id = *packet_id;
        *packet_id = packet_id.wrapping_add(1);
        if *packet_id == 0 {
            *packet_id = 1; // Skip 0 as it's reserved
        }
        id
    }

    pub async fn handle_publish(
        &self,
        client_id: &str,
        topic: &str,
        payload: Vec<u8>,
        qos: u8,
        retain: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        info!("Handling publish to topic: {} with QoS: {}", topic, qos);

        let qos_level = crate::qos::QoS::from_u8(qos).unwrap_or(crate::qos::QoS::AtMostOnce);

        let packet_id = if qos_level != crate::qos::QoS::AtMostOnce {
            self.next_packet_id().await
        } else {
            0 // QoS 0 doesn't use packet IDs
        };

        let message_state = crate::qos::MessageState {
            packet_id,
            topic: topic.to_string(),
            payload: payload.clone(),
            qos: qos_level,
            retained: retain,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs(),
        };

        // Handle retained messages
        if retain {
            let mut topics = self.topics.write().await;
            topics.retain_message(topic, payload.clone(), qos);
            info!("Stored retained message for topic: {}", topic);
        }

        // Handle based on QoS level
        match qos_level {
            crate::qos::QoS::AtMostOnce => {
                self.qos_handler.handle_qo_s0(message_state).await?;
            }
            crate::qos::QoS::AtLeastOnce => {
                self.qos_handler.handle_qo_s1(message_state, client_id).await?;
            }
            crate::qos::QoS::ExactlyOnce => {
                self.qos_handler.handle_qo_s2(message_state, client_id).await?;
            }
        }

        // Check if this matches any fixtures
        if let Some(fixture) = self.spec_registry.find_fixture_by_topic(topic) {
            info!("Found matching fixture: {}", fixture.identifier);

            // Generate response using template engine
            match self.generate_fixture_response(fixture, topic, &payload) {
                Ok(response_payload) => {
                    info!("Generated fixture response with {} bytes", response_payload.len());
                    // TODO: Publish the response to the appropriate topic
                }
                Err(e) => {
                    warn!("Failed to generate fixture response: {}", e);
                }
            }
        }

        // Route to subscribers
        let topics_read = self.topics.read().await;
        let subscribers = topics_read.match_topic(topic);
        for subscriber in &subscribers {
            info!("Routing to subscriber: {} on topic filter: {}", subscriber.client_id, subscriber.filter);
            self.route_message_to_client(&subscriber.client_id, topic, &payload, qos_level).await?;
        }

        // Record metrics
        if let Some(metrics) = &self.metrics_registry {
            metrics.mqtt_messages_published_total.inc();
        }

        Ok(())
    }

    /// Generate a response payload from a fixture using template expansion
    fn generate_fixture_response(&self, fixture: &crate::fixtures::MqttFixture, topic: &str, received_payload: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        use mockforge_core::templating;

        // Create templating context with environment variables
        let mut env_vars = std::collections::HashMap::new();
        env_vars.insert("topic".to_string(), topic.to_string());

        // Try to parse received payload as JSON and add it to context
        if let Ok(received_json) = serde_json::from_slice::<serde_json::Value>(received_payload) {
            env_vars.insert("payload".to_string(), received_json.to_string());
        } else {
            // If not JSON, add as string
            env_vars.insert("payload".to_string(), String::from_utf8_lossy(received_payload).to_string());
        }

        let context = templating::TemplatingContext::with_env(env_vars);

        // Use template engine to render payload
        let template_str = serde_json::to_string(&fixture.response.payload)?;
        let expanded_payload = templating::expand_str_with_context(&template_str, &context);

        Ok(expanded_payload.into_bytes())
    }

    /// Route a message to a specific client
    async fn route_message_to_client(&self, client_id: &str, topic: &str, payload: &[u8], qos: crate::qos::QoS) -> Result<(), Box<dyn std::error::Error>> {
        // Check if client is connected
        let clients = self.clients.read().await;
        if let Some(client_state) = clients.get(client_id) {
            info!("Delivering message to connected client {} on topic {}", client_id, topic);

            // In a real implementation, this would send the actual MQTT PUBLISH packet to the client
            // For the management layer, we simulate the delivery and record metrics

            // Record metrics
            if let Some(metrics) = &self.metrics_registry {
                metrics.mqtt_messages_received_total.inc();
            }

            // Add to client's pending messages if QoS requires it
            if qos != crate::qos::QoS::AtMostOnce {
                let mut pending_messages = client_state.pending_messages.clone();
                let message_state = crate::qos::MessageState {
                    packet_id: 0, // Would be assigned by actual MQTT protocol
                    topic: topic.to_string(),
                    payload: payload.to_vec(),
                    qos,
                    retained: false,
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)?
                        .as_secs(),
                };
                pending_messages.push(message_state);

                // Update client state (in real implementation, this would be handled by the MQTT protocol)
                // For simulation purposes, we just log
                info!("Added QoS {} message to pending delivery queue for client {}", qos.as_u8(), client_id);
            }

            Ok(())
        } else {
            warn!("Cannot route message to disconnected client: {}", client_id);
            Err(format!("Client {} is not connected", client_id).into())
        }
    }

    /// Update Prometheus metrics with current broker statistics
    pub async fn update_metrics(&self) {
        if let Some(metrics) = &self.metrics_registry {
            let connected_clients = self.get_connected_clients().await.len() as i64;
            let active_topics = self.get_active_topics().await.len() as i64;
            let topic_stats = self.get_topic_stats().await;

            metrics.mqtt_connections_active.set(connected_clients);
            metrics.mqtt_topics_active.set(active_topics);
            metrics.mqtt_subscriptions_active.set(topic_stats.total_subscriptions as i64);
            metrics.mqtt_retained_messages.set(topic_stats.retained_messages as i64);
        }
    }
}
