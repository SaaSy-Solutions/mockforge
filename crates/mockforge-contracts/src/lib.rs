// Pedantic/nursery lints inherited from workspace; allow the most common stylistic ones
// since this code was extracted from mockforge-core which has the same lints.
#![allow(
    clippy::uninlined_format_args,
    clippy::must_use_candidate,
    clippy::missing_const_for_fn,
    clippy::doc_markdown,
    clippy::return_self_not_must_use,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_lossless,
    clippy::cast_sign_loss,
    clippy::use_self,
    clippy::unused_self,
    clippy::unused_async,
    clippy::map_unwrap_or,
    clippy::format_push_string,
    clippy::module_name_repetitions,
    clippy::redundant_closure_for_method_calls,
    clippy::option_if_let_else,
    clippy::items_after_statements,
    clippy::significant_drop_tightening,
    clippy::future_not_send,
    clippy::cognitive_complexity,
    clippy::too_many_lines,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::similar_names,
    clippy::needless_pass_by_value,
    clippy::implicit_hasher,
    clippy::struct_excessive_bools,
    clippy::if_not_else,
    clippy::match_wildcard_for_single_variants,
    clippy::float_cmp
)]

//! Contract testing, drift detection, and incident management for MockForge
//!
//! This crate contains the independently extractable contract-related modules
//! from `mockforge-core`, including:
//!
//! - **consumer_contracts**: Consumer-driven contract tracking and violation detection
//! - **contract_validation**: Contract validation types and CI/CD pipeline integration
//! - **incidents**: Incident management with Jira/Slack integrations
//! - **contract_drift**: Drift detection types, consumer mapping, fitness functions, and forecasting
//! - **diff_types**: Core diff analysis types shared across contract modules
//! - **protocol**: Protocol type enumeration

pub mod consumer_contracts;
pub mod contract_drift;
pub mod contract_validation;
pub mod diff_types;
pub mod error;
pub mod incidents;
pub mod protocol;
pub mod schema_diff;

pub use error::{ContractError, Result};
