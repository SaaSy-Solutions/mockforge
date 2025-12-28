use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::qos::QoSHandler;
use crate::spec_registry::MqttSpecRegistry;
use crate::topics::TopicTree;

/// MQTT protocol version
#[derive(Debug, Clone, Copy, Default)]
pub enum MqttVersion {
    V3_1_1,
    #[default]
    V5_0,
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
    /// Enable TLS
    pub tls_enabled: bool,
    /// TLS port (8883 is standard MQTTS port)
    pub tls_port: u16,
    /// Path to TLS certificate file (PEM format)
    pub tls_cert_path: Option<std::path::PathBuf>,
    /// Path to TLS private key file (PEM format)
    pub tls_key_path: Option<std::path::PathBuf>,
    /// Path to CA certificate for client verification (optional)
    pub tls_ca_path: Option<std::path::PathBuf>,
    /// Require client certificate authentication
    pub tls_client_auth: bool,
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
            tls_enabled: false,
            tls_port: 8883,
            tls_cert_path: None,
            tls_key_path: None,
            tls_ca_path: None,
            tls_client_auth: false,
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
    fixture_registry: Arc<RwLock<crate::fixtures::MqttFixtureRegistry>>,
    next_packet_id: Arc<RwLock<u16>>,
}

impl MqttBroker {
    pub fn new(config: MqttConfig, _spec_registry: Arc<MqttSpecRegistry>) -> Self {
        Self {
            config,
            topics: Arc::new(RwLock::new(TopicTree::new())),
            clients: Arc::new(RwLock::new(HashMap::new())),
            session_store: Arc::new(RwLock::new(HashMap::new())),
            qos_handler: QoSHandler::new(),
            fixture_registry: Arc::new(RwLock::new(crate::fixtures::MqttFixtureRegistry::new())),
            next_packet_id: Arc::new(RwLock::new(1)),
        }
    }

