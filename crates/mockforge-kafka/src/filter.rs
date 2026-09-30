use crate::partitions::KafkaMessage;

/// Decides whether an incoming Kafka record should be retained in memory.
///
/// Returning `false` means the record is acknowledged and receives an offset,
/// but it is not kept in the partition message log.
///
/// This is an "acknowledge but discard" hook: the intended use is a validating
/// sink that inspects everything a producer sends without paying to retain it.
/// A discarded offset leaves a hole in the log — a Fetch that targets it comes
/// back empty with no error, so only install a filter when nothing is expected
/// to consume the dropped records.
pub trait MessageFilter: Send + Sync {
    /// Return `true` to keep the record in the partition log, `false` to
    /// acknowledge it and drop it.
    fn should_keep(&self, topic: &str, partition: i32, message: &KafkaMessage) -> bool;
}
