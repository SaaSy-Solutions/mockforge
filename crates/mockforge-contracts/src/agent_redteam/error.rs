//! Errors of red-teaming an agent.

use std::fmt;

/// Why a suite cannot be read, a call log cannot be read, or a shipped suite
/// cannot be found. Each is a reason to report "not run" rather than "pass":
/// a lab that cannot be read has tested nothing.
#[derive(thiserror::Error)]
pub enum AgentRedteamError {
    /// The file was read and is not a suite that can be served or judged.
    #[error("{path}: {reason}")]
    Suite {
        /// The file that was named.
        path: String,
        /// What is wrong with it.
        reason: String,
    },

    /// The call log has a line that is not a call record.
    #[error("{path}: line {line}: {reason}")]
    Calls {
        /// The call log that was named.
        path: String,
        /// The line that is not a call record, counted from 1.
        line: usize,
        /// What is wrong with it.
        reason: String,
    },

    /// No shipped suite has the id that was named.
    #[error("no shipped suite has the id `{0}`; the shipped suites are {1}")]
    NoSuchSuite(String, String),
}

/// Printed as the reason. A program that returns the error, as `mockforge`
/// does, then prints a sentence and not a structure.
impl fmt::Debug for AgentRedteamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

/// The result of reading a suite or a call log.
pub type Result<T> = std::result::Result<T, AgentRedteamError>;

pub(super) fn invalid_suite(path: &str, reason: &str) -> AgentRedteamError {
    AgentRedteamError::Suite {
        path: path.to_string(),
        reason: reason.trim().to_string(),
    }
}

pub(super) fn bad_call(path: &str, line: usize, reason: &str) -> AgentRedteamError {
    AgentRedteamError::Calls {
        path: path.to_string(),
        line,
        reason: reason.trim().to_string(),
    }
}
