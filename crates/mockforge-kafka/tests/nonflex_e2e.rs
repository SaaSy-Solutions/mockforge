//! End-to-end regression for the non-flexible Produce v7 + Fetch v11 path
//! that older clients (librdkafka 1.8.x, kcat 1.7.1) depend on.
//!
//! rdkafka 0.38 auto-negotiates to Produce v9 / Fetch v12, so the existing
//! librdkafka-based tests only cover the flexible side. This file drives
//! the broker over raw TCP with bytes that match the older non-flexible
//! wire layouts — if the dispatch or codecs break, produce+fetch against
//! those older clients will silently stop working and the flexible tests
//! won't notice.

use mockforge_core::config::KafkaConfig;
use mockforge_kafka::fetch_codec::serialize_record_batch_v2;
use mockforge_kafka::partitions::KafkaMessage;
use mockforge_kafka::{KafkaMockBroker, Topic, TopicConfig};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

async fn bind_free_port() -> u16 {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}

fn push_string(out: &mut Vec<u8>, s: &str) {
    out.extend_from_slice(&(s.len() as i16).to_be_bytes());
    out.extend_from_slice(s.as_bytes());
}

/// Build a request header v1: api_key, api_version, correlation_id,
/// client_id (non-null STRING). Non-flexible requests stop here — no
/// trailing tag buffer.
fn header_v1(api_key: i16, api_version: i16, correlation_id: i32) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&api_key.to_be_bytes());
    out.extend_from_slice(&api_version.to_be_bytes());
    out.extend_from_slice(&correlation_id.to_be_bytes());
    push_string(&mut out, "nonflex-e2e");
    out
}

/// Frame a Kafka request: 4-byte big-endian length + header + body.
fn frame(header: Vec<u8>, body: Vec<u8>) -> Vec<u8> {
    let mut packet = Vec::with_capacity(4 + header.len() + body.len());
    let total = (header.len() + body.len()) as i32;
    packet.extend_from_slice(&total.to_be_bytes());
    packet.extend_from_slice(&header);
    packet.extend_from_slice(&body);
    packet
}

async fn send_request(stream: &mut TcpStream, packet: &[u8]) -> Vec<u8> {
    stream.write_all(packet).await.expect("write request");
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await.expect("read response length");
    let len = i32::from_be_bytes(len_buf) as usize;
    let mut body = vec![0u8; len];
    stream.read_exact(&mut body).await.expect("read response body");
    body
}

/// Build a Produce v7 request body with a single RecordBatch v2 payload
/// carrying `key`/`value`.
fn produce_v7_body(topic: &str, partition: i32, key: Option<&[u8]>, value: &[u8]) -> Vec<u8> {
    // RecordBatch v2 assembled using the fetch serializer (same logic —
    // keeps the test focused on the wire framing).
    let stored = KafkaMessage {
        offset: 0,
        timestamp: 1_700_000_000_000,
        key: key.map(|k| k.to_vec()),
        value: value.to_vec(),
        headers: vec![],
    };
    let batch = serialize_record_batch_v2(&[&stored]);

    let mut body = Vec::new();
    // transactional_id = null
    body.extend_from_slice(&(-1i16).to_be_bytes());
    // acks = -1 (all)
    body.extend_from_slice(&(-1i16).to_be_bytes());
    // timeout_ms = 30_000
    body.extend_from_slice(&30_000i32.to_be_bytes());
    // topics count = 1
    body.extend_from_slice(&1i32.to_be_bytes());
    push_string(&mut body, topic);
    // partitions count = 1
    body.extend_from_slice(&1i32.to_be_bytes());
    body.extend_from_slice(&partition.to_be_bytes());
    // records NULLABLE_BYTES: length + blob
    body.extend_from_slice(&(batch.len() as i32).to_be_bytes());
    body.extend_from_slice(&batch);
    body
}

fn parse_produce_v7_response(data: &[u8], expected_correlation_id: i32) -> (String, i32, i16, i64) {
    // correlation_id i32
    let corr = i32::from_be_bytes([data[0], data[1], data[2], data[3]]);
    assert_eq!(corr, expected_correlation_id);
    let mut i = 4;
    // topics count i32
    let topics_count = i32::from_be_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]);
    assert_eq!(topics_count, 1, "expected exactly one topic in response");
    i += 4;
    // name STRING
    let name_len = i16::from_be_bytes([data[i], data[i + 1]]) as usize;
    i += 2;
    let name = std::str::from_utf8(&data[i..i + name_len]).unwrap().to_string();
    i += name_len;
    // partitions count i32
    let partitions_count = i32::from_be_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]);
    assert_eq!(partitions_count, 1);
    i += 4;
    let partition_index = i32::from_be_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]);
    i += 4;
    let error_code = i16::from_be_bytes([data[i], data[i + 1]]);
    i += 2;
    let base_offset = i64::from_be_bytes([
        data[i],
        data[i + 1],
        data[i + 2],
        data[i + 3],
        data[i + 4],
        data[i + 5],
        data[i + 6],
        data[i + 7],
    ]);
    (name, partition_index, error_code, base_offset)
}

