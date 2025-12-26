use regex::Regex;
use std::collections::HashMap;

/// Represents a subscription to a topic
#[derive(Debug, Clone)]
pub struct Subscription {
    pub filter: String,
    pub qos: u8,
    pub client_id: String,
}

/// Represents a retained message
#[derive(Debug, Clone)]
pub struct RetainedMessage {
    pub payload: Vec<u8>,
    pub qos: u8,
    pub timestamp: u64,
}

/// Topic tree for managing subscriptions and retained messages
pub struct TopicTree {
    subscriptions: HashMap<String, Vec<Subscription>>,
    retained: HashMap<String, RetainedMessage>,
}

impl Default for TopicTree {
    fn default() -> Self {
        Self::new()
    }
}

impl TopicTree {
    pub fn new() -> Self {
        Self {
            subscriptions: HashMap::new(),
            retained: HashMap::new(),
        }
    }

    /// Match a topic against all subscriptions
    pub fn match_topic(&self, topic: &str) -> Vec<&Subscription> {
        let mut matches = Vec::new();

        for subscriptions in self.subscriptions.values() {
            for subscription in subscriptions {
                if self.matches_filter(topic, &subscription.filter) {
                    matches.push(subscription);
                }
            }
        }

        matches
    }

    /// Check if a topic matches a filter (supports wildcards + and #)
    fn matches_filter(&self, topic: &str, filter: &str) -> bool {
        // Convert MQTT wildcard filter to regex
        let regex_pattern = filter
            .replace('+', "[^/]+")  // + matches any single level
            .replace("#", ".+")      // # matches any remaining levels
            .replace("$", "\\$"); // Escape $ for regex

        let regex = match Regex::new(&format!("^{}$", regex_pattern)) {
            Ok(r) => r,
            Err(_) => return false,
        };

        regex.is_match(topic)
    }

    /// Add a subscription
    pub fn subscribe(&mut self, filter: &str, qos: u8, client_id: &str) {
        let subscription = Subscription {
            filter: filter.to_string(),
            qos,
            client_id: client_id.to_string(),
        };

        self.subscriptions.entry(filter.to_string()).or_default().push(subscription);
    }

    /// Remove a subscription
    pub fn unsubscribe(&mut self, filter: &str, client_id: &str) {
        if let Some(subscriptions) = self.subscriptions.get_mut(filter) {
            subscriptions.retain(|s| s.client_id != client_id);
            if subscriptions.is_empty() {
                self.subscriptions.remove(filter);
            }
        }
    }

    /// Store a retained message
    pub fn retain_message(&mut self, topic: &str, payload: Vec<u8>, qos: u8) {
        if payload.is_empty() {
            // Empty payload removes retained message
            self.retained.remove(topic);
        } else {
            let message = RetainedMessage {
                payload,
                qos,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("system time before UNIX epoch")
                    .as_secs(),
            };
            self.retained.insert(topic.to_string(), message);
        }
    }

    /// Get retained message for a topic
    pub fn get_retained(&self, topic: &str) -> Option<&RetainedMessage> {
        self.retained.get(topic)
    }

    /// Get all retained messages that match a subscription filter
    pub fn get_retained_for_filter(&self, filter: &str) -> Vec<(&str, &RetainedMessage)> {
        self.retained
            .iter()
            .filter(|(topic, _)| self.matches_filter(topic, filter))
            .map(|(topic, message)| (topic.as_str(), message))
            .collect()
    }

    /// Clean up expired retained messages (basic implementation)
    pub fn cleanup_expired_retained(&mut self, max_age_secs: u64) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time before UNIX epoch")
            .as_secs();

