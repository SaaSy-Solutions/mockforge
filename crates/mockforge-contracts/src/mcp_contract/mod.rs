//! MCP contract pinning: record what a tool server declares when it is
//! approved, and report every change since.
//!
//! [`model::Contract`] is the pinned document, [`client::fetch_declared`] asks
//! a server what it declares, and [`report::compare`] turns the difference
//! into findings.
//!
//! A contract pins what a server *says*: its instructions, and each tool's
//! description, schemas and annotations. It cannot see what a server does
//! behind its own boundary. It is only comparable with the same client
//! identity and the same credentials that produced it, because a server may
//! answer different callers differently.

pub mod client;
pub mod diff;
pub mod error;
pub mod model;
pub mod report;
pub mod signals;

pub use client::{ClientOptions, Declared, MAX_BODY_BYTES, MAX_PAGES};
pub use diff::{Change, Delta};
pub use error::{McpContractError, Result};
pub use model::{
    ClientIdentity, Contract, EnvHeader, Era, ServerContract, SignalSite, ToolContract,
    CONTRACT_KIND, CONTRACT_SCHEMA_VERSION,
};
pub use report::{
    compare, CheckContext, CheckReport, Finding, FindingKind, Outcome, Severity, Summary,
    REPORT_KIND, REPORT_SCHEMA_VERSION,
};
pub use signals::{Signal, SignalKind};