/// Build a Fetch v11 request body for a single topic/partition.
fn fetch_v11_body(topic: &str, partition: i32, fetch_offset: i64) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&(-1i32).to_be_bytes()); // replica_id
    body.extend_from_slice(&200i32.to_be_bytes()); // max_wait_ms
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

fn parse_fetch_v11_records(data: &[u8], expected_correlation_id: i32) -> Vec<u8> {
    // Response header v0: correlation_id only.
    let corr = i32::from_be_bytes([data[0], data[1], data[2], data[3]]);
    assert_eq!(corr, expected_correlation_id);
    let mut i = 4;
    // throttle_time_ms
    i += 4;
    // top-level error_code (v7+)
    let top_err = i16::from_be_bytes([data[i], data[i + 1]]);
    assert_eq!(top_err, 0, "unexpected top-level fetch error");
    i += 2;
    // session_id (v7+)
    i += 4;
    // topics count
    let topics_count = i32::from_be_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]);
    assert_eq!(topics_count, 1);
    i += 4;
    // topic name
    let name_len = i16::from_be_bytes([data[i], data[i + 1]]) as usize;
    i += 2 + name_len;
    // partitions count
    let partitions_count = i32::from_be_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]);
    assert_eq!(partitions_count, 1);
    i += 4;
    // partition_index, error_code, high_watermark, last_stable_offset,
    // log_start_offset, aborted_transactions, preferred_read_replica (v11)
    i += 4; // partition_index
    let err = i16::from_be_bytes([data[i], data[i + 1]]);
    assert_eq!(err, 0, "partition-level fetch error");
    i += 2;
    i += 8; // high_watermark
    i += 8; // last_stable_offset
    i += 8; // log_start_offset
            // aborted_transactions: i32 count = 0
    let aborted = i32::from_be_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]);
    assert_eq!(aborted, 0);
    i += 4;
    i += 4; // preferred_read_replica (v11+)
            // records bytes: i32 length + body
    let records_len = i32::from_be_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]);
    i += 4;
    assert!(records_len >= 0, "records_len negative: {records_len}");
    data[i..i + records_len as usize].to_vec()
}

