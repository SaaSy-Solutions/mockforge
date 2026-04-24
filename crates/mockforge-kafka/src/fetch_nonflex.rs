//! Non-flexible Fetch codec for v4–v11.
//!
//! Modern clients auto-negotiate to Fetch v12 (flexible), which
//! `fetch_codec` already handles. Older clients stuck on librdkafka 1.8.x —
//! notably `edenhill/kcat:1.7.1` — send Fetch v11, which is non-flexible.
//!
//! v4 is the floor:
//!   - v3 added `max_bytes` at the top of the request.
//!   - v4 added `isolation_level`; every response from v4 onward carries
//!     `last_stable_offset` + `aborted_transactions` per partition, which
//!     clients parse unconditionally at v4+.
//!
//! Supporting v0–v3 would mean a second per-partition response shape
//! (without `last_stable_offset` / `aborted_transactions`) to satisfy a
//! client class that essentially no longer exists in the wild, so we skip
//! it. v4 is the minimum version any client released in the last ~7 years
//! sends.
//!
//! Request body shape differs across the v4–v11 range:
//!   v4:      replica_id, max_wait_ms, min_bytes, max_bytes, isolation_level,
//!            topics[name, partitions[partition_index, fetch_offset,
//!                                    partition_max_bytes]]
//!   v5–v6:   +per-partition log_start_offset (between fetch_offset and
//!            partition_max_bytes)
//!   v7–v8:   +session_id, session_epoch at top;
//!            +forgotten_topics_data array at end (before rack_id in v11)
//!   v9–v10:  +per-partition current_leader_epoch (between partition_index
//!            and fetch_offset)
//!   v11:     +rack_id (non-compact STRING at end)
//!
//! Response body shape:
//!   v4:      throttle_time_ms at top; per-partition has partition_index,
//!            error_code, high_watermark, last_stable_offset,
//!            aborted_transactions array, records.
//!   v5–v10:  per-partition adds log_start_offset (between
//!            last_stable_offset and aborted_transactions).
//!   v7+:     top gains error_code + session_id (after throttle_time_ms).
//!   v11:     per-partition adds preferred_read_replica (between
//!            aborted_transactions and records).
//!
//! Response header is v0 for every non-flexible version: just
//! correlation_id, no tag buffer.

use crate::fetch_codec::{
    FetchPartitionRequest, FetchRequestV12, FetchTopicRequest, FetchTopicResponse,
};

// =========================================================================
// Non-flexible wire primitives
// =========================================================================

fn take<'a>(buf: &mut &'a [u8], n: usize) -> Result<&'a [u8], String> {
    if buf.len() < n {
        return Err(format!("short read: wanted {n}, have {}", buf.len()));
    }
    let (head, tail) = buf.split_at(n);
    *buf = tail;
    Ok(head)
}

fn read_i8(buf: &mut &[u8]) -> Result<i8, String> {
    Ok(take(buf, 1)?[0] as i8)
}

fn read_i16(buf: &mut &[u8]) -> Result<i16, String> {
    let b = take(buf, 2)?;
    Ok(i16::from_be_bytes([b[0], b[1]]))
}

fn read_i32(buf: &mut &[u8]) -> Result<i32, String> {
    let b = take(buf, 4)?;
    Ok(i32::from_be_bytes([b[0], b[1], b[2], b[3]]))
}

fn read_i64(buf: &mut &[u8]) -> Result<i64, String> {
    let b = take(buf, 8)?;
    Ok(i64::from_be_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]))
}

fn read_string(buf: &mut &[u8]) -> Result<String, String> {
    let len = read_i16(buf)?;
    if len < 0 {
        return Err("expected non-null STRING, got null".into());
    }
    let bytes = take(buf, len as usize)?;
    String::from_utf8(bytes.to_vec()).map_err(|e| format!("invalid utf8: {e}"))
}

fn push_string(out: &mut Vec<u8>, s: &str) {
    out.extend_from_slice(&(s.len() as i16).to_be_bytes());
    out.extend_from_slice(s.as_bytes());
}

/// Version at which each feature first appears in a non-flexible Fetch.
fn has_log_start_offset_in_request(v: i16) -> bool {
    v >= 5
}
fn has_session_fields(v: i16) -> bool {
    v >= 7
}
fn has_current_leader_epoch(v: i16) -> bool {
    v >= 9
}
fn has_rack_id(v: i16) -> bool {
    v >= 11
}
fn has_log_start_offset_in_response(v: i16) -> bool {
    v >= 5
}
fn has_response_session_fields(v: i16) -> bool {
    v >= 7
}
fn has_preferred_read_replica(v: i16) -> bool {
    v >= 11
}

// =========================================================================
// Fetch v4–v11 request parser
// =========================================================================

