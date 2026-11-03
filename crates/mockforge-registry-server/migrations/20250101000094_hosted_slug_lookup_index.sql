-- Non-unique by design: migration must preserve ambiguous legacy records.
-- Both safe subdomain lookup and global reservation use this expression.
CREATE INDEX idx_hosted_mocks_lower_slug ON hosted_mocks (lower(slug));
