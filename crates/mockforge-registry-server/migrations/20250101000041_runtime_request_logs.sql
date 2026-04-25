-- Phase 6 (#232): structured request logs shipped from hosted-mock containers.
--
-- Each row is one HTTP request observed inside a deployed mockforge-cli
-- instance. The in-container shipper batches and POSTs to
-- /api/v1/hosted-mocks/{id}/log-ingest; the cloud admin UI reads them back
-- via /runtime-requests for the per-deployment "Requests" tab.
--
-- Retention is enforced by the existing org-quota worker (out of scope for
-- this migration). Free/Pro/Team plans get progressively longer retention;
-- the worker prunes by `created_at`.

CREATE TABLE IF NOT EXISTS runtime_request_logs (
    id BIGSERIAL PRIMARY KEY,
    deployment_id UUID NOT NULL REFERENCES hosted_mocks(id) ON DELETE CASCADE,
    -- Wall-clock timestamp captured inside the container at request time.
    -- Distinct from `created_at`, which is set when the row lands in Postgres.
    occurred_at TIMESTAMPTZ NOT NULL,
    method TEXT NOT NULL,
    path TEXT NOT NULL,
    status SMALLINT NOT NULL,
    latency_ms INTEGER NOT NULL,
    matched_route TEXT,
    client_ip TEXT,
    user_agent TEXT,
    request_id TEXT,
    bytes_in BIGINT,
    bytes_out BIGINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Most queries scope by deployment + a time window: "show me the last N
-- requests for this deployment" or "show me everything since timestamp T".
CREATE INDEX IF NOT EXISTS runtime_request_logs_deployment_time_idx
    ON runtime_request_logs (deployment_id, occurred_at DESC);

-- Diagnostics index for "what 5xxs has this deployment seen recently".
CREATE INDEX IF NOT EXISTS runtime_request_logs_status_idx
    ON runtime_request_logs (deployment_id, status, occurred_at DESC)
    WHERE status >= 500;