/// Parse a non-flexible Fetch request body for any version in `4..=11`.
/// Returns the same `FetchRequestV12` shape the broker already consumes;
/// fields that don't exist in older versions (e.g. `session_id` pre-v7) are
/// populated with the spec-defined default (0 for session_id).
pub fn parse_fetch_v4_v11(api_version: i16, body: &[u8]) -> Result<FetchRequestV12, String> {
    if !(4..=11).contains(&api_version) {
        return Err(format!("parse_fetch_v4_v11 called with unsupported version {api_version}"));
    }
    let mut cur = body;

    let _replica_id = read_i32(&mut cur)?;
    let max_wait_ms = read_i32(&mut cur)?;
    let min_bytes = read_i32(&mut cur)?;
    // max_bytes added in v3, always present at v4+.
    let max_bytes = read_i32(&mut cur)?;
    // isolation_level added in v4.
    let _isolation_level = read_i8(&mut cur)?;

    // session fields introduced in v7.
    let session_id = if has_session_fields(api_version) {
        let id = read_i32(&mut cur)?;
        let _session_epoch = read_i32(&mut cur)?;
        id
    } else {
        0
    };

    let topics_count = read_i32(&mut cur)?;
    if topics_count < 0 {
        return Err(format!("fetch topics count is negative: {topics_count}"));
    }
    let mut topics = Vec::with_capacity(topics_count as usize);

    for _ in 0..topics_count {
        let topic = read_string(&mut cur)?;
        let parts_count = read_i32(&mut cur)?;
        if parts_count < 0 {
            return Err(format!("fetch partitions count for {topic} is negative"));
        }
        let mut partitions = Vec::with_capacity(parts_count as usize);
        for _ in 0..parts_count {
            let partition_index = read_i32(&mut cur)?;
            if has_current_leader_epoch(api_version) {
                let _current_leader_epoch = read_i32(&mut cur)?;
            }
            let fetch_offset = read_i64(&mut cur)?;
            if has_log_start_offset_in_request(api_version) {
                let _log_start_offset = read_i64(&mut cur)?;
            }
            let partition_max_bytes = read_i32(&mut cur)?;
            partitions.push(FetchPartitionRequest {
                partition_index,
                fetch_offset,
                partition_max_bytes,
            });
        }
        topics.push(FetchTopicRequest { topic, partitions });
    }

    // forgotten_topics_data added in v7. Parse and discard.
    if has_session_fields(api_version) {
        let forgotten_count = read_i32(&mut cur)?;
        if forgotten_count > 0 {
            for _ in 0..forgotten_count {
                let _forgotten_topic = read_string(&mut cur)?;
                let plen = read_i32(&mut cur)?;
                for _ in 0..plen.max(0) {
                    let _ = read_i32(&mut cur)?;
                }
            }
        }
    }

    // rack_id added in v11.
    if has_rack_id(api_version) {
        let _rack_id = read_string(&mut cur)?;
    }

    Ok(FetchRequestV12 {
        max_wait_ms,
        min_bytes,
        max_bytes,
        session_id,
        topics,
    })
}

// =========================================================================
// Fetch v4–v11 response serializer
// =========================================================================

