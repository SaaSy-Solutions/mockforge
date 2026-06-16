//! Wire-format codecs for the consumer-group coordination APIs:
//! FindCoordinator v2, JoinGroup v5, SyncGroup v3, Heartbeat v3,
//! OffsetCommit v7, OffsetFetch v5.
//!
//! All six advertise the last *non-flexible* version as their
//! `max_version` in ApiVersions — so librdkafka lands on exactly these
//! codecs regardless of which newer flexible versions its client supports.
//! Keeping the entire consumer-group path on a single wire-format family
//! (non-flexible) simplifies the implementation and the tests.
//!
//! Response header for all of these is v0: just `correlation_id`, no tag
//! buffer.

use crate::codec_util::sane_capacity;
use crate::produce_codec::{read_i32, read_i8, take};

// =========================================================================
// Non-flexible primitives
// =========================================================================

fn read_i16(buf: &mut &[u8]) -> Result<i16, String> {
    let b = take(buf, 2)?;
    Ok(i16::from_be_bytes([b[0], b[1]]))
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

fn read_bytes(buf: &mut &[u8]) -> Result<Vec<u8>, String> {
    let len = read_i32(buf)?;
    if len < 0 {
        return Ok(Vec::new());
    }
    Ok(take(buf, len as usize)?.to_vec())
}

fn push_string(out: &mut Vec<u8>, s: &str) {
    out.extend_from_slice(&(s.len() as i16).to_be_bytes());
    out.extend_from_slice(s.as_bytes());
}

fn push_nullable_string(out: &mut Vec<u8>, s: Option<&str>) {
    match s {
        None => out.extend_from_slice(&(-1i16).to_be_bytes()),
        Some(v) => push_string(out, v),
    }
}

fn push_bytes(out: &mut Vec<u8>, b: &[u8]) {
    out.extend_from_slice(&(b.len() as i32).to_be_bytes());
    out.extend_from_slice(b);
}

// =========================================================================
// FindCoordinator v2
// =========================================================================

#[derive(Debug, Clone)]
pub struct FindCoordinatorRequestV2 {
    pub coordinator_key: String,
    pub key_type: i8,
}

pub fn parse_find_coordinator_v2(body: &[u8]) -> Result<FindCoordinatorRequestV2, String> {
    let mut cur = body;
    let coordinator_key = read_string(&mut cur)?;
    let key_type = read_i8(&mut cur)?;
    Ok(FindCoordinatorRequestV2 {
        coordinator_key,
        key_type,
    })
}

pub fn serialize_find_coordinator_v2_response(
    correlation_id: i32,
    advertised_host: &str,
    advertised_port: i32,
) -> Vec<u8> {
    const BROKER_NODE_ID: i32 = 1;
    let mut out = Vec::new();
    out.extend_from_slice(&correlation_id.to_be_bytes());
    out.extend_from_slice(&0i32.to_be_bytes()); // throttle_time_ms
    out.extend_from_slice(&0i16.to_be_bytes()); // error_code
    out.extend_from_slice(&(-1i16).to_be_bytes()); // error_message (null nullable_string)
    out.extend_from_slice(&BROKER_NODE_ID.to_be_bytes());
    push_string(&mut out, advertised_host);
    out.extend_from_slice(&advertised_port.to_be_bytes());
    out
}

// =========================================================================
// JoinGroup v5
// =========================================================================

#[derive(Debug, Clone)]
pub struct JoinGroupProtocol {
    pub name: String,
    pub metadata: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct JoinGroupRequestV5 {
    pub group_id: String,
    pub member_id: String,
    pub protocol_type: String,
    pub protocols: Vec<JoinGroupProtocol>,
}

pub fn parse_join_group_v5(body: &[u8]) -> Result<JoinGroupRequestV5, String> {
    let mut cur = body;
    let group_id = read_string(&mut cur)?;
    let _session_timeout_ms = read_i32(&mut cur)?;
    let _rebalance_timeout_ms = read_i32(&mut cur)?;
    let member_id = read_string(&mut cur)?;
    let _group_instance_id = read_nullable_string(&mut cur)?;
    let protocol_type = read_string(&mut cur)?;
    let protos_count = read_i32(&mut cur)?;
    if protos_count < 0 {
        return Err("join_group protocols count is negative".into());
    }
    let mut protocols = Vec::with_capacity(sane_capacity(protos_count as usize, cur.len(), 2));
    for _ in 0..protos_count {
        let name = read_string(&mut cur)?;
        let metadata = read_bytes(&mut cur)?;
        protocols.push(JoinGroupProtocol { name, metadata });
    }
    Ok(JoinGroupRequestV5 {
        group_id,
        member_id,
        protocol_type,
        protocols,
    })
}

pub struct JoinGroupResponseMember {
    pub member_id: String,
    pub metadata: Vec<u8>,
}

pub fn serialize_join_group_v5_response(
    correlation_id: i32,
    generation_id: i32,
    protocol_name: &str,
    leader_id: &str,
    own_member_id: &str,
    members: &[JoinGroupResponseMember],
) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&correlation_id.to_be_bytes());
    out.extend_from_slice(&0i32.to_be_bytes()); // throttle_time_ms
    out.extend_from_slice(&0i16.to_be_bytes()); // error_code
    out.extend_from_slice(&generation_id.to_be_bytes());
    push_string(&mut out, protocol_name);
    push_string(&mut out, leader_id);
    push_string(&mut out, own_member_id);
    out.extend_from_slice(&(members.len() as i32).to_be_bytes());
    for m in members {
        push_string(&mut out, &m.member_id);
        push_nullable_string(&mut out, None); // group_instance_id (v5+)
        push_bytes(&mut out, &m.metadata);
    }
    out
}

