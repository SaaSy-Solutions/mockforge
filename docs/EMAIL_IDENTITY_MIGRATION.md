# Email identity migration

Migration `20250101000101` makes trimmed, case-insensitive email addresses one
account identity. A unique database index covers registration, OAuth, SSO,
profile updates and direct inserts, including competing transactions. New
writes store the canonical address. Login and password-reset lookups use that
same identity, and login lockout counts combine case variants.

Before rollout, inspect the target database for legacy collision groups:

```sql
SELECT COUNT(*) AS collision_groups
FROM (
    SELECT lower(btrim(email))
    FROM users
    GROUP BY lower(btrim(email))
    HAVING COUNT(*) > 1
) AS collisions;
```

The migration aborts if any group exists. It never picks an account, merges
users, moves organization membership or deletes records. An operator must
review ownership and each account's tenant relationships before proposing a
repair. Record the approved repair and take a database backup before applying
it; rerun the count and migration afterward. Source and fixture tests do not
establish that a deployed database has no collisions.

Changing the canonical address clears `is_verified`; trimming or changing only
letter case preserves it. The user must verify ownership of the new address
through the existing email-verification flow.

OAuth checks provider-owned email proof before email-based lookup, linking or
account creation. GitHub uses the authenticated `/user/emails` list's primary
verified address, including private addresses; public profile email is not proof.
Google requires `verified_email: true`. Missing, invalid, unverified or ambiguous
proof fails before linking. Existing GitHub scope `user:email` supplies access.

Linking requires the existing account to be verified, and an atomic database
update prevents replacing a different already-linked provider identity. Provider
transport controls and uniqueness do not establish live email delivery, session
expiry, legacy pre-registration/password ownership, or reauthentication. Those
retain separate acceptance checks in #908.
