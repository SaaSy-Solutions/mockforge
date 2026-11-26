//! Errors of a range: a pack that cannot be read or served, an exchange log
//! that cannot be read, or a shipped pack that cannot be found.

use std::fmt;

/// Why a pack cannot be read, an exchange log cannot be read, or a shipped
/// pack cannot be found. A pack or a log that cannot be read has tested
/// nothing, so it is refused rather than judged.
#[derive(thiserror::Error)]
pub enum RangeError {
    /// The file was read and is not a pack that can be served.
    #[error("{path}: {reason}")]
    Pack {
        /// The file that was named.
        path: String,
        /// What is wrong with it.
        reason: String,
    },

    /// The exchange log has a line that is not an exchange.
    #[error("{path}: line {line}: {reason}")]
    Exchanges {
        /// The exchange log that was named.
        path: String,
        /// The line that is not an exchange, counted from 1.
        line: usize,
        /// What is wrong with it.
        reason: String,
    },

    /// No shipped pack has the id that was named.
    #[error("no shipped pack has the id `{0}`; the shipped packs are {1}")]
    NoSuchPack(String, String),
}

/// Printed as the reason. A program that returns the error, as `mockforge`
/// does, then prints a sentence and not a structure.
impl fmt::Debug for RangeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

/// The result of reading a pack or an exchange log.
pub type Result<T> = std::result::Result<T, RangeError>;

pub(super) fn invalid_pack(path: &str, reason: &str) -> RangeError {
    RangeError::Pack {
        path: path.to_string(),
        reason: reason.trim().to_string(),
    }
}

pub(super) fn bad_exchange(path: &str, line: usize, reason: &str) -> RangeError {
    RangeError::Exchanges {
        path: path.to_string(),
        line,
        reason: reason.trim().to_string(),
    }
}
