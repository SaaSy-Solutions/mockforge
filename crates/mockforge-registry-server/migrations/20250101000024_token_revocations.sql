-- Token revocation tracking for refresh tokens
-- This table stores JTIs (JWT IDs) of refresh tokens to support revocation

CREATE TABLE IF NOT EXISTS token_revocations (
    -- The unique JWT ID from the refresh token
    jti VARCHAR(36) PRIMARY KEY,

    -- The user who owns this token
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    -- When this token was revoked (NULL means it's an active token, not yet revoked)
    revoked_at TIMESTAMPTZ,

    -- When the token expires (for cleanup purposes)
    expires_at TIMESTAMPTZ NOT NULL,

    -- When this record was created
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Reason for revocation (optional)
    revocation_reason VARCHAR(255)
);

-- Index for fast user token lookups (e.g., "revoke all tokens for user")
CREATE INDEX IF NOT EXISTS idx_token_revocations_user_id ON token_revocations(user_id);

-- Index for cleanup of expired tokens
CREATE INDEX IF NOT EXISTS idx_token_revocations_expires_at ON token_revocations(expires_at);

-- Index for checking if a token is revoked
CREATE INDEX IF NOT EXISTS idx_token_revocations_jti_revoked ON token_revocations(jti, revoked_at);

-- Partial index for active (non-revoked) tokens
CREATE INDEX IF NOT EXISTS idx_token_revocations_active ON token_revocations(jti) WHERE revoked_at IS NULL;

COMMENT ON TABLE token_revocations IS 'Tracks refresh token JTIs for revocation support';
COMMENT ON COLUMN token_revocations.jti IS 'Unique JWT ID from the refresh token';
COMMENT ON COLUMN token_revocations.revoked_at IS 'When the token was revoked, NULL if still active';
COMMENT ON COLUMN token_revocations.revocation_reason IS 'Why the token was revoked: logout, refresh, security, admin';
