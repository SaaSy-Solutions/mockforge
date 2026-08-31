//! Seed a recorder DB with synthetic HTTP exchanges for verify-mocks smoke tests (#849).
//!
//! Usage: cargo run -p mockforge-cli --example seed_capture_db -- captures.db

use mockforge_recorder::models::{Protocol, RecordedRequest, RecordedResponse};
use mockforge_recorder::{database::RecorderDatabase, Recorder};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let db_path = std::env::args().nth(1).expect("usage: seed_capture_db <db-path>");
    let _ = std::fs::remove_file(&db_path);
    let db = RecorderDatabase::new(&db_path).await?;
    let recorder = Recorder::new(db);

    let exchange =
        |id: &str, path: &str, status: i32, body: String| -> (RecordedRequest, RecordedResponse) {
            (
                RecordedRequest {
                    id: id.to_string(),
                    protocol: Protocol::Http,
                    timestamp: chrono::Utc::now(),
                    method: "GET".into(),
                    path: path.into(),
                    query_params: None,
                    headers: "{}".into(),
                    body: None,
                    body_encoding: "utf8".into(),
                    client_ip: None,
                    trace_id: None,
                    span_id: None,
                    duration_ms: Some(5),
                    status_code: Some(status as i32),
                    tags: None,
                },
                RecordedResponse {
                    request_id: id.to_string(),
                    status_code: status as i32,
                    headers: "{}".into(),
                    body: Some(body),
                    body_encoding: "utf8".into(),
                    size_bytes: 0,
                    timestamp: chrono::Utc::now(),
                },
            )
        };

    for (id, path, status, body) in [
        ("ok-1", "/users/1", 200, r#"{"id":"1","email":"a@b.c"}"#.to_string()),
        ("drift-1", "/users/2", 200, r#"{"id":"2"}"#.to_string()),
        ("unknown-1", "/widgets/9", 200, r#"{"w":true}"#.to_string()),
    ] {
        let (req, resp) = exchange(id, path, status, body);
        recorder.record_request(req).await?;
        recorder.record_response(resp).await?;
    }
    println!("seeded");
    Ok(())
}
