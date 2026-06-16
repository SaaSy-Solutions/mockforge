//! ListOffsets v7 wire-format codec.
//!
//! ListOffsets is how consumers resolve `Offset::Beginning` / `Offset::End`
//! and `auto.offset.reset` into a real starting offset. Without it,
//! `rdkafka::StreamConsumer` and manual-assignment consumers using
//! `Offset::Beginning` get `UnsupportedFeature (Required feature not
//! supported by broker)` and refuse to start.
//!
//! v7 is the first flexible version (compact arrays + tag buffers). We only
//! implement v7; auto-negotiating clients will pick it from the advertised
//! range 0..=7.

use crate::codec_util::sane_capacity;
use crate::produce_codec::{
    push_compact_string, push_empty_tag_buffer, push_unsigned_varint, read_compact_string,
    read_i32, read_i64, read_i8, read_unsigned_varint, skip_tag_buffer,
};

/// One partition's timestamp lookup in a ListOffsets request.
#[derive(Debug, Clone)]
pub struct ListOffsetsPartitionRequest {
    pub partition_index: i32,
    /// Special values:
    ///   -1 = latest (high_watermark)
    ///   -2 = earliest (log_start_offset)
    ///   other = lookup the first message at or after this timestamp
    pub timestamp: i64,
}

#[derive(Debug, Clone)]
pub struct ListOffsetsTopicRequest {
    pub topic: String,
    pub partitions: Vec<ListOffsetsPartitionRequest>,
}

#[derive(Debug, Clone)]
pub struct ListOffsetsRequestV7 {
    pub topics: Vec<ListOffsetsTopicRequest>,
}

#[derive(Debug, Clone, Copy)]
pub struct ListOffsetsPartitionResponse {
    pub partition_index: i32,
    pub error_code: i16,
    /// Timestamp of the returned offset, or -1 if not applicable.
    pub timestamp: i64,
    pub offset: i64,
}

#[derive(Debug, Clone)]
pub struct ListOffsetsTopicResponse {
    pub topic: String,
    pub partitions: Vec<ListOffsetsPartitionResponse>,
}

pub fn parse_listoffsets_v7(body: &[u8]) -> Result<ListOffsetsRequestV7, String> {
    let mut cur = body;

    let _replica_id = read_i32(&mut cur)?;
    let _isolation_level = read_i8(&mut cur)?;

    let topics_len_plus_one = read_unsigned_varint(&mut cur)?;
    if topics_len_plus_one == 0 {
        return Err("listoffsets topics array is null".into());
    }
    let topics_len = (topics_len_plus_one - 1) as usize;
    let mut topics = Vec::with_capacity(sane_capacity(topics_len, cur.len(), 2));

    for _ in 0..topics_len {
        let name = read_compact_string(&mut cur)?;

        let parts_len_plus_one = read_unsigned_varint(&mut cur)?;
        if parts_len_plus_one == 0 {
            return Err(format!("listoffsets partitions array for {name} is null"));
        }
        let parts_len = (parts_len_plus_one - 1) as usize;
        let mut partitions = Vec::with_capacity(sane_capacity(parts_len, cur.len(), 4));
        for _ in 0..parts_len {
            let partition_index = read_i32(&mut cur)?;
            let _current_leader_epoch = read_i32(&mut cur)?;
            let timestamp = read_i64(&mut cur)?;
            skip_tag_buffer(&mut cur)?;
            partitions.push(ListOffsetsPartitionRequest {
                partition_index,
                timestamp,
            });
        }
        skip_tag_buffer(&mut cur)?;
        topics.push(ListOffsetsTopicRequest {
            topic: name,
            partitions,
        });
    }

    skip_tag_buffer(&mut cur)?;

    Ok(ListOffsetsRequestV7 { topics })
}

