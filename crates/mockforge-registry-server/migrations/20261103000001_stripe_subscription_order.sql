-- Provider lineage is distinct from local row creation / delivery order.
-- NULL marks legacy rows requiring authenticated Stripe readback before a
-- webhook can select the organization's current subscription.
ALTER TABLE subscriptions
    ADD COLUMN stripe_created_at BIGINT,
    ADD COLUMN last_stripe_event_created BIGINT,
    ADD COLUMN last_stripe_event_id TEXT,
    ADD COLUMN stripe_snapshot_authoritative BOOLEAN NOT NULL DEFAULT FALSE;

-- A readback may include changes newer than its triggering event. Since the
-- provider snapshot has no revision timestamp, subsequent events for that
-- identity must keep reading authoritative state rather than apply snapshots.

-- Entitlements measures past-due grace from updated_at. Reconciliation, flags,
-- and lineage writes must not restart the same grace episode. Leave the shared
-- timestamp function and all other tables unchanged.
CREATE FUNCTION update_subscription_timestamp()
RETURNS TRIGGER AS $$
BEGIN
    IF OLD.status = 'past_due' AND NEW.status = 'past_due' THEN
        NEW.updated_at = OLD.updated_at;
    ELSE
        NEW.updated_at = NOW();
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER update_subscriptions_updated_at ON subscriptions;
CREATE TRIGGER update_subscriptions_updated_at BEFORE UPDATE ON subscriptions
    FOR EACH ROW EXECUTE FUNCTION update_subscription_timestamp();
