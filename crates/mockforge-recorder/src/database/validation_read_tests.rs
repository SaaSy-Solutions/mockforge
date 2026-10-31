use super::*;
use chrono::Utc;

fn request(id: &str, protocol: Protocol) -> RecordedRequest {
    RecordedRequest {
        id: id.into(),
        protocol,
        timestamp: Utc::now(),
        method: "GET".into(),
        path: "/items".into(),
        query_params: None,
        headers: "{}".into(),
        body: None,
        body_encoding: "utf8".into(),
        client_ip: None,
        trace_id: None,
        span_id: None,
        duration_ms: Some(1),
        status_code: Some(200),
        tags: None,
    }
}

#[tokio::test]
async fn readonly_open_never_creates_or_writes_capture_files() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("capture.sqlite");
    assert!(RecorderDatabase::open_read_only(&path).await.is_err());
    assert!(!path.exists());
    let writer = RecorderDatabase::new(&path).await.unwrap();
    writer.insert_request(&request("original", Protocol::Http)).await.unwrap();
    let reader = RecorderDatabase::open_read_only(&path).await.unwrap();
    assert!(reader.get_request("original").await.unwrap().is_some());
    assert!(reader.insert_request(&request("must-not-write", Protocol::Http)).await.is_err());
    assert!(writer.get_request("must-not-write").await.unwrap().is_none());
}

#[tokio::test]
async fn protocol_filter_applies_before_recent_limit() {
    let db = RecorderDatabase::new_in_memory().await.unwrap();
    db.insert_request(&request("older-http", Protocol::Http)).await.unwrap();
    for i in 0..1001 {
        db.insert_request(&request(&format!("mqtt-{i}"), Protocol::Mqtt)).await.unwrap();
    }
    let http = db.list_recent_for_protocol(Protocol::Http, 1).await.unwrap();
    assert_eq!(http.len(), 1);
    assert_eq!(http[0].id, "older-http");
    assert_eq!(db.list_recent_for_protocol(Protocol::Mqtt, 120).await.unwrap().len(), 120);
    assert!(db.list_recent_for_protocol(Protocol::Http, 0).await.unwrap().is_empty());
}
