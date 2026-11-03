-- Only authoritative operator reconciliation may release uncertain credits.
ALTER TABLE ai_token_reservations
    ADD COLUMN reconciled_by UUID,
    ADD COLUMN reconciliation_reference TEXT,
    ADD COLUMN reconciled_at TIMESTAMPTZ,
    ADD CONSTRAINT ai_reconciliation_complete CHECK (
        (reconciled_by IS NULL AND reconciliation_reference IS NULL AND reconciled_at IS NULL)
        OR (reconciled_by IS NOT NULL AND reconciliation_reference IS NOT NULL
            AND reconciled_at IS NOT NULL AND state = 'completed')
    );
