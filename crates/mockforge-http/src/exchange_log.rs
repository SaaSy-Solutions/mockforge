//! Writing down every exchange with the mock, so that what an agent reports
//! can be judged against what the mock saw.
//!
//! `serve --exchange-log FILE` starts a log. Every request the mock receives
//! becomes one line of JSON, in the shape
//! `mockforge_contracts::range::Exchange` holds: the method, the path and the
//! query, the request's headers and the start of its body, the status the
//! mock answered, its headers and the start of its body, and how long it
//! took. A header that carries a credential is a digest of its value, never
//! the value. The log is the outermost layer of the server, so it sees a
//! request whoever answered it, and a request to a path the spec does not
//! declare.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use axum::body::{to_bytes, Body};
use axum::extract::Request;
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use mockforge_contracts::range::exchange::{header_value, held, Exchange};
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

/// The most the log reads of a body. A request body beyond it is refused, and
/// an answer beyond it is replaced by an error: the log has to hold a body
/// whole before it can let it through, and no mock answer comes near this.
const READ_LIMIT: usize = 64 * 1024 * 1024;

/// A file the mock appends one line to per exchange.
#[derive(Clone, Debug)]
pub struct ExchangeLog {
    file: Arc<Mutex<tokio::fs::File>>,
    /// The last sequence number handed out.
    sequence: Arc<AtomicU64>,
}

impl ExchangeLog {
    /// Start the log at `path`. A file that is there is replaced.
    pub async fn create(path: &Path) -> std::io::Result<Self> {
        let file = tokio::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)
            .await?;
        Ok(Self {
            file: Arc::new(Mutex::new(file)),
            sequence: Arc::new(AtomicU64::new(0)),
        })
    }

    /// Write an exchange down as the next line. Its number and its time are
    /// the log's to give. When this returns the line is in the file, so
    /// whoever reads the log after an answer finds that answer in it.
    pub async fn record(&self, mut exchange: Exchange) -> std::io::Result<()> {
        exchange.seq = self.sequence.fetch_add(1, Ordering::Relaxed) + 1;
        exchange.at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Micros, true);
        let mut line = serde_json::to_string(&exchange).map_err(std::io::Error::other)?;
        line.push('\n');
        let mut file = self.file.lock().await;
        // A write on tokio's file is handed to a thread and reported done
        // before that thread has run. The flush waits for it: when this
        // returns, the line is where whoever reads the log finds it.
        file.write_all(line.as_bytes()).await?;
        file.flush().await
    }
}

/// One request, written down with the mock's answer to it. The answer is
/// returned as the mock gave it.
pub async fn record(log: Arc<ExchangeLog>, request: Request, next: Next) -> Response {
    let started = Instant::now();
    let (parts, body) = request.into_parts();
    let method = parts.method.as_str().to_ascii_uppercase();
    let path = parts.uri.path().to_string();
    let query = parts.uri.query().unwrap_or_default().to_string();
    let request_headers = logged(&parts.headers);
    let bytes = match to_bytes(body, READ_LIMIT).await {
        Ok(bytes) => bytes,
        Err(error) => return refused("the request body", &error.to_string()),
    };
    let (request_body, request_body_truncated) = held(&bytes);
    let response = next.run(Request::from_parts(parts, Body::from(bytes))).await;

    let (parts, body) = response.into_parts();
    let status = parts.status.as_u16();
    let response_headers = logged(&parts.headers);
    let bytes = match to_bytes(body, READ_LIMIT).await {
        Ok(bytes) => bytes,
        Err(error) => return refused("the answer's body", &error.to_string()),
    };
    let (response_body, response_body_truncated) = held(&bytes);
    let exchange = Exchange {
        seq: 0,
        at: String::new(),
        method,
        path,
        query,
        request_headers,
        request_body,
        request_body_truncated,
        status,
        response_headers,
        response_body,
        response_body_truncated,
        duration_ms: started.elapsed().as_millis() as u64,
    };
    // The mock has answered. A log that cannot be written must not undo that.
    if let Err(error) = log.record(exchange).await {
        tracing::warn!("the exchange log could not be written: {error}");
    }
    Response::from_parts(parts, Body::from(bytes))
}

