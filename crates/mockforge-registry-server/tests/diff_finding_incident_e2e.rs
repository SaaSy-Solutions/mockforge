//! End-to-end: runner → registry → incident dispatch (#719).
//!
//! Uses the runner's `RegistryCallbacks` client to POST through the real
//! registry HTTP endpoint, then dispatches to a local webhook receiver:
//!
//! 1. A synthetic **breaking** `diff_finding` raises a `contract_drift`
//!    incident at `critical` severity, attributed to the run's org,
//!    deduped on `(run_id, endpoint)`.
//! 2. A repeat report for the same endpoint collapses onto the same
//!    open incident (dedupe), and a `medium` finding raises nothing.
//! 3. The dispatcher delivers the attributed incident to the configured
//!    webhook and records both delivery and dispatch. Repeated ticks do
//!    not deliver the trigger again.
//!
//! `#[ignore]`-gated like the other `*_e2e.rs` suites — needs live
//! Postgres and a running registry with the same internal callback token:
//!
//! ```text
//! DATABASE_URL=postgres://postgres:postgres@localhost:55433/mockforge \
//! REGISTRY_URL=http://localhost:8080 \
//! MOCKFORGE_INTERNAL_API_TOKEN=<local-fixture-token> \
//! cargo test -p mockforge-registry-server --test diff_finding_incident_e2e -- --ignored --nocapture
//! ```

use axum::{routing::post, Json, Router};
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};
use std::time::Duration;
use tokio::sync::mpsc;
use uuid::Uuid;

use mockforge_registry_core::models::notification_channel::CreateNotificationChannel;
use mockforge_registry_core::models::test_run::EnqueueTestRun;
use mockforge_registry_core::models::NotificationChannel;
use mockforge_registry_core::models::{Incident, TestRun};
use mockforge_test_runner::callbacks::RegistryCallbacks;

async fn pool() -> PgPool {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&database_url)
        .await
        .expect("DB connect failed");
    // Idempotent across repeated runs; includes 20250101000083 (#720).
    sqlx::migrate!("./migrations").run(&pool).await.expect("migrations failed");
    pool
}

/// Seed user → org → workspace → suite → run with raw SQL (models demand
/// more context than this flow exercises) and return the queued run.
async fn seed_run(pool: &PgPool) -> TestRun {
    let user_id = Uuid::new_v4();
    let org_id = Uuid::new_v4();
    let workspace_id = Uuid::new_v4();
    let suite_id = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO users (id, username, email, password_hash) \
         VALUES ($1, $2, $3, 'x')",
    )
    .bind(user_id)
    .bind(format!("e2e-{user_id}"))
    .bind(format!("e2e-{user_id}@example.test"))
    .execute(pool)
    .await
    .expect("seed user");

    sqlx::query("INSERT INTO organizations (id, name, slug, owner_id) VALUES ($1, 'e2e', $2, $3)")
        .bind(org_id)
        .bind(format!("e2e-{org_id}"))
        .bind(user_id)
        .execute(pool)
        .await
        .expect("seed org");

    sqlx::query("INSERT INTO workspaces (id, org_id, name, created_by) VALUES ($1, $2, 'e2e', $3)")
        .bind(workspace_id)
        .bind(org_id)
        .bind(user_id)
        .execute(pool)
        .await
        .expect("seed workspace");

    sqlx::query(
        "INSERT INTO test_suites (id, workspace_id, name, kind, config) \
         VALUES ($1, $2, 'e2e-suite', 'contract_diff', '{}')",
    )
    .bind(suite_id)
    .bind(workspace_id)
    .execute(pool)
    .await
    .expect("seed suite");

    TestRun::enqueue(
        pool,
        EnqueueTestRun {
            suite_id,
            org_id,
            kind: "contract_diff",
            triggered_by: "scheduled",
            triggered_by_user: None,
            git_ref: None,
            git_sha: None,
        },
    )
    .await
    .expect("enqueue run")
}

fn breaking_payload(endpoint: &str) -> serde_json::Value {
    serde_json::json!({
        "severity": "breaking",
        "endpoint": endpoint,
        "description": "GET /pets/200 response schema no longer matches spec",
    })
}

async fn open_incident_count(pool: &PgPool, org_id: Uuid, source: &str) -> i64 {
    sqlx::query(
        "SELECT COUNT(*) AS n FROM incidents WHERE org_id = $1 AND source = $2 AND status = 'open'",
    )
    .bind(org_id)
    .bind(source)
    .fetch_one(pool)
    .await
    .expect("count incidents")
    .get("n")
}

async fn ingest(callbacks: &RegistryCallbacks, run_id: Uuid, seq: u32, payload: serde_json::Value) {
    callbacks
        .run_event(run_id, seq, "diff_finding", payload)
        .await
        .expect("ingest event");
}

