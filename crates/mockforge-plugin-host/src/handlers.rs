//! Request handlers for the plugin-host IPC server.
//!
//! Phase 2 scaffold: only `health` is wired through. The other
//! handlers exist so the dispatch table is complete and the wire
//! protocol is exercised end-to-end — they all return
//! `not_implemented`. Wiring them is the next deliverable.

use std::time::Instant;

use crate::protocol::{Request, Response};

/// Application state carried across requests on a single
/// connection. Cheap to clone — the fields are either `Arc`-backed
/// or scalars.
#[derive(Clone)]
pub struct HandlerContext {
    /// When the host process booted. Used by the health endpoint
    /// so callers can detect a sidecar restart by watching for an
    /// uptime decrease.
    pub started_at: Instant,
}

impl HandlerContext {
    /// Create a context anchored at "now".
    pub fn new() -> Self {
        Self {
            started_at: Instant::now(),
        }
    }

    /// Process uptime in whole seconds.
    pub fn uptime_secs(&self) -> u64 {
        self.started_at.elapsed().as_secs()
    }
}

impl Default for HandlerContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Dispatch a single request to the appropriate handler.
pub async fn handle(ctx: &HandlerContext, request: Request) -> Response {
    match request {
        Request::Health { id } => Response::HealthOk {
            id,
            uptime_secs: ctx.uptime_secs(),
        },
        Request::LoadPlugin { id, .. } => Response::not_implemented(id, "load_plugin"),
        Request::UnloadPlugin { id, .. } => Response::not_implemented(id, "unload_plugin"),
        Request::Invoke { id, .. } => Response::not_implemented(id, "invoke"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[tokio::test]
    async fn health_returns_ok_with_uptime() {
        let ctx = HandlerContext::new();
        let id = Uuid::new_v4();
        let response = handle(&ctx, Request::Health { id }).await;
        match response {
            Response::HealthOk {
                id: echoed,
                uptime_secs,
            } => {
                assert_eq!(echoed, id);
                // Uptime is at-least-zero; we don't assert > 0 because
                // the handler may run faster than 1 second after ctx
                // is created.
                let _ = uptime_secs;
            }
            other => panic!("expected HealthOk, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn load_plugin_returns_not_implemented() {
        let ctx = HandlerContext::new();
        let id = Uuid::new_v4();
        let response = handle(
            &ctx,
            Request::LoadPlugin {
                id,
                plugin_name: "test".into(),
                version: "1.0.0".into(),
                permissions: serde_json::json!({}),
            },
        )
        .await;
        match response {
            Response::Error {
                id: echoed, code, ..
            } => {
                assert_eq!(echoed, id);
                assert_eq!(code, "not_implemented");
            }
            other => panic!("expected Error, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn unload_plugin_returns_not_implemented() {
        let ctx = HandlerContext::new();
        let id = Uuid::new_v4();
        let response = handle(
            &ctx,
            Request::UnloadPlugin {
                id,
                plugin_name: "test".into(),
            },
        )
        .await;
        assert!(matches!(response, Response::Error { code, .. } if code == "not_implemented"));
    }

    #[tokio::test]
    async fn invoke_returns_not_implemented() {
        let ctx = HandlerContext::new();
        let id = Uuid::new_v4();
        let response = handle(
            &ctx,
            Request::Invoke {
                id,
                plugin_name: "test".into(),
                function: "on_request".into(),
                input: vec![],
            },
        )
        .await;
        assert!(matches!(response, Response::Error { code, .. } if code == "not_implemented"));
    }
}
