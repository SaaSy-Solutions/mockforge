//! Real S3 SDK requests against an owned loopback object-storage fixture.
use super::{Credentials, PluginStorage, Region, S3Client, StorageBackend};
use axum::{body::Body, http::StatusCode, response::Response, Router};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use uuid::Uuid;

const LIMIT: usize = 2 * 1024 * 1024;

struct Fixture {
    endpoint: String,
    paths: Arc<Mutex<Vec<String>>>,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl Fixture {
    async fn start(org: Uuid) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let paths = Arc::new(Mutex::new(Vec::new()));
        let captured = paths.clone();
        let prefix = format!("/fixture-bucket/specs/{org}/");
        let router = Router::new().fallback(move |uri: axum::http::Uri| {
            let captured = captured.clone();
            let prefix = prefix.clone();
            async move {
                // Capture only the object path; never collect authorization headers.
                captured.lock().unwrap().push(uri.path().to_string());
                let name = uri.path().strip_prefix(&prefix).unwrap_or("");
                let body = match name {
                    "valid.json" => Body::from(br#"{"openapi":"3.0.0"}"#.as_slice()),
                    "exact.json" => Body::from(vec![b'x'; LIMIT]),
                    "overflow.json" => {
                        // No advertised length: enforce the cap while streaming, too.
                        let chunks = futures_util::stream::iter([
                            Ok::<_, std::io::Error>(vec![b'x'; LIMIT]),
                            Ok(vec![b'x']),
                        ]);
                        Body::from_stream(chunks)
                    }
                    _ => {
                        return Response::builder()
                            .status(StatusCode::NOT_FOUND)
                            .body(Body::from("<Error><Code>NoSuchKey</Code></Error>"))
                            .unwrap()
                    }
                };
                Response::builder().status(StatusCode::OK).body(body).unwrap()
            }
        });
        let task = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        Self {
            endpoint,
            paths,
            task,
        }
    }

    fn storage(&self) -> PluginStorage {
        let config = aws_sdk_s3::config::Builder::new()
            .behavior_version(aws_config::BehaviorVersion::latest())
            .region(Region::new("us-east-1"))
            .credentials_provider(Credentials::new("fixture", "fixture", None, None, "fixture"))
            .endpoint_url(&self.endpoint)
            .force_path_style(true)
            .build();
        PluginStorage {
            backend: StorageBackend::S3 {
                client: S3Client::from_conf(config),
                bucket: "fixture-bucket".into(),
                endpoint: Some(self.endpoint.clone()),
            },
        }
    }
}

async fn read(storage: &PluginStorage, org: Uuid, source: &str) -> anyhow::Result<Option<Vec<u8>>> {
    tokio::time::timeout(Duration::from_secs(5), storage.read_owned_spec(org, source))
        .await
        .expect("owned S3 SDK request timed out")
}

#[tokio::test]
async fn owned_s3_spec_checks_tenant_and_key_before_contacting_storage() {
    let org = Uuid::new_v4();
    let fixture = Fixture::start(org).await;
    let storage = fixture.storage();
    let source = format!("/storage/specs/{org}/valid.json");
    let document = read(&storage, org, &source).await.unwrap().unwrap();
    assert_eq!(document, br#"{"openapi":"3.0.0"}"#);
    assert_eq!(
        fixture.paths.lock().unwrap().as_slice(),
        [format!("/fixture-bucket/specs/{org}/valid.json")]
    );

    assert!(read(&storage, Uuid::new_v4(), &source).await.is_err());
    for filename in [
        "../valid.json",
        "nested/valid.json",
        "%2e%2e%2fvalid.json",
        "valid.json?version=1",
    ] {
        assert!(read(&storage, org, &format!("/storage/specs/{org}/{filename}")).await.is_err());
    }
    assert!(read(&storage, org, "https://fixture.invalid/external.json")
        .await
        .unwrap()
        .is_none());
    assert_eq!(
        fixture.paths.lock().unwrap().len(),
        1,
        "denied references must issue no SDK request"
    );

    // Exercise the actual public URL shape returned for the default S3 backend.
    let public = format!("https://fixture-bucket.s3.amazonaws.com/specs/{org}/valid.json");
    assert_eq!(read(&storage, org, &public).await.unwrap().unwrap(), document);
    assert_eq!(fixture.paths.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn owned_s3_spec_enforces_streamed_size_and_missing_object_errors() {
    let org = Uuid::new_v4();
    let fixture = Fixture::start(org).await;
    let storage = fixture.storage();
    let source = |filename| format!("/storage/specs/{org}/{filename}");
    assert_eq!(read(&storage, org, &source("exact.json")).await.unwrap().unwrap().len(), LIMIT);
    let error = read(&storage, org, &source("overflow.json")).await.unwrap_err();
    assert_eq!(error.to_string(), "Specification exceeds 2 MiB");
    let error = read(&storage, org, &source("missing.json")).await.unwrap_err();
    assert_eq!(error.to_string(), "Uploaded specification is unavailable");
    assert_eq!(fixture.paths.lock().unwrap().len(), 3);
}

#[tokio::test]
async fn owned_s3_spec_accepts_current_and_pre_1236_object_urls() {
    let org = Uuid::new_v4();
    let fixture = Fixture::start(org).await;
    let storage = fixture.storage();
    let path_style = format!("{}/fixture-bucket/specs/{org}/valid.json", fixture.endpoint);
    let legacy = format!("https://fixture-bucket.s3.amazonaws.com/specs/{org}/valid.json");
    for source in [&path_style, &legacy] {
        let document = read(&storage, org, source).await.unwrap();
        assert_eq!(document.as_deref(), Some(&br#"{"openapi":"3.0.0"}"#[..]), "{source}");
    }
    let foreign_bucket = format!("{}/other-bucket/specs/{org}/valid.json", fixture.endpoint);
    assert_eq!(read(&storage, org, &foreign_bucket).await.unwrap(), None);
}