/// The answer when a body was more than the log can hold.
fn refused(what: &str, why: &str) -> Response {
    tracing::warn!("the exchange log cannot hold {what}: {why}");
    (
        StatusCode::PAYLOAD_TOO_LARGE,
        format!("the exchange log cannot hold {what}: it is larger than {READ_LIMIT} bytes"),
    )
        .into_response()
}

/// The headers as the log holds them: by name in lower case, a header with
/// several values joined as HTTP joins them, and a credential as a digest.
fn logged(headers: &HeaderMap) -> BTreeMap<String, String> {
    let mut logged = BTreeMap::new();
    for (name, value) in headers {
        let Ok(value) = value.to_str() else { continue };
        let name = name.as_str().to_ascii_lowercase();
        let joined = match logged.get(&name) {
            Some(seen) => format!("{seen}, {value}"),
            None => value.to_string(),
        };
        let held = header_value(&name, &joined);
        logged.insert(name, held);
    }
    logged
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Bytes;
    use axum::routing::{get, post};
    use axum::{Json, Router};
    use mockforge_contracts::range::exchange::{digest, read_exchanges, BODY_LIMIT};
    use std::path::PathBuf;
    use tower::ServiceExt;

    /// The path of a log in a directory of its own, and that directory.
    fn place() -> (PathBuf, PathBuf) {
        let directory =
            std::env::temp_dir().join(format!("mockforge-exchange-log-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).unwrap();
        (directory.join("exchanges.jsonl"), directory)
    }

    /// `router` with the layer that writes the log, as `serve` adds it.
    fn watched(router: Router, log: Arc<ExchangeLog>) -> Router {
        router.layer(axum::middleware::from_fn(move |request, next| {
            let log = log.clone();
            async move { record(log, request, next).await }
        }))
    }

    /// A small API: an order lookup that answers with JSON, and an echo.
    fn api(log: Arc<ExchangeLog>) -> Router {
        watched(
            Router::new()
                .route(
                    "/orders/{id}",
                    get(|| async { Json(serde_json::json!([{ "id": 5000 }, { "id": 5001 }])) }),
                )
                .route(
                    "/echo",
                    post(
                        |body: String| async move { (StatusCode::CREATED, format!("got {body}")) },
                    ),
                ),
            log,
        )
    }

    #[tokio::test]
    async fn every_exchange_is_one_line_with_the_request_and_the_answer() {
        let (path, directory) = place();
        let log = Arc::new(ExchangeLog::create(&path).await.unwrap());
        let api = api(log.clone());

        let authorized = Request::builder()
            .uri("/orders/5000?limit=5")
            .header("Authorization", "Bearer demo-user-a")
            .header("Accept", "application/json")
            .body(Body::empty())
            .unwrap();
        let answer = api.clone().oneshot(authorized).await.unwrap();
        assert_eq!(answer.status(), StatusCode::OK);

        let posted = Request::builder()
            .method("POST")
            .uri("/echo")
            .header("Content-Type", "text/plain")
            .body(Body::from("hello"))
            .unwrap();
        let answer = api.clone().oneshot(posted).await.unwrap();
        assert_eq!(answer.status(), StatusCode::CREATED);
        let text = to_bytes(answer.into_body(), usize::MAX).await.unwrap();
        assert_eq!(text, "got hello");

        // A path the API does not declare is answered by the router's own 404,
        // and written down like any other exchange.
        let answer = api
            .clone()
            .oneshot(Request::builder().uri("/nope").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(answer.status(), StatusCode::NOT_FOUND);
        drop(api);
        drop(log);

        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::remove_dir_all(&directory).unwrap();
        assert!(text.ends_with('\n'));
        let exchanges = read_exchanges(&text, "exchanges.jsonl").unwrap();
        let sequences: Vec<u64> = exchanges.iter().map(|exchange| exchange.seq).collect();
        assert_eq!(sequences, vec![1, 2, 3]);
        let first = &exchanges[0];
        assert_eq!(
            (first.method.as_str(), first.path.as_str(), first.query.as_str()),
            ("GET", "/orders/5000", "limit=5")
        );
        assert_eq!(first.status, 200);
        assert!(first.at.contains('T') && first.at.ends_with('Z'), "{}", first.at);
        // The credential is a digest. Another header is itself.
        assert_eq!(first.request_headers["authorization"], digest("Bearer demo-user-a"));
        assert_eq!(first.request_headers["accept"], "application/json");
        assert!(!text.contains("demo-user-a"));
        assert_eq!(first.response_body, r#"[{"id":5000},{"id":5001}]"#);
        let second = &exchanges[1];
        assert_eq!(
            (second.method.as_str(), second.request_body.as_str(), second.status),
            ("POST", "hello", 201)
        );
        assert_eq!(second.response_body, "got hello");
        assert_eq!((exchanges[2].path.as_str(), exchanges[2].status), ("/nope", 404));
    }

    #[tokio::test]
    async fn a_body_is_held_up_to_the_limit() {
        let (path, directory) = place();
        let log = Arc::new(ExchangeLog::create(&path).await.unwrap());
        let api = watched(
            Router::new().route("/big", post(|body: Bytes| async move { body })),
            log.clone(),
        );
        let big = vec![b'x'; BODY_LIMIT + 10];
        let answer = api
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/big")
                    .body(Body::from(big.clone()))
                    .unwrap(),
            )
            .await
            .unwrap();
        // The mock answers with the whole body it was sent.
        let text = to_bytes(answer.into_body(), usize::MAX).await.unwrap();
        assert_eq!(text.len(), big.len());
        drop(api);
        drop(log);
        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::remove_dir_all(&directory).unwrap();
        let exchanges = read_exchanges(&text, "e").unwrap();
        let exchange = &exchanges[0];
        assert_eq!(
            (exchange.request_body.len(), exchange.response_body.len()),
            (BODY_LIMIT, BODY_LIMIT)
        );
        assert!(exchange.request_body_truncated && exchange.response_body_truncated);
    }

    #[tokio::test]
    async fn a_log_replaces_what_was_there_and_can_be_shared() {
        let (path, directory) = place();
        std::fs::write(&path, "left over\n").unwrap();
        let log = Arc::new(ExchangeLog::create(&path).await.unwrap());
        for status in [200u16, 404] {
            log.record(Exchange {
                seq: 0,
                at: String::new(),
                method: "GET".to_string(),
                path: "/x".to_string(),
                query: String::new(),
                request_headers: BTreeMap::new(),
                request_body: String::new(),
                request_body_truncated: false,
                status,
                response_headers: BTreeMap::new(),
                response_body: String::new(),
                response_body_truncated: false,
                duration_ms: 0,
            })
            .await
            .unwrap();
        }
        drop(log);
        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::remove_dir_all(&directory).unwrap();
        assert!(!text.contains("left over"));
        let exchanges = read_exchanges(&text, "e").unwrap();
        assert_eq!(
            exchanges
                .iter()
                .map(|exchange| (exchange.seq, exchange.status))
                .collect::<Vec<_>>(),
            vec![(1, 200), (2, 404)]
        );
    }

    #[tokio::test]
    async fn a_log_that_cannot_be_started_is_an_error() {
        let missing = std::env::temp_dir().join("mockforge-no-such-dir").join("exchanges.jsonl");
        assert!(ExchangeLog::create(&missing).await.is_err());
    }

    #[test]
    fn headers_are_joined_as_http_joins_them() {
        let mut headers = HeaderMap::new();
        headers.append("X-Trace", "a".parse().unwrap());
        headers.append("x-trace", "b".parse().unwrap());
        headers.insert("Set-Cookie", "session=1".parse().unwrap());
        let logged = logged(&headers);
        assert_eq!(logged["x-trace"], "a, b");
        assert_eq!(logged["set-cookie"], digest("session=1"));
        assert_eq!(logged.len(), 2);
    }
}