// =========================================================================
// SyncGroup v3
// =========================================================================

#[derive(Debug, Clone)]
pub struct SyncGroupAssignment {
    pub member_id: String,
    pub assignment: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct SyncGroupRequestV3 {
    pub group_id: String,
    pub generation_id: i32,
    pub member_id: String,
    pub assignments: Vec<SyncGroupAssignment>,
}

pub fn parse_sync_group_v3(body: &[u8]) -> Result<SyncGroupRequestV3, String> {
    let mut cur = body;
    let group_id = read_string(&mut cur)?;
    let generation_id = read_i32(&mut cur)?;
    let member_id = read_string(&mut cur)?;
    let _group_instance_id = read_nullable_string(&mut cur)?;
    let count = read_i32(&mut cur)?;
    if count < 0 {
        return Err("sync_group assignments count is negative".into());
    }
    let mut assignments = Vec::with_capacity(sane_capacity(count as usize, cur.len(), 2));
    for _ in 0..count {
        let m_id = read_string(&mut cur)?;
        let asn = read_bytes(&mut cur)?;
        assignments.push(SyncGroupAssignment {
            member_id: m_id,
            assignment: asn,
        });
    }
    Ok(SyncGroupRequestV3 {
        group_id,
        generation_id,
        member_id,
        assignments,
    })
}

pub fn serialize_sync_group_v3_response(correlation_id: i32, assignment: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&correlation_id.to_be_bytes());
    out.extend_from_slice(&0i32.to_be_bytes()); // throttle_time_ms
    out.extend_from_slice(&0i16.to_be_bytes()); // error_code
    push_bytes(&mut out, assignment);
    out
}

// =========================================================================
// Heartbeat v3
// =========================================================================

#[derive(Debug, Clone)]
pub struct HeartbeatRequestV3 {
    pub group_id: String,
    pub generation_id: i32,
    pub member_id: String,
}

pub fn parse_heartbeat_v3(body: &[u8]) -> Result<HeartbeatRequestV3, String> {
    let mut cur = body;
    let group_id = read_string(&mut cur)?;
    let generation_id = read_i32(&mut cur)?;
    let member_id = read_string(&mut cur)?;
    let _group_instance_id = read_nullable_string(&mut cur)?;
    Ok(HeartbeatRequestV3 {
        group_id,
        generation_id,
        member_id,
    })
}

pub fn serialize_heartbeat_v3_response(correlation_id: i32, error_code: i16) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&correlation_id.to_be_bytes());
    out.extend_from_slice(&0i32.to_be_bytes()); // throttle_time_ms
    out.extend_from_slice(&error_code.to_be_bytes());
    out
}

// =========================================================================
// LeaveGroup v0-v3 (non-flexible). Versions differ in:
//   request : v0-v2 are single-member (group_id + member_id)
//             v3+   are batch (group_id + [members])
//   response: v0-v1: error_code
//             v2+  : throttle_time_ms + error_code
//             v3+  : adds per-member errors array
// librdkafka 2.x picks v1 for our broker despite our ApiVersions saying
// max=3; we accept any of v0..=v3 and respond appropriately.
// =========================================================================

