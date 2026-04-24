//! Non-flexible Produce codec for v3–v8.
//!
//! Modern clients (librdkafka 2.x, confluent-kafka-python 2.x, Java 2.0+)
//! auto-negotiate to Produce v9, which is flexible (compact arrays + tag
//! buffers) and handled by `produce_codec`. Older clients stuck on
//! librdkafka 1.8.x — notably `edenhill/kcat:1.7.1` — top out at Produce v7,
//! which is *non-flexible* (classic int16-prefixed strings, int32 array
//! lengths, no tag buffers).
//!
//! This module adds the non-flexible parser/serializer so those clients can
//! produce against the mock broker. v3 is the floor because it's the first
//! version whose record set is always RecordBatch v2 (`magic == 2`), which
//! is what our decoder understands.
//!
//! Request body shape (v3–v8, all identical):
//!   transactional_id NULLABLE_STRING
//!   acks             INT16
//!   timeout_ms       INT32
//!   topics           ARRAY of {
//!       name         STRING
//!       partitions   ARRAY of {
//!           partition_index INT32
//!           records         NULLABLE_BYTES   // RecordBatch v2 blob
//!       }
//!   }
//!
//! Response body shape (version-dependent):
//!   v3–v4:  throttle_time_ms at top; per-partition has partition_index,
//!           error_code, base_offset, log_append_time_ms.
//!   v5–v7:  per-partition adds log_start_offset.
//!   v8:     per-partition adds record_errors (always empty) + error_message
//!           (always null).
//!
//! Response header for every non-flexible version is v0: just correlation_id
//! (no tag buffer). `produce_codec` handles v9 with the flexible v1 header.

use crate::produce_codec::{
    parse_record_batch, PartitionProduceData, ProduceRequestV9, TopicProduceData,
    TopicProduceResult,
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

fn read_i16(buf: &mut &[u8]) -> Result<i16, String> {
    let b = take(buf, 2)?;
    Ok(i16::from_be_bytes([b[0], b[1]]))
}

fn read_i32(buf: &mut &[u8]) -> Result<i32, String> {
    let b = take(buf, 4)?;
    Ok(i32::from_be_bytes([b[0], b[1], b[2], b[3]]))
}

fn read_nullable_string(buf: &mut &[u8]) -> Result<Option<String>, String> {
    let len = read_i16(buf)?;
    if len < 0 {
        return Ok(None);
    }
    let bytes = take(buf, len as usize)?;
    String::from_utf8(bytes.to_vec())
        .map(Some)
        .map_err(|e| format!("invalid utf8: {e}"))
}

fn read_string(buf: &mut &[u8]) -> Result<String, String> {
    read_nullable_string(buf)?.ok_or_else(|| "expected non-null STRING, got null".into())
}

fn read_nullable_bytes<'a>(buf: &mut &'a [u8]) -> Result<Option<&'a [u8]>, String> {
    let len = read_i32(buf)?;
    if len < 0 {
        return Ok(None);
    }
    Ok(Some(take(buf, len as usize)?))
}

fn push_string(out: &mut Vec<u8>, s: &str) {
    out.extend_from_slice(&(s.len() as i16).to_be_bytes());
    out.extend_from_slice(s.as_bytes());
}

// =========================================================================
// Produce v3–v8 request parser
// =========================================================================

/// Parse a non-flexible Produce request body (v3–v8). The request bodies
/// are byte-identical across this range; the version only matters for the
/// response serializer. Returns the same `ProduceRequestV9` shape the
/// broker already consumes so the handler can stay version-agnostic.
pub fn parse_produce_v3_v8(body: &[u8]) -> Result<ProduceRequestV9, String> {
    let mut cur = body;

    let transactional_id = read_nullable_string(&mut cur)?;
    let acks = read_i16(&mut cur)?;
    let timeout_ms = read_i32(&mut cur)?;

    let topics_count = read_i32(&mut cur)?;
    if topics_count < 0 {
        return Err(format!("produce topics count is negative: {topics_count}"));
    }
    let mut topics = Vec::with_capacity(topics_count as usize);
    for _ in 0..topics_count {
        let name = read_string(&mut cur)?;
        let parts_count = read_i32(&mut cur)?;
        if parts_count < 0 {
            return Err(format!("produce partitions count for {name} is negative"));
        }
        let mut parts = Vec::with_capacity(parts_count as usize);
        for _ in 0..parts_count {
            let partition_index = read_i32(&mut cur)?;
            let records_bytes = read_nullable_bytes(&mut cur)?;
            let (records, attributes) = match records_bytes {
                None => (Vec::new(), 0i16),
                Some(bytes) => parse_record_batch(bytes)?,
            };
            parts.push(PartitionProduceData {
                partition_index,
                records,
                compression_codec: (attributes & 0x7) as i8,
            });
        }
        topics.push(TopicProduceData {
            name,
            partitions: parts,
        });
    }

    Ok(ProduceRequestV9 {
        transactional_id,
        acks,
        timeout_ms,
        topics,
    })
}

