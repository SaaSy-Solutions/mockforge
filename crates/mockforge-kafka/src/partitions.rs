use std::collections::VecDeque;

/// Current wall-clock time in epoch milliseconds, saturating to 0 before the
/// epoch. Used to stamp message ingestion times for retention.
fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Hard cap on the number of retained messages per partition. Independent of
/// `retention_ms`, this bounds memory even when every message is fresh. (#753)
pub const MAX_LOG_MESSAGES: usize = 100_000;

/// Hard cap on the total retained value bytes per partition (256 MiB). (#753)
pub const MAX_LOG_BYTES: usize = 256 * 1024 * 1024;

/// Represents a Kafka partition
#[derive(Debug)]
pub struct Partition {
    pub id: i32,
    pub messages: VecDeque<KafkaMessage>,
    pub high_watermark: i64,
    pub log_start_offset: i64,
    /// Running total of retained `value` bytes, kept in sync with `messages`
    /// so `trim` doesn't have to re-sum the whole log on every append.
    retained_bytes: usize,
    /// Wall-clock ingestion time (epoch ms) for each retained message, in
    /// lock-step with `messages`. Retention is measured against *ingestion*
    /// time, not the record's client-supplied `timestamp` field, so a record
    /// produced with an old/zero logical timestamp isn't evicted on arrival
    /// and an attacker can't force eviction by lying about timestamps (#753).
    ingest_times_ms: VecDeque<i64>,
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
            retained_bytes: 0,
            ingest_times_ms: VecDeque::new(),
        }
    }

    /// Append a message to the partition. The message's `offset` field
    /// is overwritten with the assigned offset before it's stored, so
    /// downstream Fetch path can compare `msg.offset` against
    /// `fetch_offset` correctly.
    pub fn append(&mut self, mut message: KafkaMessage) -> i64 {
        let offset = self.high_watermark;
        message.offset = offset;
        self.retained_bytes = self.retained_bytes.saturating_add(message.value.len());
        self.ingest_times_ms.push_back(now_ms());
        self.messages.push_back(message);
        self.high_watermark += 1;
        offset
    }

    /// Evict messages from the front of the log until it is within the
    /// configured caps. Eviction advances `log_start_offset` so the Fetch
    /// path stays consistent. This is the only place that drops retained
    /// messages, enforcing the size/count/retention limits a mock broker
    /// would otherwise never apply (#753).
    ///
    /// `retention_ms` is compared against `now_ms`; a non-positive
    /// `retention_ms` disables the age check (count/byte caps still apply).
    pub fn trim(&mut self, retention_ms: i64, now_ms: i64, max_messages: usize, max_bytes: usize) {
        loop {
            if self.messages.is_empty() {
                break;
            }

            let over_count = self.messages.len() > max_messages;
            let over_bytes = self.retained_bytes > max_bytes;
            // Age is measured against ingestion time, not the record's
            // client-supplied timestamp (#753).
            let expired = retention_ms > 0
                && self
                    .ingest_times_ms
                    .front()
                    .is_some_and(|&ingested| now_ms.saturating_sub(ingested) > retention_ms);

            // Never evict the only message purely on size; a single oversized
            // record should still be fetchable. Age expiry may drain to empty.
            if !expired && !((over_count || over_bytes) && self.messages.len() > 1) {
                break;
            }

            if let Some(evicted) = self.messages.pop_front() {
                self.retained_bytes = self.retained_bytes.saturating_sub(evicted.value.len());
                self.ingest_times_ms.pop_front();
                self.log_start_offset += 1;
            } else {
                break;
            }
        }
    }

    /// Fetch messages from a given offset
    pub fn fetch(&self, offset: i64, max_bytes: i32) -> Vec<&KafkaMessage> {
        // A fetch targeting an offset below the current log start (because the
        // requested records were evicted by `trim`) must not underflow the
        // `usize` index — clamp to the start of the retained log. (#753)
        let start_idx = offset.saturating_sub(self.log_start_offset).max(0) as usize;
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
    fn test_append_stamps_assigned_offset_onto_message() {
        let mut partition = Partition::new(0);
        // Incoming message has offset=0, but this is the 3rd append so
        // the assigned offset should be 2 — append must overwrite.
        partition.append(create_test_message(0, b"m1"));
        partition.append(create_test_message(0, b"m2"));
        partition.append(create_test_message(0, b"m3"));

        let offsets: Vec<i64> = partition.messages.iter().map(|m| m.offset).collect();
        assert_eq!(offsets, vec![0, 1, 2]);
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
        assert!(!messages.is_empty());
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

    #[test]
    fn test_trim_enforces_message_count_cap_and_advances_start() {
        let mut partition = Partition::new(0);
        // Append well over the cap.
        for i in 0..10 {
            partition.append(create_test_message(0, b"x"));
            // retention disabled (0), count cap = 5, generous byte cap.
            partition.trim(0, 0, 5, 1_000_000);
            let _ = i;
        }

        // Bounded length.
        assert_eq!(partition.messages.len(), 5);
        // log_start_offset advanced past the evicted prefix.
        assert_eq!(partition.log_start_offset, 5);
        assert_eq!(partition.high_watermark, 10);
        // Front message is the oldest retained (offset 5).
        assert_eq!(partition.messages.front().unwrap().offset, 5);
    }

    #[test]
    fn test_trim_enforces_byte_cap() {
        let mut partition = Partition::new(0);
        // Each value is 100 bytes; cap at 250 bytes -> at most 2 retained.
        let payload = vec![b'a'; 100];
        for _ in 0..5 {
            partition.append(create_test_message(0, &payload));
            partition.trim(0, 0, usize::MAX, 250);
        }
        assert!(partition.messages.len() <= 3);
        // Never panics; start offset advanced.
        assert!(partition.log_start_offset > 0);
    }

    #[test]
    fn test_fetch_evicted_offset_does_not_panic() {
        let mut partition = Partition::new(0);
        for _ in 0..10 {
            partition.append(create_test_message(0, b"data"));
        }
        // Evict the first 5.
        partition.trim(0, 0, 5, 1_000_000);
        assert_eq!(partition.log_start_offset, 5);

        // Fetching an already-evicted offset (0) must not underflow/panic;
        // it should serve from the new log start.
        let msgs = partition.fetch(0, 10_000);
        assert_eq!(msgs.len(), 5);
        assert_eq!(msgs[0].offset, 5);
    }

    #[test]
    fn test_trim_age_expiry_can_drain_to_empty() {
        let mut partition = Partition::new(0);
        for _ in 0..3 {
            partition.append(create_test_message(0, b"old"));
        }
        // Retention is measured against ingestion (wall-clock) time. Pass a
        // `now_ms` far in the future so all three are well past retention.
        let far_future = now_ms() + 10_000_000;
        partition.trim(1000, far_future, usize::MAX, usize::MAX);
        assert!(partition.messages.is_empty());
        assert_eq!(partition.log_start_offset, 3);
    }

    #[test]
    fn test_trim_keeps_single_oversized_message() {
        let mut partition = Partition::new(0);
        let big = vec![b'z'; 1024];
        partition.append(create_test_message(0, &big));
        // Byte cap below the single message size; retention disabled.
        partition.trim(0, 0, usize::MAX, 10);
        // The lone message stays fetchable.
        assert_eq!(partition.messages.len(), 1);
        assert_eq!(partition.log_start_offset, 0);
    }
}
