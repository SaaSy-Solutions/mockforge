-- OAuth provider support for users
-- Adds fields to support GitHub and Google OAuth authentication

-- Add OAuth provider fields to users table
ALTER TABLE users
    ADD COLUMN IF NOT EXISTS github_id VARCHAR(255) UNIQUE,
    ADD COLUMN IF NOT EXISTS google_id VARCHAR(255) UNIQUE,
    ADD COLUMN IF NOT EXISTS auth_provider VARCHAR(50), -- 'github', 'google', 'email'
    ADD COLUMN IF NOT EXISTS avatar_url TEXT;

-- Create index for OAuth lookups
CREATE INDEX IF NOT EXISTS idx_users_github_id ON users(github_id) WHERE github_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_users_google_id ON users(google_id) WHERE google_id IS NOT NULL;

-- Update existing users to have auth_provider = 'email'
UPDATE users SET auth_provider = 'email' WHERE auth_provider IS NULL;
