//! Cloud AI Studio surface.
//!
//! Routes every cloud LLM request (chat, generate-openapi, learn, rules)
//! through one provider-selection + quota-check pipeline so the billing,
//! BYOK, and platform-key paths stay consistent across handlers.
//!
//! See `docs/cloud/CLOUD_AI_STUDIO_DESIGN.md` for the full design.

pub mod client;
pub mod contract_diff;
pub mod provider;
pub mod quota;

pub use client::{call_llm, LlmCall, LlmResult};
pub use provider::{pick_provider, Provider, ProviderSelection};
pub use quota::{check_ai_quota, record_ai_usage, QuotaCheck};
