-- Neutralise the publicly known seed accounts (#1014).
--
-- 20250101000002_seed_data.sql inserted `admin@mockforge.dev` (is_admin) and
-- `test@example.com`, and ...0027 / ...0028 later set their password hashes to
-- bcrypt hashes of the published passwords `admin123` / `test123`. Applied
-- migrations cannot be edited (sqlx verifies their checksums at boot), so this
-- forward migration disables those rows instead. On a fresh database it runs in
-- the same migration pass as ...0002, before the server accepts a request.
--
-- Safety:
-- * Only rows that are still a seed identity (fixed seed id or seed email) AND
--   still carry one of the six hashes those migrations wrote are touched. An
--   account whose owner set a different password is left exactly as it is.
-- * The rows are disabled, not deleted. Dozens of foreign keys reference users,
--   some ON DELETE RESTRICT (organizations, workspaces, federations, ...) and
--   some ON DELETE CASCADE (plugins, reviews, ...), so a DELETE would either
--   fail the migration or silently cascade away data the account owns.
-- * The replacement password hash is well-formed bcrypt (so login returns
--   "Invalid email or password" instead of a 500) but was written by hand, not
--   derived from any password, so no password verifies against it.
-- * Outstanding refresh tokens are revoked, admin is cleared, and the legacy
--   api_token and OAuth links are removed: anyone who signed in with the public
--   password may have attached them, so they cannot be trusted either.
-- * Idempotent: a second run matches nothing, because the hash has changed.
WITH seeded AS (
    SELECT id
    FROM users
    WHERE (
            id IN (
                '00000000-0000-0000-0000-000000000001'::uuid,
                '00000000-0000-0000-0000-000000000002'::uuid
            )
            OR lower(btrim(email)) IN ('admin@mockforge.dev', 'test@example.com')
        )
        AND password_hash IN (
            -- admin123: ...0002, ...0027, ...0028
            '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewY5lW7BhPx0gFKi',
            '$2b$10$n5RHYuZvSn5XPajDQNJMjuvfwmzzsGCFgS750U/BY1FCLBOcG9tt2',
            '$2b$10$5mcGb.1oYTuLLi9h08MucO1rsfWmJsryFoKtHF6.qv9p5Ge6V9IW6',
            -- test123: ...0002, ...0027, ...0028
            '$2b$12$8mKE/9bqNqNqNqNqNqNqNuKqNqNqNqNqNqNqNqNqNqNqNqNqNqNq',
            '$2b$10$yG7qQ26EKTgS7h0S4YIxtOzARlcuni9IaoDmeTP88h4EXOA8I8Ho.',
            '$2b$10$Uv30udbt8TccWoXbl4r3tufv569B/XxQ2d/SsR.9Fdzjw1/ZroEAi'
        )
),
revoked AS (
    UPDATE token_revocations
    SET revoked_at = NOW(),
        revocation_reason = 'seed credential neutralised (#1014)'
    WHERE user_id IN (SELECT id FROM seeded)
      AND revoked_at IS NULL
    RETURNING jti
)
UPDATE users
SET password_hash = '$2b$12$SeedAdminDisabled1014uNoPasswordEverHashesToThisValue',
    is_admin = FALSE,
    api_token = NULL,
    github_id = NULL,
    google_id = NULL,
    updated_at = NOW()
WHERE id IN (SELECT id FROM seeded);
