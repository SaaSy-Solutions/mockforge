//! Errors of MCP contract pinning and checking.

/// Why a contract could not be pinned or checked. Every variant is a reason to
/// report "not checked" rather than "unchanged": a server that cannot be read
/// is never treated as a server that did not change.
#[derive(Debug, thiserror::Error)]
pub enum McpContractError {
    /// The endpoint is not an `http`/`https` URL MockForge will talk to.
    #[error("invalid endpoint: {0}")]
    Endpoint(String),

    /// A `--header` value that is not `Name: value`, or has an invalid name.
    #[error("invalid header: {0}")]
    Header(String),

    /// A credential arrived where it would be written to the contract. The
    /// value is not repeated in the message.
    #[error(
        "{what} looks like a credential, and a credential is never written into a contract: \
         pass it with --auth-env \"NAME:ENV_VAR\", which reads the value from the environment \
         at run time, or use an address without it"
    )]
    Credential {
        /// What carried the credential, e.g. "header `Authorization`".
        what: String,
    },

    /// No HTTP response at all: the connection, TLS or a deadline failed.
    #[error("could not reach {url}: {reason}")]
    Transport {
        /// The endpoint that was called.
        url: String,
        /// What failed.
        reason: String,
    },

    /// The endpoint sent a redirect. Redirects are not followed: a contract is
    /// for the address that was approved.
    #[error("{url} redirected to {location}; pin the final address instead")]
    Redirect {
        /// The endpoint that was called.
        url: String,
        /// The `Location` the server sent.
        location: String,
    },

    /// An HTTP error status from a request that must succeed.
    #[error("{method} answered HTTP {status}: {detail}")]
    HttpStatus {
        /// The JSON-RPC method that was sent.
        method: String,
        /// The status code.
        status: u16,
        /// The status text and a short excerpt of the body.
        detail: String,
    },

    /// A JSON-RPC error response from the server.
    #[error("{method} returned an error: {message} (code {code})")]
    Rejected {
        /// The JSON-RPC method that was sent.
        method: String,
        /// The JSON-RPC error code.
        code: i64,
        /// The message the server sent.
        message: String,
    },

    /// The server only speaks protocol revisions this client does not.
    #[error(
        "{url} speaks MCP {supported:?}; this version of MockForge speaks {modern} and \
         the {legacy} handshake"
    )]
    UnsupportedVersion {
        /// The endpoint that was called.
        url: String,
        /// The revisions the server listed.
        supported: Vec<String>,
        /// The per-request revision this client speaks.
        modern: String,
        /// The newest handshake revision this client speaks.
        legacy: String,
    },

    /// The endpoint did not answer as an MCP server.
    #[error("{url} is not an MCP endpoint this version of MockForge can read: {detail}")]
    NotMcp {
        /// The endpoint that was called.
        url: String,
        /// What each attempt received.
        detail: String,
    },

    /// A response body over the size limit.
    #[error("a response from {url} was larger than {limit} bytes")]
    TooLarge {
        /// The endpoint that was called.
        url: String,
        /// The limit that was exceeded.
        limit: usize,
    },

    /// A response that is not shaped like the MCP specification says.
    #[error("malformed response to {method}: {detail}")]
    Protocol {
        /// The JSON-RPC method that was sent.
        method: String,
        /// What was wrong.
        detail: String,
    },

    /// The contract file could not be read or is not a contract.
    #[error("cannot use the contract {path}: {reason}")]
    Contract {
        /// The path as it was given.
        path: String,
        /// What was wrong.
        reason: String,
    },

    /// Reading or writing a local file failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl McpContractError {
    /// Whether the endpoint answered nothing at all, so trying another way of
    /// asking it cannot help.
    pub fn is_transport(&self) -> bool {
        matches!(self, Self::Transport { .. } | Self::TooLarge { .. })
    }
}

/// The result of pinning or checking a contract.
pub type Result<T> = std::result::Result<T, McpContractError>;
