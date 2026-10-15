# Contract probe schedules

Create a monitored service through
`POST /api/v1/workspaces/{workspace_id}/monitored-services`. The caller must
have access to the workspace. A probe requires an enabled service with a
nonempty `openapi_spec_url`.

Use `probe_cron` for a seconds-first cron expression and `probe_timezone` for
an IANA timezone. UTC is the default. For example, this service probes every
five minutes:

```json
{
  "name": "Orders API",
  "base_url": "https://api.example.test",
  "openapi_spec_url": "https://api.example.test/openapi.json",
  "traffic_source": "probe",
  "probe_cron": "0 */5 * * * *",
  "probe_timezone": "UTC"
}
```

For a daily 09:00 New York schedule, use `0 0 9 * * *` and
`America/New_York`. Local times follow that timezone's daylight saving rules.
Invalid expressions and timezone names return a visible API validation error.

When `probe_cron` is present, it takes precedence over `probe_interval_secs`.
Otherwise a service uses its positive interval in seconds or the
`MOCKFORGE_CONTRACT_PROBE_INTERVAL_SECS` default of 1,800 seconds. Existing
interval-only services retain their cadence.

The worker checks schedules every 30 seconds. Cron services first fire after
their creation timestamp; interval services are immediately due before their
first probe. Missed cron occurrences coalesce into one queued run, so a
restart does not enqueue a burst of old probes. A row lock and a due-state
recheck serialize competing ticks; the run and its probe timestamp commit in
one database transaction. A failed transaction advances neither.

The database claim and Redis delivery are separate operations. A Redis failure
is logged after the queued run commits; this scheduling path does not promise
automatic queue redelivery or exactly-once executor delivery. Changes to cron
scheduling do not add observability alert ingestion.

## Scheduler regression test

`MOCKFORGE_PROBE_TEST_DATABASE_URL` is a test-only connection URL for the
ignored PostgreSQL scheduler regression. It is unset by default and is not
read by the running registry. Use a disposable PostgreSQL fixture whose role
has `CREATEDB` and the superuser or `BYPASSRLS` privileges required by migrations.
The test creates a separate UUID database, applies SQLx migrations, and checks
concurrent claims, interval compatibility, disabled services, and rollback.
Dispose of the fixture after the test, including when an assertion fails.

With that fixture URL set in the environment, run:

```sh
cargo test --locked -p mockforge-registry-server --lib \
  workers::contract_probe::postgres_tests::due_claims_serialize_and_failed_timestamp_rolls_back_run \
  -- --ignored --nocapture
```