/// Dig the record value out of a RecordBatch v2 blob.
///
/// We assume a single-record batch with no compression — which is what the
/// broker produces for our single Produce-sent record. We only need to
/// pull the value bytes out to assert they roundtripped correctly.
fn extract_single_record_value(batch: &[u8]) -> Vec<u8> {
    use mockforge_kafka::produce_codec::parse_record_batch;
    let (records, attrs) = parse_record_batch(batch).expect("parse record batch");
    assert_eq!(attrs & 0x7, 0, "unexpected compression in roundtrip batch");
    assert_eq!(records.len(), 1, "expected single record in roundtrip batch");
    records.into_iter().next().unwrap().value
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn non_flexible_produce_v7_fetch_v11_round_trip() {
    let port = bind_free_port().await;
    let config = KafkaConfig {
        port,
        host: "127.0.0.1".into(),
        ..KafkaConfig::default()
    };

    let broker = Arc::new(KafkaMockBroker::new(config.clone()).await.expect("broker init"));
    {
        let mut topics = broker.topics.write().await;
        topics.insert(
            "nonflex-topic".to_string(),
            Topic::new(
                "nonflex-topic".to_string(),
                TopicConfig {
                    num_partitions: 1,
                    replication_factor: 1,
                    ..Default::default()
                },
            ),
        );
    }

    let server = Arc::clone(&broker);
    let server_handle = tokio::spawn(async move { server.start().await });
    tokio::time::sleep(Duration::from_millis(250)).await;

    let mut stream = TcpStream::connect(("127.0.0.1", port)).await.expect("connect");

    // --- Produce v7 -----------------------------------------------------
    let produce_body = produce_v7_body("nonflex-topic", 0, Some(b"k1"), b"hello-v7");
    let produce_packet = frame(header_v1(0, 7, 101), produce_body);
    let produce_resp = send_request(&mut stream, &produce_packet).await;

    let (topic_name, partition_index, error_code, base_offset) =
        parse_produce_v7_response(&produce_resp, 101);
    assert_eq!(topic_name, "nonflex-topic");
    assert_eq!(partition_index, 0);
    assert_eq!(error_code, 0, "Produce v7 returned error code {error_code}");
    assert_eq!(base_offset, 0);

    // --- Fetch v11 ------------------------------------------------------
    let fetch_body = fetch_v11_body("nonflex-topic", 0, 0);
    let fetch_packet = frame(header_v1(1, 11, 202), fetch_body);
    let fetch_resp = send_request(&mut stream, &fetch_packet).await;

    let records_blob = parse_fetch_v11_records(&fetch_resp, 202);
    assert!(!records_blob.is_empty(), "fetch returned empty records blob");

    let value = extract_single_record_value(&records_blob);
    assert_eq!(value, b"hello-v7", "fetched value didn't match what we produced");

    // Broker's in-memory storage must also hold the record.
    let topics = broker.topics.read().await;
    let topic = topics.get("nonflex-topic").expect("topic present");
    let stored: Vec<_> = topic.partitions.iter().flat_map(|p| p.messages.iter()).collect();
    assert_eq!(stored.len(), 1, "broker stored the wrong number of records");
    assert_eq!(stored[0].value, b"hello-v7");
    assert_eq!(stored[0].key.as_deref(), Some(b"k1".as_ref()));

    server_handle.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn api_versions_advertises_non_flexible_floor() {
    // ApiVersions v3 (what librdkafka/kcat probe with) must now report
    // Produce min=3 and Fetch min=4 so older clients see they can use
    // their default versions.
    let port = bind_free_port().await;
    let config = KafkaConfig {
        port,
        host: "127.0.0.1".into(),
        ..KafkaConfig::default()
    };

    let broker = Arc::new(KafkaMockBroker::new(config.clone()).await.expect("broker init"));
    let server = Arc::clone(&broker);
    let server_handle = tokio::spawn(async move { server.start().await });
    tokio::time::sleep(Duration::from_millis(250)).await;

    let mut stream = TcpStream::connect(("127.0.0.1", port)).await.expect("connect");

    // ApiVersions request header (v3 is flexible: trailing tag buffer).
    let mut header = Vec::new();
    header.extend_from_slice(&18i16.to_be_bytes()); // api_key
    header.extend_from_slice(&3i16.to_be_bytes()); // api_version = 3
    header.extend_from_slice(&7i32.to_be_bytes()); // correlation_id
    push_string(&mut header, "probe");
    header.push(0); // empty tag buffer (flexible header)
                    // ApiVersions v3 request body: client_software_name (compact string,
                    // empty = varint 1), client_software_version (compact string, empty =
                    // varint 1), trailing tag buffer.
    let body = vec![1, 1, 0];

    let packet = frame(header, body);
    let resp = send_request(&mut stream, &packet).await;

    // Response header v0: correlation_id only. Body is flexible (v3+).
    let corr = i32::from_be_bytes([resp[0], resp[1], resp[2], resp[3]]);
    assert_eq!(corr, 7);

    // Scan the flexible body for Produce / Fetch entries.
    // Body layout: error_code(2) + compact_array(api_entries) + ...
    // Each entry: api_key(i16), min(i16), max(i16), tag_buffer(u8).
    let body_bytes = &resp[4..];
    let err = i16::from_be_bytes([body_bytes[0], body_bytes[1]]);
    assert_eq!(err, 0);

    // Compact array length is unsigned-varint(len+1).
    let (mut cur, count) = {
        let mut buf = &body_bytes[2..];
        let raw = read_uvarint(&mut buf);
        (buf, raw - 1)
    };

    let mut saw_produce = false;
    let mut saw_fetch = false;
    for _ in 0..count {
        let api_key = i16::from_be_bytes([cur[0], cur[1]]);
        let min_v = i16::from_be_bytes([cur[2], cur[3]]);
        let max_v = i16::from_be_bytes([cur[4], cur[5]]);
        cur = &cur[7..]; // 2+2+2 + 1 tag buffer
        if api_key == 0 {
            assert_eq!((min_v, max_v), (3, 9), "Produce min/max mismatch — expected 3..=9");
            saw_produce = true;
        }
        if api_key == 1 {
            assert_eq!((min_v, max_v), (4, 12), "Fetch min/max mismatch — expected 4..=12");
            saw_fetch = true;
        }
    }

    assert!(saw_produce, "Produce (api_key=0) not advertised");
    assert!(saw_fetch, "Fetch (api_key=1) not advertised");

    server_handle.abort();
}

fn read_uvarint(buf: &mut &[u8]) -> u32 {
    let mut value: u32 = 0;
    let mut shift: u32 = 0;
    loop {
        let byte = buf[0];
        *buf = &buf[1..];
        value |= ((byte & 0x7F) as u32) << shift;
        if (byte & 0x80) == 0 {
            return value;
        }
        shift += 7;
    }
}
