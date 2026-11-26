//! Errors of a range: a pack that cannot be read or served, or a shipped
//! pack that cannot be found.

use std::fmt;

/// Why a pack cannot be read, or a shipped pack cannot be found. A pack that
/// cannot be read has tested nothing, so it is refused rather than run.
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

/// The result of reading a pack.
pub type Result<T> = std::result::Result<T, RangeError>;

pub(super) fn invalid_pack(path: &str, reason: &str) -> RangeError {
    RangeError::Pack {
        path: path.to_string(),
        reason: reason.trim().to_string(),
    }
}
