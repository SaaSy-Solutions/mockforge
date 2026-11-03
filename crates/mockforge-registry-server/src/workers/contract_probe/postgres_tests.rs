use super::enqueue_for_service;
use chrono::{Duration, TimeZone, Utc};
use mockforge_registry_core::models::MonitoredService;
use sqlx::{postgres::PgPoolOptions, PgPool};
use uuid::Uuid;

async fn count_runs(pool: &PgPool, service: Uuid) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM test_runs WHERE suite_id = $1")
        .bind(service)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn seed_service(pool: &PgPool, workspace: Uuid, cron: Option<&str>) -> MonitoredService {
    sqlx::query_as(
        "INSERT INTO monitored_services(workspace_id,name,base_url,openapi_spec_url,traffic_source,probe_cron,probe_interval_secs,created_at) \
         VALUES($1,'probe fixture','https://example.test','https://example.test/api.json','probe',$2,60,'2026-01-01T12:00:00Z') RETURNING *",
    ).bind(workspace).bind(cron).fetch_one(pool).await.unwrap()
}

#[tokio::test]
#[ignore = "requires a disposable PostgreSQL fixture with CREATEDB privileges"]
async fn due_claims_serialize_and_failed_timestamp_rolls_back_run() {
    let url = std::env::var("MOCKFORGE_PROBE_TEST_DATABASE_URL")
        .expect("set the dedicated disposable fixture URL");
    let options: sqlx::postgres::PgConnectOptions = url.parse().unwrap();
    let admin = PgPoolOptions::new()
        .max_connections(2)
        .connect_with(options.clone())
        .await
        .unwrap();
    // A separate database keeps the running registry's background worker
    // from competing with this test's deliberately controlled timestamps.
    let database = format!("probe_fixture_{}", Uuid::new_v4().simple());
    sqlx::query(&format!("CREATE DATABASE {database}"))
        .execute(&admin)
        .await
        .unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(6)
        .connect_with(options.database(&database))
        .await
        .unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    let user = Uuid::new_v4();
    let org = Uuid::new_v4();
    let workspace = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users(id,username,email,password_hash) VALUES($1,$2,$3,'fixture-only')",
    )
    .bind(user)
    .bind(format!("probe-{user}"))
    .bind(format!("probe-{user}@example.test"))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO organizations(id,name,slug,owner_id) VALUES($1,'probe fixture',$2,$3)",
    )
    .bind(org)
    .bind(format!("probe-{org}"))
    .bind(user)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO workspaces(id,org_id,name,created_by) VALUES($1,$2,'probe fixture',$3)",
    )
    .bind(workspace)
    .bind(org)
    .bind(user)
    .execute(&pool)
    .await
    .unwrap();

    let cron = seed_service(&pool, workspace, Some("0 */5 * * * *")).await;
    let fire = Utc.with_ymd_and_hms(2026, 1, 1, 12, 5, 0).unwrap();
    assert!(!enqueue_for_service(&pool, None, &cron, fire - Duration::seconds(1), 1800)
        .await
        .unwrap());
    assert_eq!(count_runs(&pool, cron.id).await, 0);
    let (a, b) = tokio::join!(
        enqueue_for_service(&pool, None, &cron, fire, 1800),
        enqueue_for_service(&pool, None, &cron, fire, 1800),
    );
    assert_eq!(usize::from(a.unwrap()) + usize::from(b.unwrap()), 1);
    assert_eq!(count_runs(&pool, cron.id).await, 1);
    let attribution: (Uuid, String, String, Option<Uuid>) = sqlx::query_as(
        "SELECT org_id,kind,triggered_by,triggered_by_user FROM test_runs WHERE suite_id=$1",
    )
    .bind(cron.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(attribution, (org, "contract_diff".into(), "scheduled".into(), None));
    assert!(!enqueue_for_service(&pool, None, &cron, fire, 1800).await.unwrap());
    assert!(enqueue_for_service(&pool, None, &cron, fire + Duration::minutes(5), 1800)
        .await
        .unwrap());
    assert_eq!(count_runs(&pool, cron.id).await, 2);

    let interval = seed_service(&pool, workspace, None).await;
    assert!(enqueue_for_service(&pool, None, &interval, fire, 1800).await.unwrap());
    assert!(!enqueue_for_service(&pool, None, &interval, fire + Duration::seconds(59), 1800)
        .await
        .unwrap());
    assert!(enqueue_for_service(&pool, None, &interval, fire + Duration::seconds(60), 1800)
        .await
        .unwrap());
    assert_eq!(count_runs(&pool, interval.id).await, 2);

    let missing_spec = seed_service(&pool, workspace, None).await;
    sqlx::query("UPDATE monitored_services SET openapi_spec_url=NULL WHERE id=$1")
        .bind(missing_spec.id)
        .execute(&pool)
        .await
        .unwrap();
    // Hold a different connection so the lock check cannot reuse the claim connection.
    let mut observer = pool.acquire().await.unwrap();
    assert!(!enqueue_for_service(&pool, None, &missing_spec, fire, 1800).await.unwrap());
    assert_eq!(count_runs(&pool, missing_spec.id).await, 0);
    sqlx::query("BEGIN").execute(&mut *observer).await.unwrap();
    sqlx::query("SELECT id FROM monitored_services WHERE id=$1 FOR UPDATE NOWAIT")
        .bind(missing_spec.id)
        .fetch_one(&mut *observer)
        .await
        .expect("skipped probes must release their row lock before returning");
    sqlx::query("ROLLBACK").execute(&mut *observer).await.unwrap();
    drop(observer);
    let untouched = MonitoredService::find_by_id(&pool, missing_spec.id).await.unwrap().unwrap();
    assert!(untouched.last_probed_at.is_none());

    let rollback = seed_service(&pool, workspace, Some("0 */5 * * * *")).await;
    sqlx::query(&format!(
        "CREATE FUNCTION reject_fixture_probe_timestamp() RETURNS trigger AS $$ \
         BEGIN IF NEW.id = '{}'::uuid THEN RAISE EXCEPTION 'fixture timestamp failure'; END IF; RETURN NEW; END; \
         $$ LANGUAGE plpgsql", rollback.id,
    )).execute(&pool).await.unwrap();
    sqlx::query("CREATE TRIGGER reject_fixture_probe_timestamp BEFORE UPDATE OF last_probed_at ON monitored_services FOR EACH ROW EXECUTE FUNCTION reject_fixture_probe_timestamp()")
        .execute(&pool).await.unwrap();
    assert!(enqueue_for_service(&pool, None, &rollback, fire, 1800).await.is_err());
    assert_eq!(
        count_runs(&pool, rollback.id).await,
        0,
        "failed stamp must roll back the queued run"
    );
    let restored = MonitoredService::find_by_id(&pool, rollback.id).await.unwrap().unwrap();
    assert!(restored.last_probed_at.is_none());
    sqlx::query("DROP TRIGGER reject_fixture_probe_timestamp ON monitored_services")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DROP FUNCTION reject_fixture_probe_timestamp()")
        .execute(&pool)
        .await
        .unwrap();
    assert!(enqueue_for_service(&pool, None, &rollback, fire, 1800).await.unwrap());
    assert_eq!(count_runs(&pool, rollback.id).await, 1);

    sqlx::query("UPDATE monitored_services SET enabled=FALSE WHERE id=$1")
        .bind(cron.id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(!enqueue_for_service(&pool, None, &cron, fire + Duration::minutes(10), 1800)
        .await
        .unwrap());
    assert_eq!(count_runs(&pool, cron.id).await, 2);

    pool.close().await;
    sqlx::query(&format!("DROP DATABASE {database} WITH (FORCE)"))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
}
