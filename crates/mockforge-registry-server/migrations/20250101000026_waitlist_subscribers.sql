-- Waitlist / beta email subscribers
CREATE TABLE IF NOT EXISTS waitlist_subscribers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'landing_page',
    status TEXT NOT NULL DEFAULT 'subscribed',
    unsubscribe_token UUID NOT NULL DEFAULT gen_random_uuid(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    unsubscribed_at TIMESTAMPTZ
);

-- Unique on email so re-subscribing is idempotent
CREATE UNIQUE INDEX IF NOT EXISTS idx_waitlist_subscribers_email ON waitlist_subscribers (email);
CREATE INDEX IF NOT EXISTS idx_waitlist_subscribers_status ON waitlist_subscribers (status);
