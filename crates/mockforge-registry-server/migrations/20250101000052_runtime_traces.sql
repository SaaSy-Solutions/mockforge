-- Phase 6 / #233: persistent OTLP trace storage for hosted-mock deployments.
--
-- The receiver scaffold from PR #236 only counted spans and threw them
-- away. This table actually stores them. Schema is intentionally simple:
-- one row per span, with the OTLP attributes/events/links arrays kept as
-- JSONB so we don't have to model the full OTel data model upfront.
--
-- Storage choice: Postgres-JSONB rather than a dedicated tracing
-- backend (ClickHouse / Tempo). Reasons:
--   * Reuses existing Postgres infra — no new operational dependency.
--   * GIN index on attributes JSONB supports the common "filter by
--     http.method=GET" query without bespoke indexes.
--   * Span volume per deployment is bounded by user traffic; we're not
--     ingesting fleet-wide telemetry. Postgres handles low-cardinality
--     workloads fine.
--
-- If/when volume grows past Postgres's comfort zone, the on-wire
-- protocol stays the same — we'd just swap the storage layer behind
-- the existing handler. That migration is deliberately deferred.

CREATE TABLE IF NOT EXISTS runtime_traces (
    -- Synthetic primary key; multi-column natural keys (trace_id, span_id)
    -- can collide across deployments, and we want per-deployment isolation.
    id BIGSERIAL PRIMARY KEY,
    deployment_id UUID NOT NULL REFERENCES hosted_mocks(id) ON DELETE CASCADE,

    -- OTel identifiers. Hex strings on the wire; stored as TEXT so we
    -- don't lose leading zeros and don't need to choose between u64/i64.
    trace_id TEXT NOT NULL,
    span_id TEXT NOT NULL,
    parent_span_id TEXT,

    -- Span metadata.
    service_name TEXT,
    name TEXT NOT NULL,
    kind SMALLINT, -- INTERNAL=1, SERVER=2, CLIENT=3, PRODUCER=4, CONSUMER=5

    -- Time window. Unix nanos to match OTLP's wire format precisely; we
    -- also denormalize occurred_at as TIMESTAMPTZ so retention queries
    -- can use the ordinary `occurred_at < NOW() - interval '...'` form.
    start_unix_nano BIGINT NOT NULL,
    end_unix_nano BIGINT NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL,

    -- Status and structured details kept as JSONB. Keeps the schema
    -- forward-compatible with new OTLP fields without migrations.
    status_code SMALLINT, -- 0=UNSET, 1=OK, 2=ERROR
    status_message TEXT,
    attributes JSONB NOT NULL DEFAULT '{}'::jsonb,
    events JSONB NOT NULL DEFAULT '[]'::jsonb,
    links JSONB NOT NULL DEFAULT '[]'::jsonb,
    resource_attributes JSONB NOT NULL DEFAULT '{}'::jsonb,

    received_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Most queries scope by deployment + recent time window. The list view
-- groups by trace_id; the detail view fetches all spans of one trace.
CREATE INDEX IF NOT EXISTS runtime_traces_deployment_time_idx
    ON runtime_traces (deployment_id, occurred_at DESC);

-- Lookup by trace_id is the detail-view path. Compound with deployment
-- so the read is fully covered by an index even for high-volume deployments.
CREATE INDEX IF NOT EXISTS runtime_traces_deployment_trace_idx
    ON runtime_traces (deployment_id, trace_id, start_unix_nano);

-- Attribute filtering. GIN supports `attributes @> '{"http.method": "GET"}'`
-- which is the common "drill into a specific endpoint" query.
CREATE INDEX IF NOT EXISTS runtime_traces_attributes_gin_idx
    ON runtime_traces USING GIN (attributes);

-- Error spotting: "show me the last 50 ERROR spans for this deployment"
-- is a common diagnostic flow. Partial index keeps it cheap.
CREATE INDEX IF NOT EXISTS runtime_traces_errors_idx
    ON runtime_traces (deployment_id, occurred_at DESC)
    WHERE status_code = 2;
