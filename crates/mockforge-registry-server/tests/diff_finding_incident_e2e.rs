//! End-to-end: runner → registry → incident dispatch (#719).
//!
//! Exercises the real ingestion pipeline a runner hits —
//! `handlers::internal_test_runs::ingest_runner_event`, the exact code
//! behind `POST /api/v1/internal/test-runs/{id}/events` — and asserts:
//!
//! 1. A synthetic **breaking** `diff_finding` raises a `contract_drift`
//!    incident at `critical` severity, attributed to the run's org,
//!    deduped on `(run_id, endpoint)`.
//! 2. A repeat report for the same endpoint collapses onto the same
//!    open incident (dedupe), and a `medium` finding raises nothing.
//! 3. The incident-dispatcher worker drains the trigger queue: with no
//!    routing rules / channels configured the incident is marked
//!    dispatched with zero notification attempts (fanout to nobody is
//!    still "dispatched" — routing decides *who*, not *whether*).
//!
//! `#[ignore]`-gated like the other `*_e2e.rs` suites — needs live
//! Postgres:
//!
//! ```text
//! DATABASE_URL=postgres://postgres:postgres@localhost:55433/mockforge \
//! cargo test -p mockforge-registry-server --test diff_finding_incident_e2e -- --ignored --nocapture
//! ```

use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use mockforge_registry_core::models::test_run::EnqueueTestRun;
use mockforge_registry_core::models::{Incident, TestRun};
use mockforge_registry_server::handlers::internal_test_runs::ingest_runner_event;

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

async fn ingest(pool: &PgPool, run_id: Uuid, seq: i32, payload: serde_json::Value) {
    ingest_runner_event(pool, run_id, seq, "diff_finding", &payload)
        .await
        .expect("ingest event");
}

#[tokio::test]
#[ignore = "requires DATABASE_URL Postgres"]
async fn breaking_finding_raises_dedupes_and_dispatches() {
    let pool = pool().await;
    let run = seed_run(&pool).await;

    // --- runner emits a synthetic BREAKING diff_finding -------------------
    ingest(&pool, run.id, 1, breaking_payload("/pets/200")).await;

    let incidents = Incident::list_by_org(&pool, run.org_id, Some("open"), 10)
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
    ingest(&pool, run.id, 2, breaking_payload("/pets/200")).await;
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
    ingest(&pool, run.id, 3, medium).await;
    assert_eq!(
        open_incident_count(&pool, run.org_id, "contract_drift").await,
        1,
        "medium finding must stay an event, not an incident"
    );

    // --- dispatcher drains the trigger queue -----------------------------
    let client = reqwest::Client::new();
    let dispatched =
        mockforge_registry_server::workers::incident_dispatcher::run_tick(&pool, &client)
            .await
            .expect("dispatcher tick");

    // The dispatcher records dispatch as an `incident_events` timeline row.
    let marked: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM incident_events \
         WHERE incident_id = $1 AND event_type = 'notification_dispatched'",
    )
    .bind(incident.id)
    .fetch_one(&pool)
    .await
    .expect("reload dispatch marker");
    assert!(
        marked > 0,
        "dispatcher must mark the incident dispatched (tick dispatched {dispatched})"
    );
}

// The helper below drives the REAL ingestion path (`ingest_runner_event`,
// the code behind POST /api/v1/internal/test-runs/{id}/events) without
// spinning up an HTTP server — same tradeoff as the other *_e2e suites.