        self.retained
            .retain(|_, message| now.saturating_sub(message.timestamp) < max_age_secs);
    }

    /// Get all topic filters (subscription patterns)
    pub fn get_all_topic_filters(&self) -> Vec<String> {
        self.subscriptions.keys().cloned().collect()
    }

    /// Get all retained message topics
    pub fn get_all_retained_topics(&self) -> Vec<String> {
        self.retained.keys().cloned().collect()
    }

    /// Get topic statistics
    pub fn stats(&self) -> TopicStats {
        TopicStats {
            total_subscriptions: self.subscriptions.len(),
            total_subscribers: self.subscriptions.values().map(|subs| subs.len()).sum(),
            retained_messages: self.retained.len(),
        }
    }
}

/// Topic tree statistics
#[derive(Debug, Clone)]
pub struct TopicStats {
    pub total_subscriptions: usize,
    pub total_subscribers: usize,
    pub retained_messages: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subscription_clone() {
        let sub = Subscription {
            filter: "test/topic".to_string(),
            qos: 1,
            client_id: "client-1".to_string(),
        };

        let cloned = sub.clone();
        assert_eq!(sub.filter, cloned.filter);
        assert_eq!(sub.qos, cloned.qos);
        assert_eq!(sub.client_id, cloned.client_id);
    }

    #[test]
    fn test_subscription_debug() {
        let sub = Subscription {
            filter: "sensor/#".to_string(),
            qos: 2,
            client_id: "sensor-client".to_string(),
        };
        let debug = format!("{:?}", sub);
        assert!(debug.contains("Subscription"));
        assert!(debug.contains("sensor/#"));
    }

    #[test]
    fn test_retained_message_clone() {
        let msg = RetainedMessage {
            payload: b"hello".to_vec(),
            qos: 1,
            timestamp: 1234567890,
        };

        let cloned = msg.clone();
        assert_eq!(msg.payload, cloned.payload);
        assert_eq!(msg.qos, cloned.qos);
        assert_eq!(msg.timestamp, cloned.timestamp);
    }

    #[test]
    fn test_retained_message_debug() {
        let msg = RetainedMessage {
            payload: b"test".to_vec(),
            qos: 0,
            timestamp: 0,
        };
        let debug = format!("{:?}", msg);
        assert!(debug.contains("RetainedMessage"));
    }

    #[test]
    fn test_topic_tree_new() {
        let tree = TopicTree::new();
        let stats = tree.stats();
        assert_eq!(stats.total_subscriptions, 0);
        assert_eq!(stats.total_subscribers, 0);
        assert_eq!(stats.retained_messages, 0);
    }

    #[test]
    fn test_topic_tree_default() {
        let tree = TopicTree::default();
        assert!(tree.get_all_topic_filters().is_empty());
    }

    #[test]
    fn test_subscribe() {
        let mut tree = TopicTree::new();
        tree.subscribe("sensor/temp", 1, "client-1");

        let stats = tree.stats();
        assert_eq!(stats.total_subscriptions, 1);
        assert_eq!(stats.total_subscribers, 1);
    }

    #[test]
    fn test_subscribe_multiple_clients() {
        let mut tree = TopicTree::new();
        tree.subscribe("sensor/temp", 1, "client-1");
        tree.subscribe("sensor/temp", 2, "client-2");

        let stats = tree.stats();
        assert_eq!(stats.total_subscriptions, 1);
        assert_eq!(stats.total_subscribers, 2);
    }

    #[test]
    fn test_unsubscribe() {
        let mut tree = TopicTree::new();
        tree.subscribe("sensor/temp", 1, "client-1");
        tree.subscribe("sensor/temp", 1, "client-2");

        tree.unsubscribe("sensor/temp", "client-1");

        let stats = tree.stats();
        assert_eq!(stats.total_subscribers, 1);
    }

    #[test]
    fn test_unsubscribe_removes_filter() {
        let mut tree = TopicTree::new();
        tree.subscribe("sensor/temp", 1, "client-1");
        tree.unsubscribe("sensor/temp", "client-1");

        let stats = tree.stats();
        assert_eq!(stats.total_subscriptions, 0);
    }

