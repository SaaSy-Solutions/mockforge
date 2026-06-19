-- New audit event types wired in PR 2 of the SOC2 audit-emit sprint.
--
-- The Rust mirror lives in
-- `crates/mockforge-registry-core/src/models/audit_log.rs`
-- (`AuditEventType::{AiUsage, PaymentFailed}`). The round-trip test in that
-- file guarantees every snake_case literal below also exists on the Rust side.
--
-- ADD VALUE is safe inside this migration's transaction because none of these
-- values are *used* (referenced as a literal cast to the enum) in the same
-- migration — same pattern as `20250101000080_audit_log_integrity.sql`.

-- AI usage (#866). Emitted from the shared metered LLM path
-- (`handlers::ai_studio::run_completion_for_org`) and the test-generation
-- worker after a successful `record_ai_usage`. Payload (in `metadata`)
-- includes the handler/endpoint name, provider, and prompt/completion/total
-- token counts for SOC2 cost-attribution and abuse forensics.
ALTER TYPE audit_event_type ADD VALUE IF NOT EXISTS 'ai_usage';

-- Payment failure (#873). The Stripe `invoice.payment_failed` webhook
-- previously reused `billing_canceled`, conflating a past-due dunning state
-- with a real cancellation. This dedicated value is emitted from
-- `handlers::billing::handle_payment_failed`.
ALTER TYPE audit_event_type ADD VALUE IF NOT EXISTS 'payment_failed';
