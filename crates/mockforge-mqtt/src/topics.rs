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

        self.subscriptions
            .entry(filter.to_string())
            .or_insert_with(Vec::new)
            .push(subscription);
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
                    .unwrap()
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
            .unwrap()
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