/// Request fields we care about, after merging v0-v3 variants. Every
/// version carries enough info to identify which member(s) to evict.
#[derive(Debug, Clone)]
pub struct LeaveGroupMember {
    pub member_id: String,
}

#[derive(Debug, Clone)]
pub struct LeaveGroupRequest {
    pub group_id: String,
    pub members: Vec<LeaveGroupMember>,
}

/// Parse any non-flexible LeaveGroup request (v0..=v3).
pub fn parse_leave_group(api_version: i16, body: &[u8]) -> Result<LeaveGroupRequest, String> {
    let mut cur = body;
    let group_id = read_string(&mut cur)?;
    let members = if api_version >= 3 {
        let count = read_i32(&mut cur)?;
        if count < 0 {
            return Err("leave_group members count is negative".into());
        }
        let mut out = Vec::with_capacity(sane_capacity(count as usize, cur.len(), 2));
        for _ in 0..count {
            let member_id = read_string(&mut cur)?;
            let _group_instance_id = read_nullable_string(&mut cur)?;
            out.push(LeaveGroupMember { member_id });
        }
        out
    } else {
        // v0-v2: single-member, no group_instance_id.
        let member_id = read_string(&mut cur)?;
        vec![LeaveGroupMember { member_id }]
    };
    Ok(LeaveGroupRequest { group_id, members })
}

/// Serialize a LeaveGroup response matching the request version.
pub fn serialize_leave_group_response(
    api_version: i16,
    correlation_id: i32,
    members: &[LeaveGroupMember],
) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&correlation_id.to_be_bytes());
    if api_version >= 2 {
        out.extend_from_slice(&0i32.to_be_bytes()); // throttle_time_ms
    }
    out.extend_from_slice(&0i16.to_be_bytes()); // top-level error_code
    if api_version >= 3 {
        out.extend_from_slice(&(members.len() as i32).to_be_bytes());
        for m in members {
            push_string(&mut out, &m.member_id);
            push_nullable_string(&mut out, None); // group_instance_id
            out.extend_from_slice(&0i16.to_be_bytes()); // per-member error_code
        }
    }
    out
}

// =========================================================================
// OffsetCommit v7 — parsed fields include the actual committed offset so
// the coordinator can persist it, and the metadata blob the client wants
// to round-trip on the next OffsetFetch.
// =========================================================================

#[derive(Debug, Clone)]
pub struct OffsetCommitPartition {
    pub partition_index: i32,
    pub committed_offset: i64,
    pub committed_metadata: Option<String>,
}

#[derive(Debug, Clone)]
pub struct OffsetCommitTopic {
    pub name: String,
    pub partitions: Vec<OffsetCommitPartition>,
}

#[derive(Debug, Clone)]
pub struct OffsetCommitRequestV7 {
    pub group_id: String,
    pub topics: Vec<OffsetCommitTopic>,
}

pub fn parse_offset_commit_v7(body: &[u8]) -> Result<OffsetCommitRequestV7, String> {
    let mut cur = body;
    let group_id = read_string(&mut cur)?;
    let _generation_id = read_i32(&mut cur)?;
    let _member_id = read_string(&mut cur)?;
    let _group_instance_id = read_nullable_string(&mut cur)?;
    let topics_count = read_i32(&mut cur)?;
    if topics_count < 0 {
        return Err("offset_commit topics count is negative".into());
    }
    let mut topics = Vec::with_capacity(sane_capacity(topics_count as usize, cur.len(), 2));
    for _ in 0..topics_count {
        let name = read_string(&mut cur)?;
        let parts_count = read_i32(&mut cur)?;
        if parts_count < 0 {
            return Err(format!("offset_commit partitions count for {name} is negative"));
        }
        let mut partitions = Vec::with_capacity(sane_capacity(parts_count as usize, cur.len(), 4));
        for _ in 0..parts_count {
            let partition_index = read_i32(&mut cur)?;
            let committed_offset = read_i64(&mut cur)?;
            let _committed_leader_epoch = read_i32(&mut cur)?;
            let committed_metadata = read_nullable_string(&mut cur)?;
            partitions.push(OffsetCommitPartition {
                partition_index,
                committed_offset,
                committed_metadata,
            });
        }
        topics.push(OffsetCommitTopic { name, partitions });
    }
    Ok(OffsetCommitRequestV7 { group_id, topics })
}

