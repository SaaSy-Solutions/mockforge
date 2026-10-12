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
