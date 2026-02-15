-- Login attempt tracking for rate limiting

CREATE TABLE login_attempts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) NOT NULL,
    ip_address VARCHAR(45), -- IPv6 max length
    success BOOLEAN NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_login_attempts_email ON login_attempts(email, created_at DESC);
CREATE INDEX idx_login_attempts_ip ON login_attempts(ip_address, created_at DESC);
CREATE INDEX idx_login_attempts_created ON login_attempts(created_at);

-- Cleanup old attempts (older than 30 days) - can be run periodically
-- DELETE FROM login_attempts WHERE created_at < NOW() - INTERVAL '30 days';