/// Serialize a full ListOffsets v7 response (flexible response header).
pub fn serialize_listoffsets_v7_response(
    correlation_id: i32,
    topics: &[ListOffsetsTopicResponse],
) -> Vec<u8> {
    let mut out = Vec::new();
    // Response header v1 (flexible)
    out.extend_from_slice(&correlation_id.to_be_bytes());
    push_empty_tag_buffer(&mut out);

    // throttle_time_ms
    out.extend_from_slice(&0i32.to_be_bytes());
    // topics compact array
    push_unsigned_varint(&mut out, (topics.len() as u32) + 1);
    for t in topics {
        push_compact_string(&mut out, &t.topic);
        push_unsigned_varint(&mut out, (t.partitions.len() as u32) + 1);
        for p in &t.partitions {
            out.extend_from_slice(&p.partition_index.to_be_bytes());
            out.extend_from_slice(&p.error_code.to_be_bytes());
            out.extend_from_slice(&p.timestamp.to_be_bytes());
            out.extend_from_slice(&p.offset.to_be_bytes());
            // leader_epoch (v4+) — we don't track leader epochs, return -1.
            out.extend_from_slice(&(-1i32).to_be_bytes());
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

    #[test]
    fn parse_v7_single_topic() {
        let mut body = Vec::new();
        body.extend_from_slice(&(-1i32).to_be_bytes()); // replica_id
        body.push(0); // isolation_level = read_committed off
        push_unsigned_varint(&mut body, 2); // topics len = 1
        push_compact_string(&mut body, "t");
        push_unsigned_varint(&mut body, 2); // partitions len = 1
        body.extend_from_slice(&0i32.to_be_bytes()); // partition_index
        body.extend_from_slice(&(-1i32).to_be_bytes()); // current_leader_epoch
        body.extend_from_slice(&(-2i64).to_be_bytes()); // timestamp = earliest
        push_empty_tag_buffer(&mut body);
        push_empty_tag_buffer(&mut body);
        push_empty_tag_buffer(&mut body);

        let parsed = parse_listoffsets_v7(&body).unwrap();
        assert_eq!(parsed.topics.len(), 1);
        assert_eq!(parsed.topics[0].topic, "t");
        assert_eq!(parsed.topics[0].partitions[0].partition_index, 0);
        assert_eq!(parsed.topics[0].partitions[0].timestamp, -2);
    }

    #[test]
    fn response_v7_layout_is_sane() {
        let resp = serialize_listoffsets_v7_response(
            7,
            &[ListOffsetsTopicResponse {
                topic: "t".into(),
                partitions: vec![ListOffsetsPartitionResponse {
                    partition_index: 0,
                    error_code: 0,
                    timestamp: -1,
                    offset: 42,
                }],
            }],
        );
        // correlation_id
        assert_eq!(&resp[0..4], &7i32.to_be_bytes());
        // header tag buffer
        assert_eq!(resp[4], 0);
        // throttle_time_ms
        assert_eq!(&resp[5..9], &0i32.to_be_bytes());
        // topics array length (1+1)
        assert_eq!(resp[9], 2);
        // topic name "t" (len 1, compact = 2)
        assert_eq!(resp[10], 2);
        assert_eq!(resp[11], b't');
        // partitions array length (1+1)
        assert_eq!(resp[12], 2);
        // partition_index, error_code, timestamp, offset
        assert_eq!(&resp[13..17], &0i32.to_be_bytes());
        assert_eq!(&resp[17..19], &0i16.to_be_bytes());
        assert_eq!(&resp[19..27], &(-1i64).to_be_bytes());
        assert_eq!(&resp[27..35], &42i64.to_be_bytes());
        // leader_epoch = -1
        assert_eq!(&resp[35..39], &(-1i32).to_be_bytes());
        // per-partition tag buffer
        assert_eq!(resp[39], 0);
        // per-topic tag buffer
        assert_eq!(resp[40], 0);
        // top-level tag buffer
        assert_eq!(resp[41], 0);
        assert_eq!(resp.len(), 42);
    }
}