    #[test]
    fn test_match_topic_exact() {
        let mut tree = TopicTree::new();
        tree.subscribe("sensor/temp", 1, "client-1");

        let matches = tree.match_topic("sensor/temp");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].client_id, "client-1");
    }

    #[test]
    fn test_match_topic_plus_wildcard() {
        let mut tree = TopicTree::new();
        tree.subscribe("sensor/+/temp", 1, "client-1");

        let matches = tree.match_topic("sensor/room1/temp");
        assert_eq!(matches.len(), 1);

        // Should not match different depth
        let no_matches = tree.match_topic("sensor/temp");
        assert_eq!(no_matches.len(), 0);
    }

    #[test]
    fn test_match_topic_hash_wildcard() {
        let mut tree = TopicTree::new();
        tree.subscribe("sensor/#", 1, "client-1");

        let matches1 = tree.match_topic("sensor/temp");
        assert_eq!(matches1.len(), 1);

        let matches2 = tree.match_topic("sensor/room/temp/value");
        assert_eq!(matches2.len(), 1);
    }

    #[test]
    fn test_match_topic_no_match() {
        let mut tree = TopicTree::new();
        tree.subscribe("sensor/temp", 1, "client-1");

        let matches = tree.match_topic("actuator/temp");
        assert!(matches.is_empty());
    }

    #[test]
    fn test_retain_message() {
        let mut tree = TopicTree::new();
        tree.retain_message("sensor/temp", b"25.5".to_vec(), 1);

        let retained = tree.get_retained("sensor/temp");
        assert!(retained.is_some());
        assert_eq!(retained.unwrap().payload, b"25.5".to_vec());
    }

    #[test]
    fn test_retain_message_empty_removes() {
        let mut tree = TopicTree::new();
        tree.retain_message("sensor/temp", b"25.5".to_vec(), 1);
        tree.retain_message("sensor/temp", vec![], 0);

        let retained = tree.get_retained("sensor/temp");
        assert!(retained.is_none());
    }

    #[test]
    fn test_get_retained_for_filter() {
        let mut tree = TopicTree::new();
        tree.retain_message("sensor/temp", b"25.5".to_vec(), 1);
        tree.retain_message("sensor/humidity", b"60".to_vec(), 1);
        tree.retain_message("actuator/fan", b"on".to_vec(), 1);

        let matches = tree.get_retained_for_filter("sensor/#");
        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn test_cleanup_expired_retained() {
        let mut tree = TopicTree::new();
        tree.retain_message("sensor/temp", b"25.5".to_vec(), 1);

        // Cleanup with max age of 1 year - should not remove
        tree.cleanup_expired_retained(365 * 24 * 60 * 60);
        assert!(tree.get_retained("sensor/temp").is_some());
    }

    #[test]
    fn test_get_all_topic_filters() {
        let mut tree = TopicTree::new();
        tree.subscribe("sensor/temp", 1, "client-1");
        tree.subscribe("sensor/humidity", 1, "client-2");

        let filters = tree.get_all_topic_filters();
        assert_eq!(filters.len(), 2);
    }

    #[test]
    fn test_get_all_retained_topics() {
        let mut tree = TopicTree::new();
        tree.retain_message("topic1", b"msg1".to_vec(), 1);
        tree.retain_message("topic2", b"msg2".to_vec(), 1);

        let topics = tree.get_all_retained_topics();
        assert_eq!(topics.len(), 2);
    }

    #[test]
    fn test_topic_stats_clone() {
        let stats = TopicStats {
            total_subscriptions: 5,
            total_subscribers: 10,
            retained_messages: 3,
        };

        let cloned = stats.clone();
        assert_eq!(stats.total_subscriptions, cloned.total_subscriptions);
        assert_eq!(stats.total_subscribers, cloned.total_subscribers);
        assert_eq!(stats.retained_messages, cloned.retained_messages);
    }

    #[test]
    fn test_topic_stats_debug() {
        let stats = TopicStats {
            total_subscriptions: 1,
            total_subscribers: 2,
            retained_messages: 3,
        };
        let debug = format!("{:?}", stats);
        assert!(debug.contains("TopicStats"));
    }
}
