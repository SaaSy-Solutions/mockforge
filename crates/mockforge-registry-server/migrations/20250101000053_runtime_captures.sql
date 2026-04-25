-- #234 part 2: persistent recorder captures for hosted-mock deployments.
--
-- The recorder library stores captures in the deployment's local SQLite,
-- which is wiped when the Fly machine restarts. This table is the
-- durable counterpart: the in-container shipper POSTs each completed
-- exchange (request + response) to the registry, which lands here.
--
-- Schema mirrors `RecordedRequest` + `RecordedResponse` in
-- `mockforge-recorder::models` flattened into one row. The recorder's
-- types already serialize headers and query_params as JSON-encoded
-- strings, so we keep them as TEXT here — round-tripping through JSONB
-- would force every consumer to know about the encoding shift.

CREATE TABLE IF NOT EXISTS runtime_captures (
    id BIGSERIAL PRIMARY KEY,
    deployment_id UUID NOT NULL REFERENCES hosted_mocks(id) ON DELETE CASCADE,

    -- The recorder's own UUID for this exchange. We store and surface it
    -- so a row in cloud Postgres can be cross-referenced with the local
    -- SQLite (e.g. when debugging a missing capture).
    capture_id TEXT NOT NULL,

    -- Request side.
    protocol TEXT NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL,
    method TEXT NOT NULL,
    path TEXT NOT NULL,
    query_params TEXT,
    request_headers TEXT NOT NULL,
    request_body TEXT,
    request_body_encoding TEXT NOT NULL,
    client_ip TEXT,
    trace_id TEXT,
    span_id TEXT,
    duration_ms BIGINT,
    status_code INTEGER,
    tags TEXT,

    -- Response side. Nullable because an exchange might be shipped
    -- request-first if the response hasn't been recorded yet — though in
    -- practice the shipper enqueues only after the response is in.
    response_status_code INTEGER,
    response_headers TEXT,
    response_body TEXT,
    response_body_encoding TEXT,
    response_size_bytes BIGINT,
    response_timestamp TIMESTAMPTZ,

    received_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Same exchange ingested twice (retries, redeploy with cached
    -- shipper buffer) is a no-op rather than a duplicate row.
    UNIQUE (deployment_id, capture_id)
);

-- Most queries are "recent captures for this deployment", same shape as
-- the request_logs index.
CREATE INDEX IF NOT EXISTS runtime_captures_deployment_time_idx
    ON runtime_captures (deployment_id, occurred_at DESC);

-- Diagnostics: "show me the last N 5xxs the recorder caught."
CREATE INDEX IF NOT EXISTS runtime_captures_status_idx
    ON runtime_captures (deployment_id, status_code, occurred_at DESC)
    WHERE status_code >= 500;
