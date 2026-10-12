//! Evaluate saved-query alert rules against tenant-scoped persisted telemetry.
use chrono::Utc;
use mockforge_registry_core::{
    incident_bus::{map_alert_severity, IncidentBus, PgConnectionIncidentBus},
    models::{
        incident::{Incident, RaiseIncidentInput},
        observability_alert::{AlertMetric, ObservabilityAlert},
        observability_query::ObservabilitySavedQuery,
    },
};
use sqlx::{Acquire, PgConnection, PgPool};
use std::time::Duration;
use tracing::{error, warn};

pub fn start_observability_alert_worker(pool: PgPool) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        interval.tick().await;
        loop {
            interval.tick().await;
            if let Err(error) = run_tick(&pool).await {
                error!(%error, "observability alert scan failed");
            }
        }
    });
}

pub async fn run_tick(pool: &PgPool) -> sqlx::Result<()> {
    let mut tick = pool.begin().await?;
    // One evaluator across replicas: raising and recovery must not race.
    // A transaction-scoped lock releases on cancellation/rollback as well as
    // success, and all work stays on this one pool connection.
    let claimed: bool = sqlx::query_scalar("SELECT pg_try_advisory_xact_lock($1)")
        .bind(0x4d46_4f42_5341_4c52_i64)
        .fetch_one(&mut *tick)
        .await?;
    if !claimed {
        return Ok(());
    }
    let queries = sqlx::query_as::<_, ObservabilitySavedQuery>(
        "SELECT * FROM observability_saved_queries WHERE filters->'alert' IS NOT NULL AND filters->'alert' <> 'null'::jsonb",
    ).fetch_all(&mut *tick).await?;
    for query in queries {
        let mut evaluation = tick.begin().await?;
        match evaluate_query(&mut evaluation, &query).await {
            Ok(()) => evaluation.commit().await?,
            Err(error) => {
                evaluation.rollback().await?;
                warn!(query_id=%query.id, %error, "observability alert evaluation failed");
            }
        }
    }
    tick.commit().await
}

async fn evaluate_query(
    connection: &mut PgConnection,
    query: &ObservabilitySavedQuery,
) -> sqlx::Result<()> {
    let rule =
        match ObservabilityAlert::from_filters(&query.filters, &query.kind, query.workspace_id) {
            Ok(Some(rule)) if rule.enabled => rule,
            Ok(_) => return Ok(()),
            Err(error) => {
                warn!(query_id=%query.id, %error, "invalid stored observability alert skipped");
                return Ok(());
            }
        };
    // Owner-pool worker reads must explicitly verify tenant ownership. A
    // historical invalid saved-query workspace must never widen the scope.
    if let Some(workspace) = query.workspace_id {
        let owned: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM workspaces WHERE id=$1 AND org_id=$2)")
                .bind(workspace)
                .bind(query.org_id)
                .fetch_one(&mut *connection)
                .await?;
        if !owned {
            return Ok(());
        }
    }
    let source = match query.kind.as_str() {
        "logs" => "SELECT r.deployment_id, r.occurred_at, CASE WHEN r.status BETWEEN 100 AND 599 THEN r.status >= 500 END AS failed, CASE WHEN r.latency_ms >= 0 THEN r.latency_ms::double precision END AS latency, r.path, NULL::text AS service_name, NULL::uuid AS workspace_id, NULL::text AS protocol FROM runtime_request_logs r",
        "traces" => "SELECT r.deployment_id, r.occurred_at, CASE WHEN r.status_code IN (0,1,2) THEN r.status_code = 2 END AS failed, CASE WHEN r.end_unix_nano >= r.start_unix_nano THEN (r.end_unix_nano::numeric-r.start_unix_nano::numeric)::double precision/1000000.0 END AS latency, NULL::text AS path, r.service_name, NULL::uuid AS workspace_id, NULL::text AS protocol FROM runtime_traces r",
        "metrics" => "SELECT r.deployment_id, r.occurred_at, CASE WHEN r.protocol='grpc' AND r.response_status_code BETWEEN 0 AND 16 THEN r.response_status_code <> 0 WHEN r.protocol='graphql' AND COALESCE(r.response_status_code,r.status_code) BETWEEN 100 AND 599 THEN COALESCE(r.response_status_code,r.status_code) >= 400 WHEN r.protocol IN ('http','websocket','kafka','mqtt','amqp') AND COALESCE(r.response_status_code,r.status_code) BETWEEN 100 AND 599 THEN COALESCE(r.response_status_code,r.status_code) >= 500 END AS failed, CASE WHEN r.duration_ms >= 0 THEN r.duration_ms::double precision END AS latency, r.path, NULL::text AS service_name, r.workspace_id, r.protocol FROM runtime_captures r",
        _ => return Ok(()),
    };
    // Only the fixed source SQL above is interpolated. Every user filter is
    // bound, including literal substring matching (no LIKE wildcard expansion).
    let statement = format!("SELECT COUNT(*) FILTER (WHERE CASE $8::text WHEN 'errors' THEN s.failed IS NOT NULL WHEN 'latency' THEN s.latency IS NOT NULL ELSE TRUE END)::bigint, COUNT(*) FILTER (WHERE s.failed)::bigint, percentile_cont(0.95) WITHIN GROUP (ORDER BY s.latency) FROM ({source}) s JOIN hosted_mocks hm ON hm.id=s.deployment_id WHERE hm.org_id=$1 AND hm.deleted_at IS NULL AND ($2::uuid IS NULL OR s.workspace_id=$2) AND ($3::uuid IS NULL OR s.deployment_id=$3) AND ($4::text IS NULL OR s.service_name=$4) AND ($5::text IS NULL OR strpos(s.path,$5)>0) AND ($9::text IS NULL OR s.protocol=$9) AND s.occurred_at >= $6 AND s.occurred_at <= $7");
    let observations = match rule.metric {
        AlertMetric::ErrorCount | AlertMetric::ErrorRate => "errors",
        AlertMetric::LatencyP95Ms => "latency",
        AlertMetric::SampleCount => "all",
    };
    let now = Utc::now();
    let (samples, errors, latency): (i64, i64, Option<f64>) = sqlx::query_as(&statement)
        .bind(query.org_id)
        .bind(query.workspace_id)
        .bind(rule.deployment_id)
        .bind(&rule.service_name)
        .bind(&rule.path_contains)
        .bind(now - chrono::Duration::minutes(i64::from(rule.window_minutes)))
        .bind(now)
        .bind(observations)
        .bind(&rule.protocol)
        .fetch_one(&mut *connection)
        .await?;
    let dedupe = query.id.to_string();
    match rule.breached(samples, errors, latency) {
        Some(true) => {
            let bus = PgConnectionIncidentBus::new(connection);
            let description = format!(
                "{:?} threshold={} samples={} errors={} p95_ms={latency:?} window_minutes={}",
                rule.metric, rule.threshold, samples, errors, rule.window_minutes
            );
            bus.raise(RaiseIncidentInput {
                org_id: query.org_id,
                workspace_id: query.workspace_id,
                source: "observability",
                source_ref: Some(&dedupe),
                dedupe_key: &dedupe,
                severity: map_alert_severity(&rule.severity).expect("validated alert severity"),
                title: &query.name,
                description: Some(&description),
            })
            .await?;
        }
        Some(false) => {
            Incident::auto_resolve(connection, query.org_id, "observability", &dedupe).await?;
        }
        None => {} // Missing/insufficient samples are not evidence of recovery.
    }
    Ok(())
}