pub fn serialize_offset_commit_v7_response(
    correlation_id: i32,
    topics: &[OffsetCommitTopic],
) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&correlation_id.to_be_bytes());
    out.extend_from_slice(&0i32.to_be_bytes()); // throttle_time_ms
    out.extend_from_slice(&(topics.len() as i32).to_be_bytes());
    for t in topics {
        push_string(&mut out, &t.name);
        out.extend_from_slice(&(t.partitions.len() as i32).to_be_bytes());
        for p in &t.partitions {
            out.extend_from_slice(&p.partition_index.to_be_bytes());
            out.extend_from_slice(&0i16.to_be_bytes()); // error_code = 0
        }
    }
    out
}

// =========================================================================
// OffsetFetch v5 (stub — "no committed offset" for every partition)
// =========================================================================

#[derive(Debug, Clone)]
pub struct OffsetFetchTopic {
    pub name: String,
    pub partition_indexes: Vec<i32>,
}

#[derive(Debug, Clone)]
pub struct OffsetFetchRequestV5 {
    pub group_id: String,
    pub topics: Vec<OffsetFetchTopic>,
}

pub fn parse_offset_fetch_v5(body: &[u8]) -> Result<OffsetFetchRequestV5, String> {
    let mut cur = body;
    let group_id = read_string(&mut cur)?;
    let topics_count = read_i32(&mut cur)?;
    // topics is nullable in v5 (-1 = "all topics"); we treat that as empty.
    let mut topics = Vec::new();
    if topics_count > 0 {
        for _ in 0..topics_count {
            let name = read_string(&mut cur)?;
            let parts_count = read_i32(&mut cur)?;
            let mut partition_indexes =
                Vec::with_capacity(sane_capacity(parts_count.max(0) as usize, cur.len(), 4));
            for _ in 0..parts_count.max(0) {
                partition_indexes.push(read_i32(&mut cur)?);
            }
            topics.push(OffsetFetchTopic {
                name,
                partition_indexes,
            });
        }
    }
    Ok(OffsetFetchRequestV5 { group_id, topics })
}

/// One partition's committed-offset lookup result for the OffsetFetch
/// response. `offset == -1` means "no committed offset" (librdkafka
/// then falls back to `auto.offset.reset`).
#[derive(Debug, Clone)]
pub struct OffsetFetchPartitionResponse {
    pub partition_index: i32,
    pub committed_offset: i64,
    pub committed_metadata: Option<String>,
}

#[derive(Debug, Clone)]
pub struct OffsetFetchTopicResponse {
    pub name: String,
    pub partitions: Vec<OffsetFetchPartitionResponse>,
}

