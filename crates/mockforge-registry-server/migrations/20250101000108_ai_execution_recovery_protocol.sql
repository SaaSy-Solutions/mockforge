-- Legacy pending calls lack a cooperating execution fence and cannot be
-- recovered online. New tasks acquire the fence before contacting providers.
ALTER TABLE ai_token_reservations ADD COLUMN execution_fenced BOOLEAN NOT NULL DEFAULT FALSE;
