//! RAG-based AI generator — re-export shim.
//!
//! Issue #555 phase 10 moved this module to
//! [`mockforge_intelligence::rag_ai_generator`]. Both foreign deps were
//! originally `mockforge-core` re-exports (`AiResponseConfig`,
//! `AiGenerator` trait), but they have since been promoted to
//! `mockforge-foundation::ai_response` and `mockforge-openapi::response`
//! — so the move stays cycle-safe with the Issue #562 cycle-break.
//!
//! This shim keeps existing
//! `mockforge_http::rag_ai_generator::RagAiGenerator` callers resolving
//! unchanged. Future phases of #555 may drop this shim; until then,
//! prefer importing from `mockforge_intelligence::rag_ai_generator`
//! directly in new code.

pub use mockforge_intelligence::rag_ai_generator::*;