pub fn serialize_offset_fetch_v5_response(
    correlation_id: i32,
    topics: &[OffsetFetchTopicResponse],
) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&correlation_id.to_be_bytes());
    out.extend_from_slice(&0i32.to_be_bytes()); // throttle_time_ms
    out.extend_from_slice(&(topics.len() as i32).to_be_bytes());
    for t in topics {
        push_string(&mut out, &t.name);
        out.extend_from_slice(&(t.partitions.len() as i32).to_be_bytes());
        for p in &t.partitions {
            out.extend_from_slice(&p.partition_index.to_be_bytes());
            out.extend_from_slice(&p.committed_offset.to_be_bytes());
            out.extend_from_slice(&(-1i32).to_be_bytes()); // committed_leader_epoch (v5+)
            push_nullable_string(&mut out, p.committed_metadata.as_deref());
            out.extend_from_slice(&0i16.to_be_bytes()); // error_code
        }
    }
    out.extend_from_slice(&0i16.to_be_bytes()); // group-level error_code (v2+)
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_coordinator_v2_round_trip() {
        let key = "my-group";
        let mut body = Vec::new();
        body.extend_from_slice(&(key.len() as i16).to_be_bytes());
        body.extend_from_slice(key.as_bytes());
        body.push(0);
        let parsed = parse_find_coordinator_v2(&body).unwrap();
        assert_eq!(parsed.coordinator_key, "my-group");
    }

    #[test]
    fn join_group_v5_parses_protocols() {
        let mut body = Vec::new();
        push_string(&mut body, "g");
        body.extend_from_slice(&30_000i32.to_be_bytes());
        body.extend_from_slice(&60_000i32.to_be_bytes());
        push_string(&mut body, "");
        push_nullable_string(&mut body, None);
        push_string(&mut body, "consumer");
        body.extend_from_slice(&2i32.to_be_bytes()); // 2 protocols
        push_string(&mut body, "range");
        push_bytes(&mut body, b"R");
        push_string(&mut body, "roundrobin");
        push_bytes(&mut body, b"RR");

        let parsed = parse_join_group_v5(&body).unwrap();
        assert_eq!(parsed.group_id, "g");
        assert_eq!(parsed.protocols.len(), 2);
        assert_eq!(parsed.protocols[0].name, "range");
        assert_eq!(parsed.protocols[0].metadata, b"R");
    }

    #[test]
    fn heartbeat_v3_response_has_error_code() {
        let resp = serialize_heartbeat_v3_response(1, 22);
        assert_eq!(&resp[0..4], &1i32.to_be_bytes());
        assert_eq!(&resp[4..8], &0i32.to_be_bytes()); // throttle
        assert_eq!(&resp[8..10], &22i16.to_be_bytes()); // error_code
        assert_eq!(resp.len(), 10);
    }

    #[test]
    fn offset_fetch_v5_response_says_no_committed() {
        let topics = vec![OffsetFetchTopicResponse {
            name: "t".into(),
            partitions: vec![OffsetFetchPartitionResponse {
                partition_index: 0,
                committed_offset: -1,
                committed_metadata: None,
            }],
        }];
        let resp = serialize_offset_fetch_v5_response(5, &topics);
        // Layout: corr_id (4) + throttle_time_ms (4) + topics_len (4) + …
        assert_eq!(&resp[0..4], &5i32.to_be_bytes()); // correlation_id
        assert_eq!(&resp[4..8], &0i32.to_be_bytes()); // throttle_time_ms
        assert_eq!(&resp[8..12], &1i32.to_be_bytes()); // topics_len
        assert_eq!(&resp[12..14], &1i16.to_be_bytes()); // topic name length
        assert_eq!(resp[14], b't');
        // partitions_len = 1
        assert_eq!(&resp[15..19], &1i32.to_be_bytes());
        // partition_index = 0
        assert_eq!(&resp[19..23], &0i32.to_be_bytes());
        // committed_offset = -1
        assert_eq!(&resp[23..31], &(-1i64).to_be_bytes());
        // committed_leader_epoch = -1 (v5+)
        assert_eq!(&resp[31..35], &(-1i32).to_be_bytes());
    }

    #[test]
    fn offset_fetch_v5_response_carries_real_offset() {
        let topics = vec![OffsetFetchTopicResponse {
            name: "t".into(),
            partitions: vec![OffsetFetchPartitionResponse {
                partition_index: 2,
                committed_offset: 42,
                committed_metadata: Some("m".into()),
            }],
        }];
        let resp = serialize_offset_fetch_v5_response(9, &topics);
        // partition_index at byte 19, committed_offset at 23..31.
        assert_eq!(&resp[0..4], &9i32.to_be_bytes());
        assert_eq!(&resp[19..23], &2i32.to_be_bytes());
        assert_eq!(&resp[23..31], &42i64.to_be_bytes());
    }

    #[test]
    fn offset_commit_v7_parser_keeps_offset_and_metadata() {
        let mut body = Vec::new();
        push_string(&mut body, "g");
        body.extend_from_slice(&7i32.to_be_bytes()); // generation_id
        push_string(&mut body, "m"); // member_id
        push_nullable_string(&mut body, None); // group_instance_id
        body.extend_from_slice(&1i32.to_be_bytes()); // topics_count = 1
        push_string(&mut body, "t");
        body.extend_from_slice(&1i32.to_be_bytes()); // parts_count = 1
        body.extend_from_slice(&3i32.to_be_bytes()); // partition_index
        body.extend_from_slice(&42i64.to_be_bytes()); // committed_offset
        body.extend_from_slice(&(-1i32).to_be_bytes()); // leader_epoch
        push_nullable_string(&mut body, Some("meta"));

        let parsed = parse_offset_commit_v7(&body).unwrap();
        assert_eq!(parsed.group_id, "g");
        assert_eq!(parsed.topics.len(), 1);
        let p = &parsed.topics[0].partitions[0];
        assert_eq!(p.partition_index, 3);
        assert_eq!(p.committed_offset, 42);
        assert_eq!(p.committed_metadata.as_deref(), Some("meta"));
    }
}
