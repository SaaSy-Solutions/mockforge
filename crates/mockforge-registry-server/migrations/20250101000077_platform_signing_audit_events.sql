-- Platform signing-root rotation audit events (Issue #550, RFC §8.2 / §9).
--
-- New enum values consumed by the registry whenever the operator
-- (SaaSy Solutions, not an org admin) drives the platform signing-key
-- rotation procedure. These rows are written by code in
-- `mockforge-platform-signing` via the existing `record_audit_event`
-- helper.
--
-- The Rust mirror lives in
-- `crates/mockforge-registry-core/src/models/audit_log.rs`
-- (`AuditEventType::PlatformSigning*`). Round-trip test in that file
-- guarantees every snake_case literal here also exists on the Rust
-- side.

-- Operator started a key handover. Payload (in `metadata`) includes
-- `from_key_id`, `to_key_id`, `transition_until`, `algorithm`. The
-- handover signature itself is NOT stored — it's published on the
-- rotation-event endpoint that plugin-hosts poll.
ALTER TYPE audit_event_type ADD VALUE IF NOT EXISTS 'platform_signing_rotation_started';

-- Old key formally retired after the transition window. Operator
-- runs `aws kms disable-key` manually; this row records that the
-- registry observed the retirement and updated in-memory state.
ALTER TYPE audit_event_type ADD VALUE IF NOT EXISTS 'platform_signing_key_retired';

-- Emergency revocation — old key destroyed without a successor in
-- place. Used when the active key is believed compromised. Operator
-- must follow up with `platform_signing_rotation_started` once a new
-- key is provisioned (see runbook).
ALTER TYPE audit_event_type ADD VALUE IF NOT EXISTS 'platform_signing_key_revoked';