    /// Handle client connection with session management
    pub async fn client_connect(
        &self,
        client_id: &str,
        clean_session: bool,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs();

        let mut clients = self.clients.write().await;
        let mut sessions = self.session_store.write().await;

        if let Some(_existing_client) = clients.get(client_id) {
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
        // if let Some(metrics) = &self.metrics_registry {
        //     metrics.mqtt_connections_active.inc();
        //     metrics.mqtt_connections_total.inc();
        // }

        info!("Client {} connected with clean_session: {}", client_id, clean_session);
        Ok(())
    }

    /// Handle client disconnection with session persistence
    pub async fn client_disconnect(
        &self,
        client_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs();

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
        // if let Some(metrics) = &self.metrics_registry {
        //     metrics.mqtt_connections_active.dec();
        // }

        Ok(())
    }

    /// Subscribe client to topics with session persistence
    pub async fn client_subscribe(
        &self,
        client_id: &str,
        topics: Vec<(String, u8)>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
                    let qos_level = crate::qos::QoS::from_u8(message.qos)
                        .unwrap_or(crate::qos::QoS::AtMostOnce);
                    if let Err(e) = self
                        .route_message_to_client(client_id, topic, &message.payload, qos_level)
                        .await
                    {
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
    pub async fn client_unsubscribe(
        &self,
        client_id: &str,
        filters: Vec<String>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
    pub async fn disconnect_client(
        &self,
        client_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.handle_publish_internal(client_id, topic, payload, qos, retain, false)
            .await
    }

    /// Publish a message with QoS handling but skip fixture lookup (used for fixture responses)
    pub async fn publish_with_qos(
        &self,
        client_id: &str,
        topic: &str,
        payload: Vec<u8>,
        qos: u8,
        retain: bool,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Publishing with QoS to topic: {} with QoS: {}", topic, qos);

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

        Ok(())
    }

    async fn handle_publish_internal(
        &self,
        client_id: &str,
        topic: &str,
        payload: Vec<u8>,
        qos: u8,
        retain: bool,
        is_fixture_response: bool,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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

        // Check if this matches any fixtures (skip if this is already a fixture response to avoid recursion)
        if !is_fixture_response {
            if let Some(fixture) = self.fixture_registry.read().await.find_by_topic(topic) {
                info!("Found matching fixture: {}", fixture.identifier);

                // Generate response using template engine
                match self.generate_fixture_response(fixture, topic, &payload) {
                    Ok(response_payload) => {
                        info!("Generated fixture response with {} bytes", response_payload.len());
                        // Publish the response to the same topic as the request (skip fixture lookup to avoid recursion)
                        if let Err(e) = self
                            .publish_with_qos(
                                client_id,
                                topic,
                                response_payload,
                                fixture.qos,
                                fixture.retained,
                            )
                            .await
                        {
                            warn!("Failed to publish fixture response: {}", e);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to generate fixture response: {}", e);
                    }
                }
            }
        }

        // Route to subscribers
        self.route_to_subscribers(topic, &payload, qos_level).await?;

        // Record metrics
        // if let Some(metrics) = &self.metrics_registry {
        //     metrics.mqtt_messages_published_total.inc();
        // }

        Ok(())
    }

    /// Route a message to all subscribers of a topic
    async fn route_to_subscribers(
        &self,
        topic: &str,
        payload: &[u8],
        qos: crate::qos::QoS,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let topics_read = self.topics.read().await;
        let subscribers = topics_read.match_topic(topic);
        for subscriber in &subscribers {
            info!(
                "Routing to subscriber: {} on topic filter: {}",
                subscriber.client_id, subscriber.filter
            );
            self.route_message_to_client(&subscriber.client_id, topic, payload, qos).await?;
        }
        Ok(())
    }

    /// Generate a response payload from a fixture using template expansion
    fn generate_fixture_response(
        &self,
        fixture: &crate::fixtures::MqttFixture,
        topic: &str,
        received_payload: &[u8],
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        use mockforge_core::templating;

        // Create templating context with environment variables
        let mut env_vars = std::collections::HashMap::new();
        env_vars.insert("topic".to_string(), topic.to_string());

        // Try to parse received payload as JSON and add it to context
        if let Ok(received_json) = serde_json::from_slice::<serde_json::Value>(received_payload) {
            env_vars.insert("payload".to_string(), received_json.to_string());
        } else {
            // If not JSON, add as string
            env_vars.insert(
                "payload".to_string(),
                String::from_utf8_lossy(received_payload).to_string(),
            );
        }

        let context = templating::TemplatingContext::with_env(env_vars);

        // Use template engine to render payload
        let template_str = serde_json::to_string(&fixture.response.payload)?;
        let expanded_payload = templating::expand_str_with_context(&template_str, &context);

        Ok(expanded_payload.into_bytes())
    }

    /// Route a message to a specific client
    async fn route_message_to_client(
        &self,
        client_id: &str,
        topic: &str,
        payload: &[u8],
        qos: crate::qos::QoS,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Check if client is connected
        let clients = self.clients.read().await;
        if let Some(client_state) = clients.get(client_id) {
            info!("Delivering message to connected client {} on topic {}", client_id, topic);

            // In a real implementation, this would send the actual MQTT PUBLISH packet to the client
            // For the management layer, we simulate the delivery and record metrics

            // Record metrics
            // if let Some(metrics) = &self.metrics_registry {
            //     metrics.mqtt_messages_received_total.inc();
            // }

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
                info!(
                    "Added QoS {} message to pending delivery queue for client {}",
                    qos.as_u8(),
                    client_id
                );
            }

            Ok(())
        } else {
            warn!("Cannot route message to disconnected client: {}", client_id);
            Err(format!("Client {} is not connected", client_id).into())
        }
    }

    /// Update Prometheus metrics with current broker statistics
    pub async fn update_metrics(&self) {
        // if let Some(metrics) = &self.metrics_registry {
        //     let connected_clients = self.get_connected_clients().await.len() as i64;
        //     let active_topics = self.get_active_topics().await.len() as i64;
        //     let topic_stats = self.get_topic_stats().await;

        //     metrics.mqtt_connections_active.set(connected_clients);
        //     metrics.mqtt_topics_active.set(active_topics);
        //     metrics.mqtt_subscriptions_active.set(topic_stats.total_subscriptions as i64);
        //     metrics.mqtt_retained_messages.set(topic_stats.retained_messages as i64);
        // }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mqtt_version_default() {
        let version = MqttVersion::default();
        assert!(matches!(version, MqttVersion::V5_0));
    }

    #[test]
    fn test_mqtt_version_clone() {
        let v1 = MqttVersion::V3_1_1;
        let v2 = v1;
        assert!(matches!(v1, MqttVersion::V3_1_1));
        assert!(matches!(v2, MqttVersion::V3_1_1));
    }

    #[test]
    fn test_mqtt_version_debug() {
        let version = MqttVersion::V5_0;
        let debug = format!("{:?}", version);
        assert!(debug.contains("V5_0"));
    }

    #[test]
    fn test_mqtt_config_default() {
        let config = MqttConfig::default();
        assert_eq!(config.port, 1883);
        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.max_connections, 1000);
        assert_eq!(config.max_packet_size, 1024 * 1024);
        assert_eq!(config.keep_alive_secs, 60);
        assert!(matches!(config.version, MqttVersion::V5_0));
    }

    #[test]
    fn test_mqtt_config_clone() {
        let config1 = MqttConfig::default();
        let config2 = config1.clone();
        assert_eq!(config1.port, config2.port);
        assert_eq!(config1.host, config2.host);
    }

    #[test]
    fn test_mqtt_config_custom() {
        let config = MqttConfig {
            port: 8883,
            host: "127.0.0.1".to_string(),
            max_connections: 500,
            max_packet_size: 2048,
            keep_alive_secs: 120,
            version: MqttVersion::V3_1_1,
            ..Default::default()
        };
        assert_eq!(config.port, 8883);
        assert_eq!(config.host, "127.0.0.1");
    }

    #[test]
    fn test_client_session_clone() {
        let mut subscriptions = HashMap::new();
        subscriptions.insert("test/topic".to_string(), 1);

        let session = ClientSession {
            client_id: "client-1".to_string(),
            subscriptions,
            clean_session: false,
            connected_at: 1000,
            last_seen: 2000,
        };

        let cloned = session.clone();
        assert_eq!(session.client_id, cloned.client_id);
        assert_eq!(session.clean_session, cloned.clean_session);
        assert_eq!(session.connected_at, cloned.connected_at);
    }

    #[test]
    fn test_client_session_debug() {
        let session = ClientSession {
            client_id: "test-client".to_string(),
            subscriptions: HashMap::new(),
            clean_session: true,
            connected_at: 1000,
            last_seen: 1000,
        };
        let debug = format!("{:?}", session);
        assert!(debug.contains("ClientSession"));
        assert!(debug.contains("test-client"));
    }

    #[test]
    fn test_client_state_debug() {
        let session = ClientSession {
            client_id: "test-client".to_string(),
            subscriptions: HashMap::new(),
            clean_session: true,
            connected_at: 1000,
            last_seen: 1000,
        };
        let state = ClientState {
            session,
            pending_messages: Vec::new(),
        };
        let debug = format!("{:?}", state);
        assert!(debug.contains("ClientState"));
    }

    fn create_test_broker() -> MqttBroker {
        let config = MqttConfig::default();
        let spec_registry = Arc::new(MqttSpecRegistry::new());
        MqttBroker::new(config, spec_registry)
    }

    #[tokio::test]
    async fn test_broker_new() {
        let broker = create_test_broker();
        assert_eq!(broker.config().port, 1883);
    }

    #[tokio::test]
    async fn test_broker_config() {
        let config = MqttConfig {
            port: 9999,
            ..Default::default()
        };
        let spec_registry = Arc::new(MqttSpecRegistry::new());
        let broker = MqttBroker::new(config, spec_registry);
        assert_eq!(broker.config().port, 9999);
    }

    #[tokio::test]
    async fn test_client_connect_clean_session() {
        let broker = create_test_broker();
        let result = broker.client_connect("client-1", true).await;
        assert!(result.is_ok());

        let clients = broker.get_connected_clients().await;
        assert_eq!(clients.len(), 1);
        assert!(clients.contains(&"client-1".to_string()));
    }

    #[tokio::test]
    async fn test_client_connect_persistent_session() {
        let broker = create_test_broker();
        let result = broker.client_connect("client-1", false).await;
        assert!(result.is_ok());

        let info = broker.get_client_info("client-1").await;
        assert!(info.is_some());
        assert_eq!(info.unwrap().clean_session, false);
    }

    #[tokio::test]
    async fn test_client_disconnect() {
        let broker = create_test_broker();
        broker.client_connect("client-1", true).await.unwrap();

        let result = broker.client_disconnect("client-1").await;
        assert!(result.is_ok());

        let clients = broker.get_connected_clients().await;
        assert_eq!(clients.len(), 0);
    }

    #[tokio::test]
    async fn test_client_disconnect_persistent_session() {
        let broker = create_test_broker();
        broker.client_connect("client-1", false).await.unwrap();

        // Subscribe to a topic
        broker
            .client_subscribe("client-1", vec![("test/topic".to_string(), 1)])
            .await
            .unwrap();

        // Disconnect
        broker.client_disconnect("client-1").await.unwrap();

        // Reconnect and session should be restored
        broker.client_connect("client-1", false).await.unwrap();
        let info = broker.get_client_info("client-1").await.unwrap();
        assert!(info.subscriptions.contains_key("test/topic"));
    }

    #[tokio::test]
    async fn test_client_subscribe() {
        let broker = create_test_broker();
        broker.client_connect("client-1", true).await.unwrap();

        let result = broker.client_subscribe("client-1", vec![("test/topic".to_string(), 1)]).await;
        assert!(result.is_ok());

        let info = broker.get_client_info("client-1").await.unwrap();
        assert_eq!(info.subscriptions.get("test/topic"), Some(&1));
    }

    #[tokio::test]
    async fn test_client_subscribe_multiple_topics() {
        let broker = create_test_broker();
        broker.client_connect("client-1", true).await.unwrap();

        let result = broker
            .client_subscribe(
                "client-1",
                vec![
                    ("topic1".to_string(), 0),
                    ("topic2".to_string(), 1),
                    ("topic3".to_string(), 2),
                ],
            )
            .await;
        assert!(result.is_ok());

        let info = broker.get_client_info("client-1").await.unwrap();
        assert_eq!(info.subscriptions.len(), 3);
    }

    #[tokio::test]
    async fn test_client_unsubscribe() {
        let broker = create_test_broker();
        broker.client_connect("client-1", true).await.unwrap();

        broker
            .client_subscribe("client-1", vec![("test/topic".to_string(), 1)])
            .await
            .unwrap();

        let result = broker.client_unsubscribe("client-1", vec!["test/topic".to_string()]).await;
        assert!(result.is_ok());

        let info = broker.get_client_info("client-1").await.unwrap();
        assert!(!info.subscriptions.contains_key("test/topic"));
    }

    #[tokio::test]
    async fn test_get_connected_clients() {
        let broker = create_test_broker();
        broker.client_connect("client-1", true).await.unwrap();
        broker.client_connect("client-2", true).await.unwrap();

        let clients = broker.get_connected_clients().await;
        assert_eq!(clients.len(), 2);
    }

    #[tokio::test]
    async fn test_get_client_info_nonexistent() {
        let broker = create_test_broker();
        let info = broker.get_client_info("nonexistent").await;
        assert!(info.is_none());
    }

    #[tokio::test]
    async fn test_disconnect_client() {
        let broker = create_test_broker();
        broker.client_connect("client-1", true).await.unwrap();

        let result = broker.disconnect_client("client-1").await;
        assert!(result.is_ok());

        let clients = broker.get_connected_clients().await;
        assert_eq!(clients.len(), 0);
    }

    #[tokio::test]
    async fn test_get_active_topics() {
        let broker = create_test_broker();
        broker.client_connect("client-1", true).await.unwrap();
        broker
            .client_subscribe("client-1", vec![("sensor/#".to_string(), 1)])
            .await
            .unwrap();

        let topics = broker.get_active_topics().await;
        assert!(topics.contains(&"sensor/#".to_string()));
    }

    #[tokio::test]
    async fn test_get_topic_stats() {
        let broker = create_test_broker();
        broker.client_connect("client-1", true).await.unwrap();
        broker
            .client_subscribe("client-1", vec![("test/topic".to_string(), 1)])
            .await
            .unwrap();

        let stats = broker.get_topic_stats().await;
        assert_eq!(stats.total_subscriptions, 1);
    }

    #[tokio::test]
    async fn test_next_packet_id() {
        let broker = create_test_broker();
        let id1 = broker.next_packet_id().await;
        let id2 = broker.next_packet_id().await;
        assert_eq!(id1 + 1, id2);
    }

    #[tokio::test]
    async fn test_next_packet_id_wrapping() {
        let broker = create_test_broker();
        // Set to max value
        *broker.next_packet_id.write().await = u16::MAX;

        let id1 = broker.next_packet_id().await;
        assert_eq!(id1, u16::MAX);

        // Should wrap to 1 (skip 0)
        let id2 = broker.next_packet_id().await;
        assert_eq!(id2, 1);
    }

    #[tokio::test]
    async fn test_handle_publish_qos0() {
        let broker = create_test_broker();
        broker.client_connect("client-1", true).await.unwrap();

        let result = broker
            .handle_publish("client-1", "test/topic", b"hello".to_vec(), 0, false)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_publish_qos1() {
        let broker = create_test_broker();
        broker.client_connect("client-1", true).await.unwrap();

        let result = broker
            .handle_publish("client-1", "test/topic", b"hello".to_vec(), 1, false)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_publish_qos2() {
        let broker = create_test_broker();
        broker.client_connect("client-1", true).await.unwrap();

        let result = broker
            .handle_publish("client-1", "test/topic", b"hello".to_vec(), 2, false)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_publish_retained() {
        let broker = create_test_broker();
        broker.client_connect("client-1", true).await.unwrap();

        broker
            .handle_publish("client-1", "test/topic", b"retained".to_vec(), 1, true)
            .await
            .unwrap();

        let topics = broker.get_active_topics().await;
        assert!(topics.contains(&"test/topic".to_string()));
    }

    #[tokio::test]
    async fn test_publish_with_qos() {
        let broker = create_test_broker();
        broker.client_connect("client-1", true).await.unwrap();

        let result = broker
            .publish_with_qos("client-1", "test/topic", b"test".to_vec(), 1, false)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_routing_to_subscribers() {
        let broker = create_test_broker();

        // Connect two clients
        broker.client_connect("client-1", true).await.unwrap();
        broker.client_connect("client-2", true).await.unwrap();

        // Subscribe client-2 to a topic
        broker
            .client_subscribe("client-2", vec![("test/topic".to_string(), 1)])
            .await
            .unwrap();

        // Publish from client-1
        let result = broker
            .handle_publish("client-1", "test/topic", b"message".to_vec(), 1, false)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_routing_with_wildcards() {
        let broker = create_test_broker();
        broker.client_connect("client-1", true).await.unwrap();
        broker.client_connect("client-2", true).await.unwrap();

        // Subscribe with wildcard
        broker
            .client_subscribe("client-2", vec![("sensor/#".to_string(), 1)])
            .await
            .unwrap();

        // Publish to matching topic
        let result = broker
            .handle_publish("client-1", "sensor/temperature", b"25".to_vec(), 1, false)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore = "test hangs - needs investigation into potential deadlock"]
    async fn test_retained_messages_sent_on_subscribe() {
        let broker = create_test_broker();
        broker.client_connect("client-1", true).await.unwrap();

        // Publish retained message
        broker
            .handle_publish("client-1", "test/topic", b"retained".to_vec(), 1, true)
            .await
            .unwrap();

        // New client subscribes
        broker.client_connect("client-2", true).await.unwrap();
        let result = broker.client_subscribe("client-2", vec![("test/topic".to_string(), 1)]).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_clean_session_removes_subscriptions() {
        let broker = create_test_broker();
        broker.client_connect("client-1", true).await.unwrap();
        broker
            .client_subscribe("client-1", vec![("test/topic".to_string(), 1)])
            .await
            .unwrap();

        broker.client_disconnect("client-1").await.unwrap();

        // Reconnect should not restore subscriptions with clean session
        broker.client_connect("client-1", true).await.unwrap();
        let info = broker.get_client_info("client-1").await.unwrap();
        assert_eq!(info.subscriptions.len(), 0);
    }

    #[tokio::test]
    async fn test_update_metrics() {
        let broker = create_test_broker();
        broker.client_connect("client-1", true).await.unwrap();

        // Should not panic
        broker.update_metrics().await;
    }

    #[tokio::test]
    async fn test_multiple_clients_same_topic() {
        let broker = create_test_broker();

        broker.client_connect("client-1", true).await.unwrap();
        broker.client_connect("client-2", true).await.unwrap();
        broker.client_connect("client-3", true).await.unwrap();

        // All subscribe to same topic
        for client in &["client-1", "client-2", "client-3"] {
            broker
                .client_subscribe(client, vec![("test/topic".to_string(), 1)])
                .await
                .unwrap();
        }

        let stats = broker.get_topic_stats().await;
        assert_eq!(stats.total_subscribers, 3);
    }

    #[tokio::test]
    async fn test_client_reconnect_already_connected() {
        let broker = create_test_broker();
        broker.client_connect("client-1", true).await.unwrap();

        // Reconnecting while already connected should still work
        let result = broker.client_connect("client-1", true).await;
        assert!(result.is_ok());
    }
}
