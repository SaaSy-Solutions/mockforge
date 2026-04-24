//! Fetch v12 wire-format codec.
//!
//! Fetch v12 is flexible: compact arrays, tag buffers. Consumers call Fetch
//! with `(topic, partition, fetch_offset)` triples and expect RecordBatch v2
//! blobs in response. This module:
//!
//!   1. Parses a flexible v12 request into topic/partition fetch slots.
//!   2. Serializes a v12 response that includes fresh, CRC32C-validated
//!      RecordBatch v2 blobs — librdkafka validates CRC and drops batches
//!      that don't match.
//!
//! The broker is responsible for looking records up in partition storage;
//! this module just decodes the request and encodes the response.

use crate::partitions::KafkaMessage;
use crate::produce_codec::{
    push_compact_string, push_empty_tag_buffer, push_signed_varint, push_unsigned_varint,
    read_compact_string, read_i32, read_i64, read_i8, read_unsigned_varint, skip_tag_buffer, take,
};

/// One partition's fetch slot extracted from a Fetch v12 request.
#[derive(Debug, Clone)]
pub struct FetchPartitionRequest {
    pub partition_index: i32,
    pub fetch_offset: i64,
    pub partition_max_bytes: i32,
}

/// One topic's worth of fetch requests.
#[derive(Debug, Clone)]
pub struct FetchTopicRequest {
    pub topic: String,
    pub partitions: Vec<FetchPartitionRequest>,
}

/// Parsed Fetch v12 request body.
#[derive(Debug, Clone)]
pub struct FetchRequestV12 {
    pub max_wait_ms: i32,
    pub min_bytes: i32,
    pub max_bytes: i32,
    pub session_id: i32,
    pub topics: Vec<FetchTopicRequest>,
}

