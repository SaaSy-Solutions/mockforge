-- Stripe redelivers an event until it gets a 2xx, and can deliver the same
-- event more than once even after one (#1456). Each event id is claimed here
-- before it is acted on, so a repeat is acknowledged without running twice.
--
-- `processed_at IS NULL` marks a claim still in flight. A failed attempt
-- deletes its claim so Stripe's retry runs again; a claim left behind by a
-- crashed process can be retaken after five minutes.
--
-- Global (not org-scoped) and touched only by the webhook on the owner pool,
-- so it carries no RLS policy.
CREATE TABLE stripe_webhook_events (
    event_id TEXT PRIMARY KEY,
    event_type TEXT NOT NULL,
    claimed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    processed_at TIMESTAMPTZ,
    outcome TEXT
);
