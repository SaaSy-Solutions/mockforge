use std::collections::VecDeque;

/// Represents a Kafka partition
#[derive(Debug)]
pub struct Partition {
    pub id: i32,
    pub messages: VecDeque<KafkaMessage>,
    pub high_watermark: i64,
    pub log_start_offset: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct KafkaMessage {
    pub offset: i64,
    pub timestamp: i64,
    pub key: Option<Vec<u8>>,
    pub value: Vec<u8>,
    pub headers: Vec<(String, Vec<u8>)>,
}

impl Partition {
    /// Create a new partition
    pub fn new(id: i32) -> Self {
        Self {
            id,
            messages: VecDeque::new(),
            high_watermark: 0,
            log_start_offset: 0,
        }
    }

    /// Append a message to the partition
    pub fn append(&mut self, message: KafkaMessage) -> i64 {
        let offset = self.high_watermark;
        self.messages.push_back(message);
        self.high_watermark += 1;
        offset
    }

    /// Fetch messages from a given offset
    pub fn fetch(&self, offset: i64, max_bytes: i32) -> Vec<&KafkaMessage> {
        let start_idx = (offset - self.log_start_offset) as usize;
        let mut result = Vec::new();
        let mut total_bytes = 0;

        for message in self.messages.iter().skip(start_idx) {
            if total_bytes + message.value.len() as i32 > max_bytes && !result.is_empty() {
                break;
            }
            result.push(message);
            total_bytes += message.value.len() as i32;
        }

        result
    }

    /// Get the latest offset
    pub fn latest_offset(&self) -> i64 {
        self.high_watermark - 1
    }

    /// Check if partition has messages from offset
    pub fn has_offset(&self, offset: i64) -> bool {
        offset >= self.log_start_offset && offset < self.high_watermark
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_message(offset: i64, value: &[u8]) -> KafkaMessage {
        KafkaMessage {
            offset,
            timestamp: 1234567890,
            key: None,
            value: value.to_vec(),
            headers: vec![],
        }
    }

    #[test]
    fn test_partition_new() {
        let partition = Partition::new(5);
        assert_eq!(partition.id, 5);
        assert!(partition.messages.is_empty());
        assert_eq!(partition.high_watermark, 0);
        assert_eq!(partition.log_start_offset, 0);
    }

    #[test]
    fn test_partition_append() {
        let mut partition = Partition::new(0);
        let msg = create_test_message(0, b"test message");

        let offset = partition.append(msg);
        assert_eq!(offset, 0);
        assert_eq!(partition.high_watermark, 1);
        assert_eq!(partition.messages.len(), 1);
    }

    #[test]
    fn test_partition_append_multiple() {
        let mut partition = Partition::new(0);

        let offset1 = partition.append(create_test_message(0, b"msg1"));
        let offset2 = partition.append(create_test_message(1, b"msg2"));
        let offset3 = partition.append(create_test_message(2, b"msg3"));

        assert_eq!(offset1, 0);
        assert_eq!(offset2, 1);
        assert_eq!(offset3, 2);
        assert_eq!(partition.high_watermark, 3);
    }

    #[test]
    fn test_partition_fetch() {
        let mut partition = Partition::new(0);
        partition.append(create_test_message(0, b"msg1"));
        partition.append(create_test_message(1, b"msg2"));
        partition.append(create_test_message(2, b"msg3"));

        let messages = partition.fetch(0, 1000);
        assert_eq!(messages.len(), 3);
    }

    #[test]
    fn test_partition_fetch_from_offset() {
        let mut partition = Partition::new(0);
        partition.append(create_test_message(0, b"msg1"));
        partition.append(create_test_message(1, b"msg2"));
        partition.append(create_test_message(2, b"msg3"));

        let messages = partition.fetch(1, 1000);
        assert_eq!(messages.len(), 2);
    }

    #[test]
    fn test_partition_fetch_with_byte_limit() {
        let mut partition = Partition::new(0);
        partition.append(create_test_message(0, b"short"));
        partition.append(create_test_message(1, b"this is a longer message"));
        partition.append(create_test_message(2, b"another long message here"));

        // Limit to 10 bytes - should get first message at least
        let messages = partition.fetch(0, 10);
        assert!(messages.len() >= 1);
    }

    #[test]
    fn test_partition_latest_offset() {
        let mut partition = Partition::new(0);
        assert_eq!(partition.latest_offset(), -1); // Empty partition

        partition.append(create_test_message(0, b"msg1"));
        assert_eq!(partition.latest_offset(), 0);

        partition.append(create_test_message(1, b"msg2"));
        assert_eq!(partition.latest_offset(), 1);
    }

    #[test]
    fn test_partition_has_offset() {
        let mut partition = Partition::new(0);
        assert!(!partition.has_offset(0)); // Empty partition

        partition.append(create_test_message(0, b"msg1"));
        partition.append(create_test_message(1, b"msg2"));

        assert!(partition.has_offset(0));
        assert!(partition.has_offset(1));
        assert!(!partition.has_offset(2));
        assert!(!partition.has_offset(-1));
    }

    #[test]
    fn test_kafka_message_clone() {
        let msg = KafkaMessage {
            offset: 10,
            timestamp: 1234567890,
            key: Some(b"key".to_vec()),
            value: b"value".to_vec(),
            headers: vec![("header1".to_string(), b"hvalue".to_vec())],
        };

        let cloned = msg.clone();
        assert_eq!(msg.offset, cloned.offset);
        assert_eq!(msg.key, cloned.key);
        assert_eq!(msg.value, cloned.value);
        assert_eq!(msg.headers, cloned.headers);
    }

    #[test]
    fn test_kafka_message_serialize() {
        let msg = KafkaMessage {
            offset: 5,
            timestamp: 1234567890,
            key: Some(b"key".to_vec()),
            value: b"value".to_vec(),
            headers: vec![],
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"offset\":5"));
    }

    #[test]
    fn test_kafka_message_debug() {
        let msg = create_test_message(0, b"test");
        let debug = format!("{:?}", msg);
        assert!(debug.contains("KafkaMessage"));
    }

    #[test]
    fn test_partition_debug() {
        let partition = Partition::new(3);
        let debug = format!("{:?}", partition);
        assert!(debug.contains("Partition"));
        assert!(debug.contains("3"));
    }
}