/// Per-partition response slot the broker assembles.
#[derive(Debug, Clone)]
pub struct FetchPartitionResponse {
    pub partition_index: i32,
    pub error_code: i16,
    pub high_watermark: i64,
    pub log_start_offset: i64,
    /// Pre-serialized RecordBatch v2 bytes (empty = no records).
    pub records: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct FetchTopicResponse {
    pub topic: String,
    pub partitions: Vec<FetchPartitionResponse>,
}

// =========================================================================
// Fetch v12 request parser
// =========================================================================

/// Parse a Fetch v12 (flexible) request body. Returns the fields the broker
/// needs to service the fetch.
pub fn parse_fetch_v12(body: &[u8]) -> Result<FetchRequestV12, String> {
    let mut cur = body;

    let _replica_id = read_i32(&mut cur)?;
    let max_wait_ms = read_i32(&mut cur)?;
    let min_bytes = read_i32(&mut cur)?;
    let max_bytes = read_i32(&mut cur)?;
    let _isolation_level = read_i8(&mut cur)?;
    let session_id = read_i32(&mut cur)?;
    let _session_epoch = read_i32(&mut cur)?;

    let topics_len_plus_one = read_unsigned_varint(&mut cur)?;
    if topics_len_plus_one == 0 {
        return Err("fetch topics array is null".into());
    }
    let topics_len = (topics_len_plus_one - 1) as usize;
    let mut topics = Vec::with_capacity(topics_len);

    for _ in 0..topics_len {
        let topic = read_compact_string(&mut cur)?;

        let parts_len_plus_one = read_unsigned_varint(&mut cur)?;
        if parts_len_plus_one == 0 {
            return Err(format!("fetch partitions array for {topic} is null"));
        }
        let parts_len = (parts_len_plus_one - 1) as usize;
        let mut partitions = Vec::with_capacity(parts_len);
        for _ in 0..parts_len {
            let partition_index = read_i32(&mut cur)?;
            let _current_leader_epoch = read_i32(&mut cur)?;
            let fetch_offset = read_i64(&mut cur)?;
            let _last_fetched_epoch = read_i32(&mut cur)?;
            let _log_start_offset = read_i64(&mut cur)?;
            let partition_max_bytes = read_i32(&mut cur)?;
            skip_tag_buffer(&mut cur)?;
            partitions.push(FetchPartitionRequest {
                partition_index,
                fetch_offset,
                partition_max_bytes,
            });
        }
        skip_tag_buffer(&mut cur)?;
        topics.push(FetchTopicRequest { topic, partitions });
    }

    // forgotten_topics_data (skip content)
    let forgotten_len_plus_one = read_unsigned_varint(&mut cur)?;
    if forgotten_len_plus_one > 0 {
        for _ in 0..(forgotten_len_plus_one - 1) {
            let _forgotten_topic = read_compact_string(&mut cur)?;
            let plen_plus_one = read_unsigned_varint(&mut cur)?;
            if plen_plus_one > 0 {
                for _ in 0..(plen_plus_one - 1) {
                    let _ = read_i32(&mut cur)?;
                }
            }
            skip_tag_buffer(&mut cur)?;
        }
    }

    // rack_id (compact string, may be empty)
    let rack_len_plus_one = read_unsigned_varint(&mut cur)?;
    if rack_len_plus_one > 0 {
        let rack_len = (rack_len_plus_one - 1) as usize;
        let _ = take(&mut cur, rack_len)?;
    }

    skip_tag_buffer(&mut cur)?;

    Ok(FetchRequestV12 {
        max_wait_ms,
        min_bytes,
        max_bytes,
        session_id,
        topics,
    })
}

// =========================================================================
// RecordBatch v2 serializer
// =========================================================================

/// Serialize a set of stored `KafkaMessage`s (with absolute offsets already
/// assigned) as one uncompressed RecordBatch v2 blob suitable for inclusion
/// in a Fetch response.
///
/// Uncompressed responses are valid regardless of the client's
/// `compression.type` — consumers select decompression per-batch based on
/// the attributes bits, not their local config. Call
/// `serialize_record_batch_v2_with_compression` if you need to produce a
/// compressed batch for a consumer that requires it.
pub fn serialize_record_batch_v2(records: &[&KafkaMessage]) -> Vec<u8> {
    serialize_record_batch_v2_with_compression(
        records,
        crate::record_compression::CompressionCodec::None,
    )
}

/// Like `serialize_record_batch_v2` but applies `codec` to the records
/// blob and sets the matching attributes bits. CRC32C is computed over the
/// compressed body, as the Kafka spec requires.
pub fn serialize_record_batch_v2_with_compression(
    records: &[&KafkaMessage],
    codec: crate::record_compression::CompressionCodec,
) -> Vec<u8> {
    if records.is_empty() {
        return Vec::new();
    }
    let base_offset = records[0].offset;
    let base_timestamp = records[0].timestamp;
    let max_timestamp = records.iter().map(|r| r.timestamp).max().unwrap_or(base_timestamp);
    let last_offset_delta = (records.last().unwrap().offset - base_offset) as i32;

    // Build the records blob (uncompressed form first — compression is
    // applied at the boundary below).
    let mut records_blob = Vec::new();
    for r in records {
        let mut rec = Vec::new();
        rec.push(0i8 as u8); // attributes
        push_signed_varint(&mut rec, r.timestamp - base_timestamp); // timestamp_delta
        push_signed_varint(&mut rec, r.offset - base_offset); // offset_delta
        match &r.key {
            None => push_signed_varint(&mut rec, -1),
            Some(k) => {
                push_signed_varint(&mut rec, k.len() as i64);
                rec.extend_from_slice(k);
            }
        }
        push_signed_varint(&mut rec, r.value.len() as i64);
        rec.extend_from_slice(&r.value);
        push_signed_varint(&mut rec, r.headers.len() as i64);
        for (hk, hv) in &r.headers {
            push_signed_varint(&mut rec, hk.len() as i64);
            rec.extend_from_slice(hk.as_bytes());
            push_signed_varint(&mut rec, hv.len() as i64);
            rec.extend_from_slice(hv);
        }
        let mut framed = Vec::new();
        push_signed_varint(&mut framed, rec.len() as i64);
        framed.extend_from_slice(&rec);
        records_blob.extend_from_slice(&framed);
    }

    // Apply compression to the records blob. `compress` handles None as a
    // passthrough.
    let compressed_blob = crate::record_compression::compress(codec, &records_blob)
        .expect("compression of in-memory records blob must succeed");

    // Body-after-CRC: everything from `attributes` to the end of `records`.
    // attributes (2) + last_offset_delta (4) + base_timestamp (8) +
    // max_timestamp (8) + producer_id (8) + producer_epoch (2) +
    // base_sequence (4) + records_count (4) + records_blob
    let mut body = Vec::new();
    // attributes: low 3 bits = codec, create-time (bit 3 = 0), no-tx
    body.extend_from_slice(&codec.attributes_bits().to_be_bytes());
    body.extend_from_slice(&last_offset_delta.to_be_bytes());
    body.extend_from_slice(&base_timestamp.to_be_bytes());
    body.extend_from_slice(&max_timestamp.to_be_bytes());
    body.extend_from_slice(&(-1i64).to_be_bytes()); // producer_id
    body.extend_from_slice(&(-1i16).to_be_bytes()); // producer_epoch
    body.extend_from_slice(&(-1i32).to_be_bytes()); // base_sequence
    body.extend_from_slice(&(records.len() as i32).to_be_bytes());
    body.extend_from_slice(&compressed_blob);

    let crc = crc32c::crc32c(&body);

    // Full batch: base_offset + batch_length + partition_leader_epoch +
    // magic + crc + body
    let mut batch = Vec::new();
    batch.extend_from_slice(&base_offset.to_be_bytes());
    // batch_length = size of [partition_leader_epoch..end]
    // partition_leader_epoch(4) + magic(1) + crc(4) + body.len()
    let batch_length = 4 + 1 + 4 + body.len() as i32;
    batch.extend_from_slice(&batch_length.to_be_bytes());
    batch.extend_from_slice(&(-1i32).to_be_bytes()); // partition_leader_epoch
    batch.push(2); // magic = 2
    batch.extend_from_slice(&crc.to_be_bytes());
    batch.extend_from_slice(&body);
    batch
}

// =========================================================================
// Fetch v12 response serializer
// =========================================================================

/// Serialize a full Fetch v12 response including flexible response header.
pub fn serialize_fetch_v12_response(
    correlation_id: i32,
    session_id: i32,
    topics: &[FetchTopicResponse],
) -> Vec<u8> {
    let mut out = Vec::new();
    // Response header v1 (flexible)
    out.extend_from_slice(&correlation_id.to_be_bytes());
    push_empty_tag_buffer(&mut out);

    // Body
    out.extend_from_slice(&0i32.to_be_bytes()); // throttle_time_ms
    out.extend_from_slice(&0i16.to_be_bytes()); // top-level error_code
    out.extend_from_slice(&session_id.to_be_bytes());

    // responses compact array
    push_unsigned_varint(&mut out, (topics.len() as u32) + 1);
    for topic in topics {
        push_compact_string(&mut out, &topic.topic);
        push_unsigned_varint(&mut out, (topic.partitions.len() as u32) + 1);
        for p in &topic.partitions {
            out.extend_from_slice(&p.partition_index.to_be_bytes());
            out.extend_from_slice(&p.error_code.to_be_bytes());
            out.extend_from_slice(&p.high_watermark.to_be_bytes());
            out.extend_from_slice(&p.high_watermark.to_be_bytes()); // last_stable_offset = HWM
            out.extend_from_slice(&p.log_start_offset.to_be_bytes());
            // aborted_transactions: empty compact array
            push_unsigned_varint(&mut out, 1);
            // preferred_read_replica = -1 (no preference)
            out.extend_from_slice(&(-1i32).to_be_bytes());
            // records compact_bytes. librdkafka treats a null (varint 0)
            // here as "invalid MessageSetSize -1" and backs off, so we
            // always emit a non-null compact_bytes — length+1 as varint
            // followed by the batch bytes (empty fetch → just the 0x01
            // varint for a zero-length bytes field).
            push_unsigned_varint(&mut out, (p.records.len() as u32) + 1);
            out.extend_from_slice(&p.records);
            push_empty_tag_buffer(&mut out);
        }
        push_empty_tag_buffer(&mut out);
    }
    push_empty_tag_buffer(&mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::partitions::KafkaMessage;
    use crate::produce_codec::parse_record_batch;

    fn stored_msg(offset: i64, ts: i64, key: Option<&[u8]>, value: &[u8]) -> KafkaMessage {
        KafkaMessage {
            offset,
            timestamp: ts,
            key: key.map(|k| k.to_vec()),
            value: value.to_vec(),
            headers: vec![],
        }
    }

    #[test]
    fn record_batch_roundtrips_through_parser() {
        let msgs = [
            stored_msg(10, 1_000, Some(b"k1"), b"v1"),
            stored_msg(11, 1_500, None, b"v2"),
        ];
        let refs: Vec<&KafkaMessage> = msgs.iter().collect();
        let batch = serialize_record_batch_v2(&refs);
        let (decoded, attrs) = parse_record_batch(&batch).expect("parse");
        assert_eq!(attrs & 0x7, 0);
        assert_eq!(decoded.len(), 2);
        assert_eq!(decoded[0].value, b"v1");
        assert_eq!(decoded[0].key.as_deref(), Some(b"k1".as_ref()));
        assert_eq!(decoded[0].timestamp_ms, 1_000);
        assert_eq!(decoded[1].value, b"v2");
        assert!(decoded[1].key.is_none());
        assert_eq!(decoded[1].timestamp_ms, 1_500);
    }

    #[test]
    fn record_batch_crc_matches_spec() {
        let msgs = [stored_msg(0, 0, None, b"x")];
        let refs: Vec<&KafkaMessage> = msgs.iter().collect();
        let batch = serialize_record_batch_v2(&refs);
        // CRC is big-endian at offset 17 (base_offset[8] + batch_length[4] +
        // partition_leader_epoch[4] + magic[1]). The value must equal
        // crc32c over bytes [21..] (everything after the CRC itself).
        let crc_in_batch = u32::from_be_bytes([batch[17], batch[18], batch[19], batch[20]]);
        let expected = crc32c::crc32c(&batch[21..]);
        assert_eq!(crc_in_batch, expected, "CRC in serialized batch must match crc32c of body");
    }

    #[test]
    fn empty_records_serialize_to_empty_blob() {
        let v: Vec<&KafkaMessage> = Vec::new();
        assert!(serialize_record_batch_v2(&v).is_empty());
    }

    #[test]
    fn compressed_record_batch_roundtrips_through_parser() {
        use crate::record_compression::CompressionCodec;

        let msgs = [
            stored_msg(5, 100, Some(b"k"), b"value-one"),
            stored_msg(6, 200, None, b"value-two-slightly-longer-to-exercise-compression"),
        ];
        let refs: Vec<&KafkaMessage> = msgs.iter().collect();

        for codec in [
            CompressionCodec::Gzip,
            CompressionCodec::Snappy,
            CompressionCodec::Lz4,
            CompressionCodec::Zstd,
        ] {
            let batch = serialize_record_batch_v2_with_compression(&refs, codec);
            let (decoded, attrs) =
                parse_record_batch(&batch).unwrap_or_else(|e| panic!("parse {codec:?}: {e}"));
            assert_eq!(attrs & 0x7, codec.attributes_bits(), "{codec:?}: attributes bits mismatch");
            assert_eq!(decoded.len(), 2, "{codec:?}: wrong record count");
            assert_eq!(decoded[0].value, b"value-one", "{codec:?}: v1 mismatch");
            assert_eq!(decoded[0].key.as_deref(), Some(b"k".as_ref()), "{codec:?}: k1 mismatch");
            assert_eq!(
                decoded[1].value, b"value-two-slightly-longer-to-exercise-compression",
                "{codec:?}: v2 mismatch"
            );
        }
    }

    #[test]
    fn compressed_batch_crc_validates() {
        use crate::record_compression::CompressionCodec;

        let msgs = [stored_msg(0, 0, None, b"hello")];
        let refs: Vec<&KafkaMessage> = msgs.iter().collect();
        let batch = serialize_record_batch_v2_with_compression(&refs, CompressionCodec::Snappy);
        // CRC is at offset 17; spec body starts at 21.
        let crc_in_batch = u32::from_be_bytes([batch[17], batch[18], batch[19], batch[20]]);
        let expected = crc32c::crc32c(&batch[21..]);
        assert_eq!(crc_in_batch, expected);
    }

    #[test]
    fn fetch_v12_request_parses_single_topic() {
        // Build a minimal valid v12 request body and round-trip.
        let mut body = Vec::new();
        body.extend_from_slice(&(-1i32).to_be_bytes()); // replica_id
        body.extend_from_slice(&500i32.to_be_bytes()); // max_wait_ms
        body.extend_from_slice(&1i32.to_be_bytes()); // min_bytes
        body.extend_from_slice(&1_048_576i32.to_be_bytes()); // max_bytes
        body.push(0); // isolation_level
        body.extend_from_slice(&0i32.to_be_bytes()); // session_id
        body.extend_from_slice(&(-1i32).to_be_bytes()); // session_epoch

        push_unsigned_varint(&mut body, 2); // topics len=1 → 1+1
        push_compact_string(&mut body, "orders");
        push_unsigned_varint(&mut body, 2); // partitions len=1 → 1+1
        body.extend_from_slice(&3i32.to_be_bytes()); // partition_index
        body.extend_from_slice(&(-1i32).to_be_bytes()); // current_leader_epoch
        body.extend_from_slice(&42i64.to_be_bytes()); // fetch_offset
        body.extend_from_slice(&(-1i32).to_be_bytes()); // last_fetched_epoch
        body.extend_from_slice(&(-1i64).to_be_bytes()); // log_start_offset
        body.extend_from_slice(&65_536i32.to_be_bytes()); // partition_max_bytes
        push_empty_tag_buffer(&mut body); // partition tags
        push_empty_tag_buffer(&mut body); // topic tags

        push_unsigned_varint(&mut body, 1); // forgotten_topics = empty
        push_unsigned_varint(&mut body, 1); // rack_id = empty string
        push_empty_tag_buffer(&mut body); // top-level tags

        let parsed = parse_fetch_v12(&body).unwrap();
        assert_eq!(parsed.max_wait_ms, 500);
        assert_eq!(parsed.max_bytes, 1_048_576);
        assert_eq!(parsed.topics.len(), 1);
        assert_eq!(parsed.topics[0].topic, "orders");
        assert_eq!(parsed.topics[0].partitions[0].partition_index, 3);
        assert_eq!(parsed.topics[0].partitions[0].fetch_offset, 42);
        assert_eq!(parsed.topics[0].partitions[0].partition_max_bytes, 65_536);
    }
}
