//! Red-teaming an agent with a hostile MCP tool server.
//!
//! A [`Suite`] is a server that tries something: a tool description that
//! tells the agent what to do, a tool result that carries an instruction, a
//! tool with a wider reach than the task needs. It holds the task to give the
//! agent, the server as a catalog, and the rules an agent that resists keeps.
//! The mock serves the catalog and records what the agent sends ([`Call`]);
//! [`judge`] checks the calls against the rules and gives a verdict with what
//! it rests on ([`Report`]).
//!
//! The judge is deterministic and reads only the calls, so the same calls give
//! the same verdict. What the agent *said* to the user is not judged.
//!
//! MockForge ships suites ([`shipped`]); a team can write its own.

pub mod calls;
pub mod error;
pub mod judge;
pub mod suite;

pub use calls::{read_calls, Call, Client};
pub use error::AgentRedteamError;
pub use judge::{judge, Finding, FindingKind, Report, Verdict, REPORT_KIND, REPORT_SCHEMA_VERSION};
pub use suite::{shipped, shipped_ids, Rule, RuleKind, Suite, SUITE_KIND, SUITE_SCHEMA_VERSION};
