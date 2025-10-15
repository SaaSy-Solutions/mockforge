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
