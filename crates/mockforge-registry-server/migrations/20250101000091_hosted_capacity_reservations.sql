-- Resource reservations are independent of customer-editable display status.
-- Legacy stopped/failed/deleted flags do not prove provider resources are gone:
-- the old lifecycle API could change these flags without confirmed cleanup.
-- Keep every legacy row reserved until provider reconciliation confirms that
-- it is stopped or destroyed. New creation reserves a slot immediately.
ALTER TABLE hosted_mocks
    ADD COLUMN capacity_reserved BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN deletion_requested_at TIMESTAMPTZ;

-- Display status is customer-editable; it cannot authorize provider deletion.
-- Legacy soft deletion came from the authorized deletion path and can retry.
UPDATE hosted_mocks SET deletion_requested_at = deleted_at
WHERE deleted_at IS NOT NULL;

CREATE INDEX idx_hosted_mocks_reserved_capacity
    ON hosted_mocks (org_id) WHERE capacity_reserved;

CREATE INDEX idx_hosted_mocks_pending_cleanup
    ON hosted_mocks (updated_at)
    WHERE (capacity_reserved OR deleted_at IS NULL) AND deletion_requested_at IS NOT NULL;
