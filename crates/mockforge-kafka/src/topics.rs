use std::sync::Arc;

use crate::partitions::Partition;
use crate::fixtures::KafkaFixture;

/// Represents a Kafka topic
#[derive(Debug)]
pub struct Topic {
    pub name: String,
    pub partitions: Vec<Partition>,
    pub config: TopicConfig,
    pub fixtures: Vec<Arc<KafkaFixture>>,
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
            retention_ms: 604800000, // 7 days
            max_message_bytes: 1048576, // 1MB
        }
    }
}

impl Topic {
    /// Create a new topic
    pub fn new(name: String, config: TopicConfig) -> Self {
        let partitions = (0..config.num_partitions)
            .map(|id| Partition::new(id))
            .collect();

        Self {
            name,
            partitions,
            config,
            fixtures: vec![],
        }
    }

    /// Assign partition for a message based on key
    pub fn assign_partition(&self, key: Option<&[u8]>) -> i32 {
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
                // TODO: Implement round-robin counter
                0
            }
        }
    }

    /// Produce a record to the appropriate partition
    pub async fn produce(&mut self, partition: i32, record: crate::partitions::KafkaMessage) -> mockforge_core::Result<i64> {
        if let Some(partition) = self.partitions.get_mut(partition as usize) {
            Ok(partition.append(record))
        } else {
            Err(mockforge_core::Error::generic(format!("Partition {} does not exist", partition)))
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
