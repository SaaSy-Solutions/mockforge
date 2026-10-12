use super::*;
use clap::Parser;

struct Server {
    url: String,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn serve(router: Router) -> Server {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    Server { url, task }
}

#[tokio::test]
async fn proxy_forwards_bytes_records_violations_and_enforces_only_strict_mode() {
    use axum::{extract::Path, http::StatusCode, response::IntoResponse, routing::post};
    use serde_json::json;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    let calls = Arc::new(AtomicUsize::new(0));
    let upstream = serve(
        Router::new()
            .route(
                "/items/{id}",
                post({
                    let calls = calls.clone();
                    move |Path(id): Path<String>| {
                        let calls = calls.clone();
                        async move {
                            calls.fetch_add(1, Ordering::SeqCst);
                            let body = match id.as_str() {
                                "invalid" => r#"{"created":"wrong type"}"#,
                                "malformed" => "not JSON",
                                _ => r#"{"created":true}"#,
                            };
                            ([("content-type", "application/json")], body).into_response()
                        }
                    }
                }),
            )
            .route(
                "/blob",
                get(|| async {
                    ([("content-type", "application/octet-stream")], vec![0, 255, 128, 42])
                        .into_response()
                }),
            )
            .route(
                "/redirect",
                get(|| async {
                    (StatusCode::FOUND, [("location", "/blob")], "redirect body").into_response()
                }),
            ),
    )
    .await;
    let spec = json!({"openapi":"3.0.0","info":{"title":"Proxy fixture","version":"1"},"paths":{
        "/items/{id}":{"post":{
            "parameters":[{"name":"id","in":"path","required":true,"schema":{"type":"string"}},
                {"name":"X-Required","in":"header","required":true,"schema":{"type":"string"}},
                {"name":"session","in":"cookie","required":true,"schema":{"type":"string","enum":["fixture"]}}],
            "requestBody":{"required":true,"content":{"application/json":{"schema":{"type":"object","required":["name"],"properties":{"name":{"type":"string"}}}}}},
            "responses":{"2XX":{"description":"created","content":{"application/json":{"schema":{"type":"object","required":["created"],"properties":{"created":{"type":"boolean"}}}}}}}
        }},
        "/blob":{"get":{"responses":{"200":{"description":"binary","content":{"application/octet-stream":{"schema":{"type":"string","format":"binary"}}}}}}},
        "/redirect":{"get":{"responses":{"302":{"description":"redirect"}}}}
    }});
    let args = |strict| ProxyArgs {
        upstream: upstream.url.parse().unwrap(),
        host: "127.0.0.1".parse().unwrap(),
        port: 0,
        spec: None,
        validate_conformance: true,
        validate_conformance_strict: strict,
    };
    conformance_violations::clear();
    let passive = serve(build_router(&args(false), Some(spec.clone())).unwrap()).await;
    let strict = serve(build_router(&args(true), Some(spec)).unwrap()).await;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let response = client
        .post(format!("{}/items/invalid", passive.url))
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(response.text().await.unwrap(), r#"{"created":"wrong type"}"#);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let rows: serde_json::Value = client
        .get(format!("{}/__mockforge/api/conformance/violations", passive.url))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(rows["violations"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["category"] == "response-shape"));
    assert!(rows["violations"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["category"] != "response-shape"));
    let binary = client.get(format!("{}/blob", passive.url)).send().await.unwrap();
    assert_eq!(binary.bytes().await.unwrap().as_ref(), &[0, 255, 128, 42]);
    let redirect = client.get(format!("{}/redirect", passive.url)).send().await.unwrap();
    assert_eq!(redirect.status(), 302);
    assert_eq!(redirect.headers()["location"], "/blob");
    assert_eq!(redirect.text().await.unwrap(), "redirect body");
    let rejected = client
        .post(format!("{}/items/valid", strict.url))
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(rejected.status(), 400);
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "invalid strict request must not reach upstream"
    );
    let missing_cookie = client
        .post(format!("{}/items/valid", strict.url))
        .header("X-Required", "fixture")
        .json(&json!({"name":"widget"}))
        .send()
        .await
        .unwrap();
    assert_eq!(missing_cookie.status(), 400);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    for id in ["invalid", "malformed"] {
        let response = client
            .post(format!("{}/items/{id}", strict.url))
            .header("X-Required", "fixture")
            .header("Cookie", "session=\"fixture\"")
            .json(&json!({"name":"widget"}))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 502);
    }
    let valid = client
        .post(format!("{}/items/valid", strict.url))
        .header("X-Required", "fixture")
        .header("Cookie", "session=\"fixture\"")
        .json(&json!({"name":"widget"}))
        .send()
        .await
        .unwrap();
    assert_eq!(valid.status(), 200, "2XX response range must accept a valid 200 response");
    assert_eq!(valid.json::<serde_json::Value>().await.unwrap(), json!({"created":true}));
    assert_eq!(calls.load(Ordering::SeqCst), 4);
}

#[test]
fn proxy_cli_requires_valid_upstream_and_validation_spec() {
    #[derive(Parser)]
    struct Arguments {
        #[command(flatten)]
        args: ProxyArgs,
    }
    let cli = Arguments::try_parse_from([
        "proxy",
        "--upstream",
        "http://127.0.0.1:9090",
        "--validate-conformance-strict",
    ])
    .unwrap();
    assert!(build_router(&cli.args, None).is_err());
    let cli = Arguments::try_parse_from(["proxy", "--upstream", "file:///tmp/upstream"]).unwrap();
    assert!(build_router(&cli.args, None).is_err());
}
