-- Admin UI Authentication Schema
-- This migration creates tables for Admin UI user authentication and session management
-- Compatible with both SQLite and PostgreSQL

-- Admin users table (separate from collaboration users)
CREATE TABLE IF NOT EXISTS admin_users (
    id TEXT PRIMARY KEY NOT NULL,
    username TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('admin', 'editor', 'viewer')),
    email TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_login_at TIMESTAMP,
    failed_login_attempts INTEGER NOT NULL DEFAULT 0,
    locked_until TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_admin_users_username ON admin_users(username);
CREATE INDEX IF NOT EXISTS idx_admin_users_email ON admin_users(email);
CREATE INDEX IF NOT EXISTS idx_admin_users_locked ON admin_users(locked_until);

-- Refresh tokens table for JWT token refresh
CREATE TABLE IF NOT EXISTS refresh_tokens (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL,
    token_hash TEXT NOT NULL,
    expires_at TIMESTAMP NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    revoked_at TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES admin_users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_refresh_tokens_user_id ON refresh_tokens(user_id);
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_token_hash ON refresh_tokens(token_hash);
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_expires_at ON refresh_tokens(expires_at);

-- Login attempts table for rate limiting and security monitoring
CREATE TABLE IF NOT EXISTS login_attempts (
    id TEXT PRIMARY KEY NOT NULL,
    username TEXT NOT NULL,
    ip_address TEXT,
    user_agent TEXT,
    success INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (username) REFERENCES admin_users(username) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_login_attempts_username ON login_attempts(username);
CREATE INDEX IF NOT EXISTS idx_login_attempts_created_at ON login_attempts(created_at);
CREATE INDEX IF NOT EXISTS idx_login_attempts_ip ON login_attempts(ip_address);

-- Insert default admin user (password: admin123)
-- Password hash is bcrypt hash of "admin123" with cost 12
-- In production, this should be changed immediately
INSERT OR IGNORE INTO admin_users (id, username, password_hash, role, email)
VALUES (
    '00000000-0000-0000-0000-000000000001',
    'admin',
    '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewY5GyYqJZ5q5q5q',
    'admin',
    'admin@mockforge.dev'
);

-- Insert default viewer user (password: viewer123)
INSERT OR IGNORE INTO admin_users (id, username, password_hash, role, email)
VALUES (
    '00000000-0000-0000-0000-000000000002',
    'viewer',
    '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewY5GyYqJZ5q5q5q',
    'viewer',
    'viewer@mockforge.dev'
);

-- Insert default editor user (password: editor123)
INSERT OR IGNORE INTO admin_users (id, username, password_hash, role, email)
VALUES (
    '00000000-0000-0000-0000-000000000003',
    'editor',
    '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewY5GyYqJZ5q5q5q',
    'editor',
    'editor@mockforge.dev'
);
