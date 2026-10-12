//! Persona analytics acceptance test over the real HTTP request boundary.
//!
//! This executable owns the process-global analytics database, keeping it
//! isolated from other tests that intentionally run with analytics disabled.

use anyhow::Context;
use axum::{middleware, routing::get, Router};
use mockforge_analytics::{get_global_db, set_global_db, AnalyticsDatabase};
use mockforge_http::collect_http_metrics;
use std::time::Duration;

#[tokio::test]
async fn http_persona_headers_persist_ci_hits_without_changing_responses() -> anyhow::Result<()> {
    assert!(get_global_db().is_none());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let app = Router::new()
        .route("/fixture", get(|| async { "mock response" }))
        .layer(middleware::from_fn(collect_http_metrics));
    let server = tokio::spawn(async move { axum::serve(listener, app).await });
    // Aborting on Drop also cleans up the owned server on assertion failure.
    struct ServerGuard(tokio::task::JoinHandle<std::io::Result<()>>);
    impl Drop for ServerGuard {
        fn drop(&mut self) {
            self.0.abort();
        }
    }
    let _server = ServerGuard(server);
    let client = reqwest::Client::new();
    let url = format!("http://{address}/fixture");

    let disabled = client
        .get(&url)
        .header("X-MockForge-Persona", "analytics-disabled")
        .send()
        .await?;
    assert_eq!(disabled.status(), reqwest::StatusCode::OK);
    assert_eq!(disabled.text().await?, "mock response");
    assert!(get_global_db().is_none());

    let directory = tempfile::tempdir()?;
    let db = AnalyticsDatabase::new(&directory.path().join("analytics.db")).await?;
    db.run_migrations().await?;
    assert!(set_global_db(db.clone()).is_ok());

    for (persona, run) in [
        (Some(" checkout-user "), Some(" gh-run-123 ")),
        (Some("guest-user"), None),
        (Some("  "), Some("ignored-blank")),
        (None, Some("ignored-no-persona")),
    ] {
        let mut request = client.get(&url);
        if let Some(persona) = persona {
            request = request.header("X-MockForge-Persona", persona);
        }
        if let Some(run) = run {
            request = request.header("X-MockForge-CI-Run", run);
        }
        let response = request.send().await?;
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        assert_eq!(response.text().await?, "mock response");
    }

    let invalid_header = reqwest::header::HeaderValue::from_bytes(b"\xff")?;
    let invalid = client.get(&url).header("X-MockForge-Persona", invalid_header).send().await?;
    assert_eq!(invalid.status(), reqwest::StatusCode::OK);
    assert_eq!(invalid.text().await?, "mock response");

    // Recording is asynchronous; wait for observable persistence, not an
    // assumed fixed scheduler delay.
    let hits = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let hits = db.get_persona_ci_hits(None, None, None).await?;
            if hits.len() >= 2 {
                break Ok::<_, mockforge_analytics::AnalyticsError>(hits);
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .context("persona hits must reach the analytics database")??;
    assert_eq!(hits.len(), 2, "blank, absent and invalid persona headers must not record hits");
    let checkout = hits
        .iter()
        .find(|hit| hit.persona_id == "checkout-user")
        .context("checkout hit missing")?;
    assert_eq!(checkout.ci_run_id.as_deref(), Some("gh-run-123"));
    let guest = hits
        .iter()
        .find(|hit| hit.persona_id == "guest-user")
        .context("guest hit missing")?;
    assert_eq!(guest.ci_run_id, None);
    for hit in hits {
        assert_eq!(hit.hit_count, 1);
        assert!(hit.hit_at > 0);
        assert_eq!(hit.workspace_id, None);
        assert_eq!(hit.org_id, None);
    }
    Ok(())
}
