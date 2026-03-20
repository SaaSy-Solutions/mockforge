//! # MockForge Security Core
//!
//! Security and encryption primitives for MockForge.
//!
//! This crate provides:
//! - **Security**: Access review, change management, compliance, privileged access,
//!   risk assessment, SIEM integration, and observability
//! - **Encryption**: Crypto algorithms (AES-GCM, ChaCha20-Poly1305), key management,
//!   key derivation (Argon2, PBKDF2), key rotation, and auto-encryption policies

#![allow(deprecated)]
#![allow(missing_docs)]
// Allow pedantic/nursery clippy lints inherited from workspace config.
// These are pre-existing in the code moved from mockforge-core.
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::significant_drop_tightening)]
#![allow(clippy::unnecessary_wraps)]
#![allow(clippy::unused_self)]
#![allow(clippy::match_same_arms)]
#![allow(clippy::items_after_statements)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::redundant_closure_for_method_calls)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::missing_const_for_fn)]
#![allow(clippy::redundant_clone)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::option_if_let_else)]
#![allow(clippy::match_wildcard_for_single_variants)]
#![allow(clippy::redundant_else)]
#![allow(clippy::let_and_return)]
#![allow(clippy::needless_continue)]
#![allow(clippy::struct_excessive_bools)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_lossless)]
#![allow(clippy::assigning_clones)]
#![allow(clippy::manual_let_else)]
#![allow(clippy::map_unwrap_or)]
#![allow(clippy::unused_async)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::single_char_pattern)]
#![allow(clippy::similar_names)]
#![allow(clippy::wildcard_imports)]
#![allow(clippy::use_self)]
#![allow(clippy::cognitive_complexity)]
#![allow(clippy::if_not_else)]
#![allow(clippy::to_string_trait_impl)]
#![allow(clippy::manual_midpoint)]
#![allow(clippy::needless_borrowed_reference)]
#![allow(clippy::match_bool)]
#![allow(clippy::single_match_else)]
#![allow(clippy::doc_link_with_quotes)]
#![allow(clippy::semicolon_if_nothing_returned)]
#![allow(clippy::str_to_string)]
#![allow(clippy::needless_borrows_for_generic_args)]
#![allow(clippy::branches_sharing_code)]
#![allow(clippy::string_lit_as_bytes)]
#![allow(clippy::manual_string_new)]
#![allow(clippy::map_identity)]
#![allow(clippy::ref_option)]
#![allow(clippy::non_std_lazy_statics)]
#![allow(clippy::inefficient_to_string)]
#![allow(clippy::needless_pass_by_ref_mut)]
#![allow(clippy::single_option_map)]
#![allow(unexpected_cfgs)]

pub mod encryption;
pub mod error;
pub mod security;

pub use error::{Error, Result};
