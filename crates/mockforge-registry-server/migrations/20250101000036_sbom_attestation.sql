-- Publisher attestation of SBOM contents.
--
-- Binding the SBOM to an artifact by checksum (migration 20250101000034)
-- catches "this SBOM is about the wrong WASM" but doesn't prove the *author*
-- of the SBOM is the publisher. This migration adds the primitives for an
-- attestation chain: publishers register an Ed25519 public key, and at
-- publish time they can submit a detached signature over the
-- `(checksum || sbom_digest)` pair. Verification happens server-side and
-- the result is recorded on the version row so the scanner can surface it
-- as a positive finding.
--
-- Deliberately narrower than Sigstore / in-toto: no certificate chain, no
-- rekor transparency log, no key rotation UX. Each user can hold multiple
-- keys (re-issuance, multi-device) and any registered key is accepted.
-- When we need a stronger chain the `user_public_keys` table is where the
-- issuer metadata will land.

CREATE TABLE IF NOT EXISTS user_public_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- Curve/algorithm. Only "ed25519" is supported today, but the column
    -- is a free-form tag so additional algorithms can land without a
    -- migration.
    algorithm VARCHAR(32) NOT NULL DEFAULT 'ed25519',
    -- Raw 32-byte Ed25519 public key, base64url-encoded for
    -- transport-friendly storage.
    public_key_b64 VARCHAR(128) NOT NULL,
    -- Human-readable label so users can distinguish keys ("laptop",
    -- "CI-2025"), not cryptographically meaningful.
    label VARCHAR(128) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    revoked_at TIMESTAMPTZ,

    CONSTRAINT user_public_keys_alg_values CHECK (algorithm IN ('ed25519'))
);

CREATE INDEX IF NOT EXISTS user_public_keys_user_idx
    ON user_public_keys(user_id)
    WHERE revoked_at IS NULL;

-- One signed attestation per version. `signed_key_id` points at the
-- `user_public_keys` row that verified the signature; `signed_at` is set
-- at publish time. Both null when the publisher didn't submit a signature.
ALTER TABLE plugin_versions
    ADD COLUMN IF NOT EXISTS sbom_signed_key_id UUID
        REFERENCES user_public_keys(id) ON DELETE SET NULL;

ALTER TABLE plugin_versions
    ADD COLUMN IF NOT EXISTS sbom_signed_at TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS plugin_versions_sbom_signed_idx
    ON plugin_versions(sbom_signed_key_id)
    WHERE sbom_signed_key_id IS NOT NULL;