// =========================================================================
// Produce v3–v8 response serializer
// =========================================================================

/// Serialize a full Produce response with non-flexible framing for the
/// given `api_version`. Writes the response header v0 (correlation_id
/// only) followed by a body whose shape is version-branched.
///
/// Supported versions: 3, 4, 5, 6, 7, 8.
pub fn serialize_produce_v3_v8_response(
    correlation_id: i32,
    api_version: i16,
    results: &[TopicProduceResult],
) -> Vec<u8> {
    debug_assert!(
        (3..=8).contains(&api_version),
        "serialize_produce_v3_v8_response called with api_version {api_version}"
    );

    let has_log_start_offset = api_version >= 5;
    let has_record_errors = api_version >= 8;

    let mut out = Vec::new();
    // Response header v0: correlation_id only, no tag buffer.
    out.extend_from_slice(&correlation_id.to_be_bytes());

    // topics array (int32 length)
    out.extend_from_slice(&(results.len() as i32).to_be_bytes());
    for topic in results {
        push_string(&mut out, &topic.name);
        out.extend_from_slice(&(topic.partitions.len() as i32).to_be_bytes());
        for p in &topic.partitions {
            out.extend_from_slice(&p.partition_index.to_be_bytes());
            out.extend_from_slice(&p.error_code.to_be_bytes());
            out.extend_from_slice(&p.base_offset.to_be_bytes());
            // v2+ has log_append_time_ms; we only target v3+ so it's always present.
            out.extend_from_slice(&p.log_append_time_ms.to_be_bytes());
            if has_log_start_offset {
                out.extend_from_slice(&p.log_start_offset.to_be_bytes());
            }
            if has_record_errors {
                // record_errors: empty array (int32 = 0)
                out.extend_from_slice(&0i32.to_be_bytes());
                // error_message: null nullable-string (int16 = -1)
                out.extend_from_slice(&(-1i16).to_be_bytes());
            }
        }
    }

    // throttle_time_ms at the end (v1+).
    out.extend_from_slice(&0i32.to_be_bytes());
    out
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::produce_codec::{one_record_batch_for_testing, PartitionProduceResult};

    fn build_request_body(topic: &str, partition: i32, batch: &[u8]) -> Vec<u8> {
        let mut body = Vec::new();
        // transactional_id = null
        body.extend_from_slice(&(-1i16).to_be_bytes());
        // acks = -1
        body.extend_from_slice(&(-1i16).to_be_bytes());
        // timeout_ms = 30_000
        body.extend_from_slice(&30_000i32.to_be_bytes());
        // topics count = 1
        body.extend_from_slice(&1i32.to_be_bytes());
        push_string(&mut body, topic);
        // partitions count = 1
        body.extend_from_slice(&1i32.to_be_bytes());
        body.extend_from_slice(&partition.to_be_bytes());
        // records NULLABLE_BYTES: length-prefixed with int32
        body.extend_from_slice(&(batch.len() as i32).to_be_bytes());
        body.extend_from_slice(batch);
        body
    }

    #[test]
    fn parses_minimal_produce_v7_request() {
        let batch = one_record_batch_for_testing(Some(b"k"), b"hello");
        let body = build_request_body("orders", 2, &batch);

        let parsed = parse_produce_v3_v8(&body).unwrap();
        assert_eq!(parsed.transactional_id, None);
        assert_eq!(parsed.acks, -1);
        assert_eq!(parsed.timeout_ms, 30_000);
        assert_eq!(parsed.topics.len(), 1);
        assert_eq!(parsed.topics[0].name, "orders");
        assert_eq!(parsed.topics[0].partitions.len(), 1);
        assert_eq!(parsed.topics[0].partitions[0].partition_index, 2);
        assert_eq!(parsed.topics[0].partitions[0].records.len(), 1);
        assert_eq!(parsed.topics[0].partitions[0].records[0].value, b"hello");
        assert_eq!(parsed.topics[0].partitions[0].compression_codec, 0);
    }

    #[test]
    fn parses_produce_v3_v8_with_null_records() {
        let mut body = Vec::new();
        body.extend_from_slice(&(-1i16).to_be_bytes()); // transactional_id null
        body.extend_from_slice(&1i16.to_be_bytes()); // acks = 1
        body.extend_from_slice(&5_000i32.to_be_bytes()); // timeout_ms
        body.extend_from_slice(&1i32.to_be_bytes()); // topics = 1
        push_string(&mut body, "t");
        body.extend_from_slice(&1i32.to_be_bytes()); // partitions = 1
        body.extend_from_slice(&0i32.to_be_bytes()); // partition_index
        body.extend_from_slice(&(-1i32).to_be_bytes()); // records length -1 = null

        let parsed = parse_produce_v3_v8(&body).unwrap();
        assert_eq!(parsed.acks, 1);
        assert!(parsed.topics[0].partitions[0].records.is_empty());
    }

    #[test]
    fn detects_compressed_batch_in_v7_request() {
        let mut batch = one_record_batch_for_testing(None, b"x");
        // Attributes i16 is at offset 21: flip compression to gzip (bit 0).
        batch[21] = 0;
        batch[22] = 1;
        let body = build_request_body("t", 0, &batch);

        let parsed = parse_produce_v3_v8(&body).unwrap();
        assert_eq!(parsed.topics[0].partitions[0].compression_codec, 1);
        assert!(parsed.topics[0].partitions[0].records.is_empty());
    }

    fn one_result(log_start_offset: i64) -> Vec<TopicProduceResult> {
        vec![TopicProduceResult {
            name: "orders".to_string(),
            partitions: vec![PartitionProduceResult {
                partition_index: 2,
                error_code: 0,
                base_offset: 41,
                log_append_time_ms: -1,
                log_start_offset,
            }],
        }]
    }

    #[test]
    fn response_v3_shape() {
        let data = serialize_produce_v3_v8_response(99, 3, &one_result(0));

        // correlation_id
        assert_eq!(&data[0..4], &99i32.to_be_bytes());
        // topics count = 1
        assert_eq!(&data[4..8], &1i32.to_be_bytes());
        // topic name "orders" length=6 then bytes
        assert_eq!(&data[8..10], &6i16.to_be_bytes());
        assert_eq!(&data[10..16], b"orders");
        // partitions count = 1
        assert_eq!(&data[16..20], &1i32.to_be_bytes());
        // partition_index, error_code, base_offset, log_append_time_ms
        assert_eq!(&data[20..24], &2i32.to_be_bytes());
        assert_eq!(&data[24..26], &0i16.to_be_bytes());
        assert_eq!(&data[26..34], &41i64.to_be_bytes());
        assert_eq!(&data[34..42], &(-1i64).to_be_bytes());
        // throttle_time_ms at end
        assert_eq!(&data[42..46], &0i32.to_be_bytes());
        assert_eq!(data.len(), 46);
    }

    #[test]
    fn response_v7_adds_log_start_offset() {
        let data = serialize_produce_v3_v8_response(7, 7, &one_result(0));
        // Expected layout length for v7: header(4) + topics_len(4) +
        // string_len(2) + "orders"(6) + parts_len(4) + partition_index(4) +
        // error_code(2) + base_offset(8) + log_append_time_ms(8) +
        // log_start_offset(8) + throttle_time_ms(4) = 54
        assert_eq!(data.len(), 54);
        // Double-check log_start_offset lands at the right offset.
        assert_eq!(&data[42..50], &0i64.to_be_bytes());
    }

    #[test]
    fn response_v8_adds_record_errors_and_error_message() {
        let data = serialize_produce_v3_v8_response(123, 8, &one_result(0));
        // v8 layout: v7 (54 bytes minus trailing throttle) + record_errors(4)
        // + error_message(2) + throttle(4)
        //   header(4) + topics(4) + name(8) + parts_count(4) + partition(4)
        //   + error_code(2) + base_offset(8) + log_append_time(8) +
        //   log_start_offset(8) + record_errors(4) + error_message(2) +
        //   throttle(4) = 60
        assert_eq!(data.len(), 60);
        // record_errors array length = 0
        assert_eq!(&data[50..54], &0i32.to_be_bytes());
        // error_message = null (-1)
        assert_eq!(&data[54..56], &(-1i16).to_be_bytes());
    }

    #[test]
    fn request_parser_roundtrips_through_serializer() {
        let batch = one_record_batch_for_testing(None, b"v");
        let body = build_request_body("t", 0, &batch);

        let parsed = parse_produce_v3_v8(&body).unwrap();

        // Build a plausible response using a v7 shape.
        let results = vec![TopicProduceResult {
            name: parsed.topics[0].name.clone(),
            partitions: vec![PartitionProduceResult {
                partition_index: parsed.topics[0].partitions[0].partition_index,
                error_code: 0,
                base_offset: 0,
                log_append_time_ms: 1_700_000_000_000,
                log_start_offset: 0,
            }],
        }];
        let resp = serialize_produce_v3_v8_response(42, 7, &results);
        assert_eq!(&resp[0..4], &42i32.to_be_bytes());
    }
}