/// Serialize a full non-flexible Fetch response. Writes response header v0
/// (correlation_id only) followed by a body whose shape depends on
/// `api_version` (4..=11).
pub fn serialize_fetch_v4_v11_response(
    correlation_id: i32,
    api_version: i16,
    session_id: i32,
    topics: &[FetchTopicResponse],
) -> Vec<u8> {
    debug_assert!(
        (4..=11).contains(&api_version),
        "serialize_fetch_v4_v11_response called with api_version {api_version}"
    );

    let mut out = Vec::new();
    // Response header v0.
    out.extend_from_slice(&correlation_id.to_be_bytes());

    // throttle_time_ms (v1+, always present at v4+).
    out.extend_from_slice(&0i32.to_be_bytes());

    // error_code + session_id at top-level in v7+.
    if has_response_session_fields(api_version) {
        out.extend_from_slice(&0i16.to_be_bytes()); // top-level error_code
        out.extend_from_slice(&session_id.to_be_bytes());
    }

    // responses (topic array, int32 length).
    out.extend_from_slice(&(topics.len() as i32).to_be_bytes());
    for topic in topics {
        push_string(&mut out, &topic.topic);
        out.extend_from_slice(&(topic.partitions.len() as i32).to_be_bytes());
        for p in &topic.partitions {
            out.extend_from_slice(&p.partition_index.to_be_bytes());
            out.extend_from_slice(&p.error_code.to_be_bytes());
            out.extend_from_slice(&p.high_watermark.to_be_bytes());
            // last_stable_offset: v4+ always present. We don't track
            // transactional state, so advertise high_watermark.
            out.extend_from_slice(&p.high_watermark.to_be_bytes());
            if has_log_start_offset_in_response(api_version) {
                out.extend_from_slice(&p.log_start_offset.to_be_bytes());
            }
            // aborted_transactions: empty array (int32 = 0) at v4+.
            out.extend_from_slice(&0i32.to_be_bytes());
            if has_preferred_read_replica(api_version) {
                // preferred_read_replica = -1 (no preference)
                out.extend_from_slice(&(-1i32).to_be_bytes());
            }
            // records: non-null bytes (int32 length + bytes). Empty fetch
            // sends length = 0, which clients accept without triggering a
            // "MessageSetSize = -1" back-off.
            out.extend_from_slice(&(p.records.len() as i32).to_be_bytes());
            out.extend_from_slice(&p.records);
        }
    }
    out
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fetch_codec::{serialize_record_batch_v2, FetchPartitionResponse};
    use crate::partitions::KafkaMessage;

    /// Build a minimal v11 request body: one topic, one partition,
    /// all optional fields present.
    fn build_v11_request(topic: &str, partition: i32, fetch_offset: i64) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&(-1i32).to_be_bytes()); // replica_id
        body.extend_from_slice(&500i32.to_be_bytes()); // max_wait_ms
        body.extend_from_slice(&1i32.to_be_bytes()); // min_bytes
        body.extend_from_slice(&1_048_576i32.to_be_bytes()); // max_bytes
        body.push(0); // isolation_level
        body.extend_from_slice(&0i32.to_be_bytes()); // session_id
        body.extend_from_slice(&(-1i32).to_be_bytes()); // session_epoch

        body.extend_from_slice(&1i32.to_be_bytes()); // topics count = 1
        push_string(&mut body, topic);
        body.extend_from_slice(&1i32.to_be_bytes()); // partitions count = 1
        body.extend_from_slice(&partition.to_be_bytes()); // partition_index
        body.extend_from_slice(&(-1i32).to_be_bytes()); // current_leader_epoch (v9+)
        body.extend_from_slice(&fetch_offset.to_be_bytes());
        body.extend_from_slice(&(-1i64).to_be_bytes()); // log_start_offset (v5+)
        body.extend_from_slice(&65_536i32.to_be_bytes()); // partition_max_bytes

        body.extend_from_slice(&0i32.to_be_bytes()); // forgotten_topics_data count = 0
        push_string(&mut body, ""); // rack_id empty
        body
    }

    #[test]
    fn parses_v11_request() {
        let body = build_v11_request("orders", 3, 42);
        let parsed = parse_fetch_v4_v11(11, &body).unwrap();
        assert_eq!(parsed.max_wait_ms, 500);
        assert_eq!(parsed.max_bytes, 1_048_576);
        assert_eq!(parsed.session_id, 0);
        assert_eq!(parsed.topics.len(), 1);
        assert_eq!(parsed.topics[0].topic, "orders");
        assert_eq!(parsed.topics[0].partitions[0].partition_index, 3);
        assert_eq!(parsed.topics[0].partitions[0].fetch_offset, 42);
        assert_eq!(parsed.topics[0].partitions[0].partition_max_bytes, 65_536);
    }

    #[test]
    fn parses_v4_request_minimal() {
        // v4: no session fields, no current_leader_epoch, no log_start_offset,
        // no forgotten_topics, no rack_id.
        let mut body = Vec::new();
        body.extend_from_slice(&(-1i32).to_be_bytes()); // replica_id
        body.extend_from_slice(&100i32.to_be_bytes()); // max_wait_ms
        body.extend_from_slice(&1i32.to_be_bytes()); // min_bytes
        body.extend_from_slice(&524_288i32.to_be_bytes()); // max_bytes
        body.push(0); // isolation_level

        body.extend_from_slice(&1i32.to_be_bytes()); // topics count
        push_string(&mut body, "t");
        body.extend_from_slice(&1i32.to_be_bytes()); // partitions count
        body.extend_from_slice(&0i32.to_be_bytes()); // partition_index
        body.extend_from_slice(&7i64.to_be_bytes()); // fetch_offset
        body.extend_from_slice(&32_768i32.to_be_bytes()); // partition_max_bytes

        let parsed = parse_fetch_v4_v11(4, &body).unwrap();
        assert_eq!(parsed.max_wait_ms, 100);
        assert_eq!(parsed.topics[0].partitions[0].fetch_offset, 7);
        assert_eq!(parsed.topics[0].partitions[0].partition_max_bytes, 32_768);
    }

    #[test]
    fn rejects_unsupported_versions() {
        assert!(parse_fetch_v4_v11(3, &[]).is_err());
        assert!(parse_fetch_v4_v11(12, &[]).is_err());
    }

    fn msg(offset: i64, value: &[u8]) -> KafkaMessage {
        KafkaMessage {
            offset,
            timestamp: 1_000,
            key: None,
            value: value.to_vec(),
            headers: vec![],
        }
    }

    fn one_response(records: Vec<u8>) -> Vec<FetchTopicResponse> {
        vec![FetchTopicResponse {
            topic: "t".to_string(),
            partitions: vec![FetchPartitionResponse {
                partition_index: 0,
                error_code: 0,
                high_watermark: 5,
                log_start_offset: 0,
                records,
            }],
        }]
    }

    #[test]
    fn response_v4_shape() {
        // v4 per-partition: partition_index(4) + error_code(2) +
        //   high_watermark(8) + last_stable_offset(8) +
        //   aborted_transactions(4=0 entries) + records(len+bytes)
        let data = serialize_fetch_v4_v11_response(7, 4, 0, &one_response(vec![]));

        // correlation_id
        assert_eq!(&data[0..4], &7i32.to_be_bytes());
        // throttle_time_ms
        assert_eq!(&data[4..8], &0i32.to_be_bytes());
        // topics count (v4 has no session fields)
        assert_eq!(&data[8..12], &1i32.to_be_bytes());
        // topic name "t"
        assert_eq!(&data[12..14], &1i16.to_be_bytes());
        assert_eq!(&data[14..15], b"t");
        // partitions count
        assert_eq!(&data[15..19], &1i32.to_be_bytes());
        // partition layout
        assert_eq!(&data[19..23], &0i32.to_be_bytes()); // partition_index
        assert_eq!(&data[23..25], &0i16.to_be_bytes()); // error_code
        assert_eq!(&data[25..33], &5i64.to_be_bytes()); // high_watermark
        assert_eq!(&data[33..41], &5i64.to_be_bytes()); // last_stable_offset
        assert_eq!(&data[41..45], &0i32.to_be_bytes()); // aborted_transactions count
        assert_eq!(&data[45..49], &0i32.to_be_bytes()); // records length = 0
        assert_eq!(data.len(), 49);
    }

    #[test]
    fn response_v11_adds_session_and_preferred_replica() {
        // v11 extras vs v4:
        //   top: +error_code(2) + session_id(4)
        //   partition: +log_start_offset(8) + preferred_read_replica(4)
        let data = serialize_fetch_v4_v11_response(7, 11, 42, &one_response(vec![]));

        // header(4) + throttle(4) + error(2) + session(4) = 14 before topics
        assert_eq!(&data[8..10], &0i16.to_be_bytes()); // top-level error_code
        assert_eq!(&data[10..14], &42i32.to_be_bytes()); // session_id
        assert_eq!(&data[14..18], &1i32.to_be_bytes()); // topics count
                                                        // topic name "t"
        assert_eq!(&data[18..20], &1i16.to_be_bytes());
        assert_eq!(&data[20..21], b"t");
        // partitions count
        assert_eq!(&data[21..25], &1i32.to_be_bytes());
        // partition layout
        assert_eq!(&data[25..29], &0i32.to_be_bytes()); // partition_index
        assert_eq!(&data[29..31], &0i16.to_be_bytes()); // error_code
        assert_eq!(&data[31..39], &5i64.to_be_bytes()); // high_watermark
        assert_eq!(&data[39..47], &5i64.to_be_bytes()); // last_stable_offset
        assert_eq!(&data[47..55], &0i64.to_be_bytes()); // log_start_offset
        assert_eq!(&data[55..59], &0i32.to_be_bytes()); // aborted_transactions
        assert_eq!(&data[59..63], &(-1i32).to_be_bytes()); // preferred_read_replica
        assert_eq!(&data[63..67], &0i32.to_be_bytes()); // records length = 0
        assert_eq!(data.len(), 67);
    }

    #[test]
    fn response_v11_embeds_records_blob() {
        // The records field carries a complete RecordBatch v2 blob.
        let stored = [msg(10, b"hi")];
        let refs: Vec<&KafkaMessage> = stored.iter().collect();
        let batch = serialize_record_batch_v2(&refs);
        let batch_len = batch.len();
        let topics = one_response(batch.clone());

        let data = serialize_fetch_v4_v11_response(1, 11, 0, &topics);
        // Records sit at the very end; the preceding i32 is their length.
        let len_offset = data.len() - batch_len - 4;
        let got_len = i32::from_be_bytes([
            data[len_offset],
            data[len_offset + 1],
            data[len_offset + 2],
            data[len_offset + 3],
        ]);
        assert_eq!(got_len as usize, batch_len);
        assert_eq!(&data[len_offset + 4..], batch.as_slice());
    }
}
