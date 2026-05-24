//! AI response handler — re-export shim.
//!
//! Issue #656 (post-#555 follow-up) moved this module to
//! [`mockforge_intelligence::ai_handler`]. The original
//! `mockforge_core::{Result, Error}` imports were already re-exports of
//! `mockforge_foundation` equivalents, and `mockforge-data` was already
//! a direct dep of `mockforge-intelligence` — so the move stayed
//! cycle-safe with the Issue #562 core↔intelligence cycle-break.
//!
//! This shim keeps existing
//! `mockforge_http::ai_handler::{AiResponseHandler, AiResponseConfig,
//! process_response_with_ai, create_ai_handler}` callers (notably the
//! top-level `mockforge_http::{AiResponseConfig, AiResponseHandler,
//! process_response_with_ai}` re-exports in `lib.rs`) resolving
//! unchanged. Future drains may drop this shim; until then, prefer
//! importing from `mockforge_intelligence::ai_handler` directly in new
//! code.

pub use mockforge_intelligence::ai_handler::*;
