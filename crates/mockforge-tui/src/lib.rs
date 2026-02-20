//! MockForge Terminal UI — a keyboard-driven dashboard for monitoring and
//! controlling a running MockForge instance over its admin HTTP API.

// TUI application crate — relaxed pedantic lints appropriate for an internal app.
#![allow(missing_docs)]
#![allow(clippy::missing_docs_in_private_items)]
#![allow(clippy::new_without_default)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::wildcard_imports)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::match_same_arms)]
#![allow(clippy::if_not_else)]
#![allow(clippy::redundant_closure_for_method_calls)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::unnecessary_wraps)]
#![allow(clippy::unused_self)]
#![allow(clippy::missing_const_for_fn)]
#![allow(clippy::option_if_let_else)]
#![allow(clippy::map_unwrap_or)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::derivable_impls)]
#![allow(clippy::unnecessary_map_or)]
#![allow(clippy::or_fun_call)]
#![allow(clippy::used_underscore_binding)]
#![allow(clippy::unnecessary_literal_bound)]
#![allow(clippy::comparison_chain)]

pub mod api;
pub mod app;
pub mod config;
pub mod event;
pub mod keybindings;
pub mod screens;
pub mod theme;
pub mod tui;
pub mod widgets;

pub use app::App;
