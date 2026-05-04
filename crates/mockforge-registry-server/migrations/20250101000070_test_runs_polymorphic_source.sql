-- Make test_runs.suite_id polymorphic (cloud-enablement task #4 / Phase 3).
--
-- The original schema constrained suite_id with a FK to test_suites because
-- test runs were assumed to belong to a user-authored suite. Tasks #6 #7 #8
-- #9 #10 reuse test_runs as a generic worker-pool table — chaos campaigns,
-- snapshot captures, contract diffs, flows, replays, and clone-training all
-- create rows with a non-suite source_id. Keeping the FK would reject those
-- inserts at runtime; keeping the kind only on test_suites would force every
-- cloud feature to bootstrap a fake test_suites row.
--
-- Resolution:
--   1. Drop the FK so suite_id is a generic owning-resource id.
--   2. Add a kind column on test_runs so the worker callback path can
--      route by kind without joining test_suites (which won't exist for
--      non-suite runs).
--
-- Column keeps its name (`suite_id`) to avoid churning every query string
-- and the offline sqlx cache; it now means "id of the resource that owns
-- this run, kind-dependent" — see test_runs.kind for which table to join.

ALTER TABLE test_runs
    DROP CONSTRAINT IF EXISTS test_runs_suite_id_fkey;

ALTER TABLE test_runs
    ADD COLUMN IF NOT EXISTS kind TEXT;

-- Backfill: existing rows pre-migration are all suite-kind, so copy from
-- test_suites where the row still exists. Anything else gets 'unit' as a
-- conservative default — Phase 1 only used unit/integration/conformance/
-- bench/owasp via test_suites.
UPDATE test_runs r
   SET kind = COALESCE(s.kind, 'unit')
  FROM test_suites s
 WHERE r.suite_id = s.id
   AND r.kind IS NULL;

UPDATE test_runs SET kind = 'unit' WHERE kind IS NULL;

ALTER TABLE test_runs
    ALTER COLUMN kind SET NOT NULL;

CREATE INDEX IF NOT EXISTS idx_test_runs_kind_status
    ON test_runs(kind, status);
