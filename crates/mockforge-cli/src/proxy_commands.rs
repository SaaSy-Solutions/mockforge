//! First-class HTTP proxy and passive contract validation (#864).

use anyhow::{bail, Context, Result};
use axum::{extract::Query, routing::get, Json, Router};
use clap::Args;
use mockforge_foundation::conformance_violations;
use mockforge_proxy::{conformance::ConformanceTap, server::ProxyServer, ProxyConfig};
use serde::Deserialize;
use std::{net::IpAddr, path::PathBuf};

#[derive(Debug, Args)]
pub struct ProxyArgs {
    /// Upstream HTTP(S) URL (also accepts --target)
    #[arg(long, alias = "target")]
    upstream: url::Url,
    /// Address to listen on
    #[arg(long, default_value = "127.0.0.1")]
    host: IpAddr,
    /// Proxy listen port
    #[arg(long, default_value_t = 9081)]
    port: u16,
    /// OpenAPI/Swagger document used for conformance validation
    #[arg(long, env = "MOCKFORGE_PROXY_SPEC")]
    spec: Option<PathBuf>,
    /// Record request and response violations while forwarding traffic
    #[arg(long, env = "MOCKFORGE_PROXY_VALIDATE_CONFORMANCE")]
    validate_conformance: bool,
    /// Reject invalid requests and return 502 for invalid upstream responses
    #[arg(long, env = "MOCKFORGE_PROXY_VALIDATE_CONFORMANCE_STRICT")]
    validate_conformance_strict: bool,
}

fn build_router(args: &ProxyArgs, spec: Option<serde_json::Value>) -> Result<Router> {
    if !matches!(args.upstream.scheme(), "http" | "https")
        || args.upstream.host_str().is_none()
        || !args.upstream.username().is_empty()
        || args.upstream.password().is_some()
        || args.upstream.fragment().is_some()
    {
        bail!("--upstream must be an HTTP(S) URL without embedded credentials or a fragment");
    }
    let enabled = args.validate_conformance || args.validate_conformance_strict;
    let tap = if enabled {
        let spec =
            spec.context("Conformance validation requires --spec or MOCKFORGE_PROXY_SPEC")?;
        Some(
            ConformanceTap::from_spec_value(spec, args.validate_conformance_strict)
                .map_err(anyhow::Error::msg)?,
        )
    } else {
        None
    };
    let mut config = ProxyConfig::new(args.upstream.to_string());
    config.prefix = None;
    // Redirect responses are part of the upstream contract, not another request.
    config.follow_redirects = false;
    let mut router = ProxyServer::with_conformance(config, false, false, tap).router();
    if enabled {
        router = router.route(
            "/__mockforge/api/conformance/violations",
            get(violations).delete(clear_violations),
        );
    }
    Ok(router)
}

#[derive(Default, Deserialize)]
struct ListQuery {
    limit: Option<usize>,
}

async fn violations(Query(query): Query<ListQuery>) -> Json<serde_json::Value> {
    let mut rows = conformance_violations::snapshot();
    let total = rows.len();
    if let Some(limit) = query.limit {
        rows.truncate(limit);
    }
    Json(serde_json::json!({"violations": rows, "total": total}))
}

async fn clear_violations() -> Json<serde_json::Value> {
    let before = conformance_violations::len();
    conformance_violations::clear();
    Json(serde_json::json!({"cleared": before}))
}

pub async fn run(args: ProxyArgs) -> Result<()> {
    let spec = if let Some(path) = &args.spec {
        let raw = tokio::fs::read_to_string(path).await.context("Cannot read proxy spec")?;
        Some(
            serde_json::from_str(&raw)
                .or_else(|_| serde_yaml::from_str(&raw))
                .context("Cannot parse proxy spec as JSON or YAML")?,
        )
    } else {
        None
    };
    let router = build_router(&args, spec)?;
    let listener = tokio::net::TcpListener::bind((args.host, args.port))
        .await
        .context("Cannot bind proxy listener")?;
    println!("Proxy listening on {}", listener.local_addr()?);
    axum::serve(listener, router).with_graceful_shutdown(shutdown()).await?;
    Ok(())
}

async fn shutdown() {
    #[cfg(unix)]
    {
        let terminate = async {
            match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
                Ok(mut signal) => {
                    signal.recv().await;
                }
                Err(error) => {
                    tracing::error!(%error, "Cannot listen for SIGTERM");
                    std::future::pending::<()>().await;
                }
            }
        };
        tokio::select! {
            result = tokio::signal::ctrl_c() => {
                if let Err(error) = result { tracing::error!(%error, "Cannot listen for Ctrl-C"); }
            }
            () = terminate => {}
        }
    }
    #[cfg(not(unix))]
    if let Err(error) = tokio::signal::ctrl_c().await {
        tracing::error!(%error, "Cannot listen for Ctrl-C");
    }
}

#[cfg(test)]
mod tests {
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
                        (StatusCode::FOUND, [("location", "/blob")], "redirect body")
                            .into_response()
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
        let cli =
            Arguments::try_parse_from(["proxy", "--upstream", "file:///tmp/upstream"]).unwrap();
        assert!(build_router(&cli.args, None).is_err());
    }
}
