-- Two-Factor Authentication (2FA) support
-- Adds TOTP (Time-based One-Time Password) support for users

-- Add 2FA fields to users table
ALTER TABLE users
    ADD COLUMN IF NOT EXISTS two_factor_enabled BOOLEAN DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS two_factor_secret VARCHAR(255), -- Base32-encoded TOTP secret
    ADD COLUMN IF NOT EXISTS two_factor_backup_codes TEXT[], -- Array of hashed backup codes
    ADD COLUMN IF NOT EXISTS two_factor_verified_at TIMESTAMPTZ; -- When 2FA was last verified

-- Create index for 2FA-enabled users
CREATE INDEX IF NOT EXISTS idx_users_2fa_enabled ON users(two_factor_enabled) WHERE two_factor_enabled = TRUE;

-- Add comment for documentation
COMMENT ON COLUMN users.two_factor_secret IS 'Base32-encoded TOTP secret (encrypted at rest)';
COMMENT ON COLUMN users.two_factor_backup_codes IS 'Array of bcrypt-hashed backup codes for account recovery';
