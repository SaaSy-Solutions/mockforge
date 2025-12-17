use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::fixtures::KafkaFixture;
use crate::partitions::Partition;

/// Represents a Kafka topic
#[derive(Debug)]
pub struct Topic {
    pub name: String,
    pub partitions: Vec<Partition>,
    pub config: TopicConfig,
    pub fixtures: Vec<Arc<KafkaFixture>>,
    round_robin_counter: AtomicUsize,
}

#[derive(Debug, Clone)]
pub struct TopicConfig {
    pub num_partitions: i32,
    pub replication_factor: i16,
    pub retention_ms: i64,
    pub max_message_bytes: i32,
}

impl Default for TopicConfig {
    fn default() -> Self {
        Self {
            num_partitions: 3,
            replication_factor: 1,
            retention_ms: 604800000,    // 7 days
            max_message_bytes: 1048576, // 1MB
        }
    }
}

impl Topic {
    /// Create a new topic
    pub fn new(name: String, config: TopicConfig) -> Self {
        let partitions = (0..config.num_partitions).map(Partition::new).collect();

        Self {
            name,
            partitions,
            config,
            fixtures: vec![],
            round_robin_counter: AtomicUsize::new(0),
        }
    }

    /// Assign partition for a message based on key
    pub fn assign_partition(&mut self, key: Option<&[u8]>) -> i32 {
        match key {
            Some(key_bytes) => {
                // Use murmur hash for partition assignment
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                let mut hasher = DefaultHasher::new();
                key_bytes.hash(&mut hasher);
                let hash = hasher.finish();
                (hash % self.config.num_partitions as u64) as i32
            }
            None => {
                // Round-robin for messages without keys
                let partition = self.round_robin_counter.fetch_add(1, Ordering::Relaxed)
                    % self.config.num_partitions as usize;
                partition as i32
            }
        }
    }

    /// Produce a record to the appropriate partition
    pub async fn produce(
        &mut self,
        partition: i32,
        record: crate::partitions::KafkaMessage,
    ) -> mockforge_core::Result<i64> {
        if let Some(partition) = self.partitions.get_mut(partition as usize) {
            Ok(partition.append(record))
        } else {
            Err(mockforge_core::Error::generic(format!(
                "Partition {} does not exist",
                partition
            )))
        }
    }

    /// Get partition by ID
    pub fn get_partition(&self, id: i32) -> Option<&Partition> {
        self.partitions.get(id as usize)
    }

    /// Get mutable partition by ID
    pub fn get_partition_mut(&mut self, id: i32) -> Option<&mut Partition> {
        self.partitions.get_mut(id as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topic_config_default() {
        let config = TopicConfig::default();
        assert_eq!(config.num_partitions, 3);
        assert_eq!(config.replication_factor, 1);
        assert_eq!(config.retention_ms, 604800000);
        assert_eq!(config.max_message_bytes, 1048576);
    }

    #[test]
    fn test_topic_config_clone() {
        let config = TopicConfig {
            num_partitions: 5,
            replication_factor: 3,
            retention_ms: 86400000,
            max_message_bytes: 2097152,
        };
        let cloned = config.clone();
        assert_eq!(config.num_partitions, cloned.num_partitions);
        assert_eq!(config.replication_factor, cloned.replication_factor);
    }

    #[test]
    fn test_topic_new() {
        let config = TopicConfig::default();
        let topic = Topic::new("test-topic".to_string(), config);

        assert_eq!(topic.name, "test-topic");
        assert_eq!(topic.partitions.len(), 3);
        assert!(topic.fixtures.is_empty());
    }

    #[test]
    fn test_topic_new_custom_partitions() {
        let config = TopicConfig {
            num_partitions: 10,
            ..Default::default()
        };
        let topic = Topic::new("test".to_string(), config);
        assert_eq!(topic.partitions.len(), 10);
    }

    #[test]
    fn test_topic_assign_partition_with_key() {
        let config = TopicConfig {
            num_partitions: 5,
            ..Default::default()
        };
        let mut topic = Topic::new("test".to_string(), config);

        // Same key should always get the same partition
        let key = b"user-123";
        let partition1 = topic.assign_partition(Some(key));
        let partition2 = topic.assign_partition(Some(key));
        assert_eq!(partition1, partition2);

        // Partition should be in valid range
        assert!(partition1 >= 0 && partition1 < 5);
    }

    #[test]
    fn test_topic_assign_partition_without_key() {
        let config = TopicConfig {
            num_partitions: 3,
            ..Default::default()
        };
        let mut topic = Topic::new("test".to_string(), config);

        // Without key, should round-robin
        let p1 = topic.assign_partition(None);
        let p2 = topic.assign_partition(None);
        let p3 = topic.assign_partition(None);
        let p4 = topic.assign_partition(None);

        // All should be in valid range
        assert!(p1 >= 0 && p1 < 3);
        assert!(p2 >= 0 && p2 < 3);
        assert!(p3 >= 0 && p3 < 3);
        assert!(p4 >= 0 && p4 < 3);

        // Should cycle through partitions
        assert_eq!(p1, 0);
        assert_eq!(p2, 1);
        assert_eq!(p3, 2);
        assert_eq!(p4, 0); // wraps around
    }

    #[test]
    fn test_topic_get_partition() {
        let config = TopicConfig::default();
        let topic = Topic::new("test".to_string(), config);

        assert!(topic.get_partition(0).is_some());
        assert!(topic.get_partition(1).is_some());
        assert!(topic.get_partition(2).is_some());
        assert!(topic.get_partition(3).is_none());
    }

    #[test]
    fn test_topic_get_partition_mut() {
        let config = TopicConfig::default();
        let mut topic = Topic::new("test".to_string(), config);

        assert!(topic.get_partition_mut(0).is_some());
        assert!(topic.get_partition_mut(100).is_none());
    }

    #[test]
    fn test_different_keys_may_get_different_partitions() {
        let config = TopicConfig {
            num_partitions: 10,
            ..Default::default()
        };
        let mut topic = Topic::new("test".to_string(), config);

        // Different keys should potentially get different partitions
        // (though they could happen to hash to the same one)
        let partitions: Vec<i32> = (0..100)
            .map(|i| {
                let key = format!("key-{}", i);
                topic.assign_partition(Some(key.as_bytes()))
            })
            .collect();

        // Should have some variety (not all same partition)
        let unique_partitions: std::collections::HashSet<_> = partitions.iter().collect();
        assert!(unique_partitions.len() > 1);
    }

    #[test]
    fn test_topic_debug() {
        let config = TopicConfig::default();
        let topic = Topic::new("debug-test".to_string(), config);
        let debug = format!("{:?}", topic);
        assert!(debug.contains("Topic"));
        assert!(debug.contains("debug-test"));
    }

    #[test]
    fn test_topic_config_debug() {
        let config = TopicConfig::default();
        let debug = format!("{:?}", config);
        assert!(debug.contains("TopicConfig"));
        assert!(debug.contains("num_partitions"));
    }
}
