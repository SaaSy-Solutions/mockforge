-- Keep provenance with pending/uncertain credit holds, including cancelled callers.
-- Historical IDs remain useful after user/job deletion, like existing audit IDs.
ALTER TABLE ai_token_reservations
    ADD COLUMN call_origin TEXT NOT NULL DEFAULT 'legacy'
        CHECK (call_origin IN ('legacy', 'organization_completion', 'test_generation')),
    ADD COLUMN actor_user_id UUID,
    ADD COLUMN test_generation_job_id UUID,
    ADD CONSTRAINT ai_reservation_job_origin
        CHECK ((call_origin = 'test_generation') = (test_generation_job_id IS NOT NULL));
