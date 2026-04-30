-- Invitation lifecycle audit-event variants.
--
-- Organization invitations have always been recordable (create / revoke /
-- accept), but the audit_event_type enum was never extended to cover them.
-- Without these values, any record_audit_event() call from the invitation
-- handlers fails the enum binding and the audit row is silently dropped.
--
-- ALTER TYPE ADD VALUE is the established pattern in this codebase
-- (see 20250101000055_publisher_keys_org_scope.sql).
ALTER TYPE audit_event_type ADD VALUE IF NOT EXISTS 'invitation_created';
ALTER TYPE audit_event_type ADD VALUE IF NOT EXISTS 'invitation_revoked';
ALTER TYPE audit_event_type ADD VALUE IF NOT EXISTS 'invitation_accepted';