#[tokio::test]
#[ignore = "requires fixture Postgres and registry HTTP server"]
async fn breaking_finding_raises_dedupes_and_dispatches() {
    let pool = pool().await;
    let run = seed_run(&pool).await;
    let registry_url = std::env::var("REGISTRY_URL").expect("REGISTRY_URL must be set");
    let token = std::env::var("MOCKFORGE_INTERNAL_API_TOKEN")
        .expect("MOCKFORGE_INTERNAL_API_TOKEN must be set");
    let callbacks = RegistryCallbacks::new(registry_url.clone(), token);
    let rejected = RegistryCallbacks::new(registry_url, "incorrect-fixture-token".into());
    assert!(rejected
        .run_event(run.id, 1, "diff_finding", breaking_payload("/pets/200"))
        .await
        .is_err());
    let stored: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM test_run_events WHERE run_id = $1")
        .bind(run.id)
        .fetch_one(&pool)
        .await
        .expect("count rejected events");
    assert_eq!(stored, 0, "rejected callback must not persist an event");

    let (sender, mut received) = mpsc::channel::<serde_json::Value>(4);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind receiver");
    let webhook_url = format!("http://{}/notify", listener.local_addr().expect("receiver address"));
    let receiver = Router::new().route(
        "/notify",
        post(move |Json(body): Json<serde_json::Value>| {
            let sender = sender.clone();
            async move {
                sender.send(body).await.expect("receive notification");
                Json(serde_json::json!({"accepted": true}))
            }
        }),
    );
    let _server = ReceiverServer(tokio::spawn(async move {
        axum::serve(listener, receiver).await.expect("serve receiver");
    }));
    let config = serde_json::json!({"url": webhook_url});
    let channel = NotificationChannel::create(
        &pool,
        CreateNotificationChannel {
            org_id: run.org_id,
            name: "incident HTTP fixture",
            kind: "webhook",
            config: &config,
            enabled: true,
        },
    )
    .await
    .expect("create fixture channel");

    // --- runner emits a synthetic BREAKING diff_finding -------------------
    ingest(&callbacks, run.id, 1, breaking_payload("/pets/200")).await;
    // A retry of the same wire event must keep exactly one persisted row.
    ingest(&callbacks, run.id, 1, breaking_payload("/pets/200")).await;
    let stored: serde_json::Value = sqlx::query_scalar(
        "SELECT payload FROM test_run_events WHERE run_id = $1 AND seq = 1 AND event_type = 'diff_finding'",
    ).bind(run.id).fetch_one(&pool).await.expect("reload actual HTTP event");
    assert_eq!(stored, breaking_payload("/pets/200"));
    let event_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM test_run_events WHERE run_id = $1 AND seq = 1")
            .bind(run.id)
            .fetch_one(&pool)
            .await
            .expect("count retried wire event");
    assert_eq!(event_count, 1, "same wire event must persist exactly once");

    let incidents = Incident::list_by_org(
        &mut pool.acquire().await.expect("acquire"),
        run.org_id,
        Some("open"),
        10,
    )
    .await
    .expect("list incidents");
    assert_eq!(incidents.len(), 1, "exactly one open incident after one breaking finding");
    let incident = &incidents[0];
    assert_eq!(incident.source, "contract_drift");
    assert_eq!(incident.severity, "critical", "breaking maps to critical");
    assert!(
        incident.dedupe_key.starts_with(&format!("contract-drift:{}", run.id)),
        "dedupe key scoped per (run, endpoint), got {}",
        incident.dedupe_key
    );

    // --- repeat report for the same endpoint collapses via dedupe --------
    ingest(&callbacks, run.id, 2, breaking_payload("/pets/200")).await;
    assert_eq!(
        open_incident_count(&pool, run.org_id, "contract_drift").await,
        1,
        "duplicate finding must not open a second incident"
    );

    // --- milder findings never page --------------------------------------
    let medium = serde_json::json!({
        "severity": "medium",
        "endpoint": "/pets/200",
        "description": "optional field drift",
    });
    ingest(&callbacks, run.id, 3, medium).await;
    assert_eq!(
        open_incident_count(&pool, run.org_id, "contract_drift").await,
        1,
        "medium finding must stay an event, not an incident"
    );

    // Wait for the live server's worker rather than racing it with a second
    // dispatcher. This also proves that the worker is started in the server.
    let notification = tokio::time::timeout(Duration::from_secs(15), received.recv())
        .await
        .expect("webhook delivery deadline")
        .expect("webhook delivered");
    assert_eq!(notification["event_kind"], "trigger");
    assert_eq!(notification["incident"]["id"], incident.id.to_string());
    assert_eq!(notification["incident"]["org_id"], run.org_id.to_string());
    assert_eq!(notification["incident"]["source"], "contract_drift");
    assert_eq!(notification["incident"]["severity"], "critical");
    // The receiver sees the HTTP request before the worker can finish writing
    // its timeline. Wait for completion before asserting the persisted result.
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let marked: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM incident_events WHERE incident_id = $1 AND event_type = 'notification_dispatched'",
            )
            .bind(incident.id)
            .fetch_one(&pool)
            .await
            .expect("reload dispatch marker");
            if marked > 0 {
                assert_eq!(marked, 1, "one completed trigger dispatch");
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("dispatch completion deadline");
    let sent: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM incident_events WHERE incident_id = $1 AND event_type = 'notification_sent'",
    ).bind(incident.id).fetch_one(&pool).await.expect("delivery timeline");
    assert_eq!(sent, 1, "one successful channel delivery");
    let delivery: serde_json::Value = sqlx::query_scalar(
        "SELECT payload FROM incident_events WHERE incident_id = $1 AND event_type = 'notification_sent'",
    )
    .bind(incident.id)
    .fetch_one(&pool)
    .await
    .expect("reload delivery result");
    assert_eq!(delivery["channel_id"], channel.id.to_string());
    assert_eq!(delivery["ok"], true);
    assert_eq!(delivery["status_code"], 200);
    let client = reqwest::Client::new();
    mockforge_registry_server::workers::incident_dispatcher::run_tick(&pool, &client)
        .await
        .expect("repeat dispatcher tick");
    assert!(received.try_recv().is_err(), "repeat tick must not redeliver");
    NotificationChannel::delete(&pool, channel.id)
        .await
        .expect("remove fixture channel");
}

struct ReceiverServer(tokio::task::JoinHandle<()>);

impl Drop for ReceiverServer {
    fn drop(&mut self) {
        self.0.abort();
    }
}
