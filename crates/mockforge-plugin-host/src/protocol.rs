//! Wire protocol for main-mockforge ↔ plugin-host IPC.
//!
//! Newline-delimited JSON. Every request gets exactly one response,
//! tagged by `id` so callers can multiplex without ordering
//! guarantees. Chosen over protobuf for v1 because: (a) the message
//! set is tiny, (b) JSON is debuggable with `nc -U`, (c) we can
//! swap to protobuf later without changing semantics — only the
//! framing.
//!
//! Compared with `wasi:http` for plugin↔host (the build-vs-buy spike
//! recommendation): that's the *plugin's* interface inside the
//! host, exposed via Wasmtime imports. This protocol is the **host
//! ↔ host** interface — between mockforge's request handler and
//! the sidecar's plugin runtime — and is unrelated.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Request from main mockforge to the plugin host.
///
/// Each request carries a client-chosen `id` that the matching
/// [`Response`] echoes. IDs need only be unique within a single
/// connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Request {
    /// Liveness check. Cheap, no side effects. Used by both the
    /// startup probe and the steady-state heartbeat.
    Health {
        /// Correlation id — echoed back in the response so callers
        /// can multiplex requests over a single connection.
        id: Uuid,
    },
    /// Load a WASM plugin into a fresh `Store`.
    ///
    /// In v1 the WASM bytes are inlined as base64 in the request.
    /// The follow-up "registry fetch" path will replace this with a
    /// `download_url` field that the host fetches itself, so
    /// signature verification can happen before any bytes touch
    /// the loader. Until then, the caller is expected to verify
    /// signatures upstream.
    LoadPlugin {
        /// Correlation id.
        id: Uuid,
        /// Plugin name from the registry.
        plugin_name: String,
        /// Plugin version, pinned at attach time. Parsed via
        /// `PluginVersion::parse` (semver-shaped:
        /// `major.minor.patch`).
        version: String,
        /// Permission grant payload (RFC §4.2). Stored alongside
        /// the loaded plugin; runtime enforcement of the
        /// `manifest ∩ grant` invariant lands with the egress
        /// proxy + env-grant integration.
        permissions: serde_json::Value,
        /// Base64-encoded WASM module bytes.
        wasm_b64: String,
    },
    /// Detach a plugin and free its sandbox.
    UnloadPlugin {
        /// Correlation id.
        id: Uuid,
        /// Plugin name from the registry.
        plugin_name: String,
    },
    /// Invoke a plugin function on a request/response pair.
    Invoke {
        /// Correlation id.
        id: Uuid,
        /// Plugin name from the registry.
        plugin_name: String,
        /// Exported function to call (e.g. `on_request`,
        /// `on_response`).
        function: String,
        /// Opaque input bytes — the loader handles serialization
        /// of the host context + input together.
        input: Vec<u8>,
    },
}

impl Request {
    /// The correlation id this request was tagged with. The
    /// matching [`Response`] echoes it.
    pub fn id(&self) -> Uuid {
        match self {
            Request::Health { id }
            | Request::LoadPlugin { id, .. }
            | Request::UnloadPlugin { id, .. }
            | Request::Invoke { id, .. } => *id,
        }
    }
}

/// Response from the plugin host back to main mockforge.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Response {
    /// Health check answered. Returns the host's startup time so
    /// callers can detect a restart.
    HealthOk {
        /// Echoed correlation id from the matching request.
        id: Uuid,
        /// Process uptime in seconds.
        uptime_secs: u64,
    },
    /// Plugin operation succeeded. Body is operation-specific.
    Ok {
        /// Echoed correlation id from the matching request.
        id: Uuid,
        /// Operation-specific result payload.
        body: serde_json::Value,
    },
    /// Operation failed. `code` is a stable string callers can
    /// match against; `message` is human-readable detail.
    Error {
        /// Echoed correlation id from the matching request.
        id: Uuid,
        /// Stable machine-readable error code.
        code: String,
        /// Human-readable detail.
        message: String,
    },
}

impl Response {
    /// Convenience: build a `not_implemented` error for handlers
    /// that haven't shipped yet (load/unload/invoke).
    pub fn not_implemented(id: Uuid, what: &str) -> Self {
        Response::Error {
            id,
            code: "not_implemented".to_string(),
            message: format!("{} is not yet implemented in this plugin-host scaffold", what),
        }
    }

    /// Correlation id this response is tagged with.
    pub fn id(&self) -> Uuid {
        match self {
            Response::HealthOk { id, .. }
            | Response::Ok { id, .. }
            | Response::Error { id, .. } => *id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_round_trips_through_json() {
        let id = Uuid::new_v4();
        let req = Request::Health { id };
        let bytes = serde_json::to_vec(&req).unwrap();
        let parsed: Request = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(parsed.id(), id);
    }

    #[test]
    fn load_plugin_request_carries_permissions_and_wasm() {
        let req = Request::LoadPlugin {
            id: Uuid::new_v4(),
            plugin_name: "stripe-rewriter".into(),
            version: "1.2.3".into(),
            permissions: serde_json::json!({
                "egress": { "allow": ["*.stripe.com"] }
            }),
            wasm_b64: "AGFzbQEAAAA=".into(), // empty WASM module
        };
        let bytes = serde_json::to_vec(&req).unwrap();
        let parsed: Request = serde_json::from_slice(&bytes).unwrap();
        match parsed {
            Request::LoadPlugin {
                plugin_name,
                version,
                permissions,
                wasm_b64,
                ..
            } => {
                assert_eq!(plugin_name, "stripe-rewriter");
                assert_eq!(version, "1.2.3");
                assert!(permissions.get("egress").is_some());
                assert_eq!(wasm_b64, "AGFzbQEAAAA=");
            }
            other => panic!("expected LoadPlugin, got {:?}", other),
        }
    }

    #[test]
    fn not_implemented_helper_carries_correlation_id() {
        let id = Uuid::new_v4();
        let resp = Response::not_implemented(id, "invoke");
        match resp {
            Response::Error {
                id: echoed,
                code,
                message,
            } => {
                assert_eq!(echoed, id);
                assert_eq!(code, "not_implemented");
                assert!(message.contains("invoke"));
            }
            other => panic!("expected Error, got {:?}", other),
        }
    }

    #[test]
    fn response_serialization_uses_snake_case_kind() {
        let resp = Response::HealthOk {
            id: Uuid::new_v4(),
            uptime_secs: 42,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"kind\":\"health_ok\""));
    }

    #[test]
    fn unknown_request_kind_fails_to_parse() {
        // Forward compatibility check: requests with new kinds get
        // a clean parse error rather than silently mismatching to a
        // similar variant.
        let bytes = br#"{"kind":"future_op","id":"00000000-0000-0000-0000-000000000000"}"#;
        let result: Result<Request, _> = serde_json::from_slice(bytes);
        assert!(result.is_err());
    }
}
