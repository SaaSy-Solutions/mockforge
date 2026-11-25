//! Red-teaming an agent with a hostile MCP tool server.
//!
//! A [`Suite`] is a server that tries something: a tool description that
//! tells the agent what to do, a tool result that carries an instruction, a
//! tool with a wider reach than the task needs. It holds the task to give the
//! agent, the server as a catalog, and the rules an agent that resists keeps.
//! The mock serves the catalog and records what the agent sends ([`Call`]);
//! [`judge_with_reply`] checks the calls, and the agent's reply when the
//! operator supplies it, against the rules and gives a verdict with what it
//! rests on ([`Report`]).
//!
//! The judge is deterministic, so the same calls and the same reply give the
//! same verdict. It reads nothing else. A suite with a rule about the reply
//! is not run when no reply is given: it cannot pass on the calls alone.
//!
//! MockForge ships suites ([`shipped`]); a team can write its own.

pub mod calls;
pub mod error;
pub mod formats;
pub mod judge;
pub mod suite;

pub use calls::{read_calls, Answer, Call, Client};
pub use error::AgentRedteamError;
pub use formats::{evidence_lines, headline, junit, rule_name, sarif};
pub use judge::{
    judge, judge_with_reply, Finding, FindingKind, Reply, Report, Verdict, REPORT_KIND,
    REPORT_SCHEMA_VERSION,
};
pub use suite::{shipped, shipped_ids, Rule, RuleKind, Suite, SUITE_KIND, SUITE_SCHEMA_VERSION};
