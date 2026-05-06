//! Request handlers for the plugin-host IPC server.
//!
//! Each [`Request`] variant routes to a method on [`Host`]; errors
//! are caught and translated to [`Response::Error`] with a stable
//! `code` from [`crate::host::HostError::code`].

use base64::Engine;

use crate::host::{Host, HostError};
use crate::protocol::{Request, Response};

/// Dispatch a single request to the appropriate handler.
pub async fn handle(host: &Host, request: Request) -> Response {
    match request {
        Request::Health { id } => Response::HealthOk {
            id,
            uptime_secs: host.uptime_secs(),
        },

        Request::LoadPlugin {
            id,
            plugin_name,
            version,
            permissions,
            wasm_b64,
            signature_b64,
            publisher_key_id,
        } => {
            let bytes = match base64::engine::general_purpose::STANDARD.decode(wasm_b64.as_bytes())
            {
                Ok(b) => b,
                Err(err) => {
                    return Response::Error {
                        id,
                        code: "invalid_base64".to_string(),
                        message: format!("decoding wasm_b64: {err}"),
                    };
                }
            };
            match host
                .load_plugin(
                    &plugin_name,
                    &version,
                    permissions,
                    bytes,
                    signature_b64,
                    publisher_key_id,
                )
                .await
            {
                Ok(plugin_id) => Response::Ok {
                    id,
                    body: serde_json::json!({
                        "plugin_id": plugin_id.to_string(),
                        "plugin_name": plugin_name,
                        "version": version,
                    }),
                },
                Err(err) => host_error_to_response(id, err),
            }
        }

        Request::UnloadPlugin { id, plugin_name } => match host.unload_plugin(&plugin_name).await {
            Ok(was_loaded) => Response::Ok {
                id,
                body: serde_json::json!({
                    "plugin_name": plugin_name,
                    // `false` means it wasn't loaded — idempotent
                    // detach. Callers may treat this as either
                    // success or "already gone".
                    "detached": was_loaded,
                }),
            },
            Err(err) => host_error_to_response(id, err),
        },

        Request::Invoke {
            id,
            plugin_name,
            function,
            input,
        } => match host.invoke(&plugin_name, &function, input).await {
            Ok(value) => Response::Ok {
                id,
                body: serde_json::json!({
                    "plugin_name": plugin_name,
                    "function": function,
                    "result": value,
                }),
            },
            Err(err) => host_error_to_response(id, err),
        },
    }
}

fn host_error_to_response(id: uuid::Uuid, err: HostError) -> Response {
    let code = err.code().to_string();
    Response::Error {
        id,
        code,
        message: err.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockforge_plugin_loader::PluginLoaderConfig;
    use uuid::Uuid;

    /// Smallest valid WASM module bytes — `\0asm` + version 1.
    const MINIMAL_WASM: &[u8] = &[0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];

    fn b64_minimal() -> String {
        base64::engine::general_purpose::STANDARD.encode(MINIMAL_WASM)
    }

    /// Drive `body` and the actor concurrently on a current-thread
    /// runtime — same pattern used in `host::tests`. Tests use this
    /// instead of `#[tokio::test]` because the actor future is
    /// `!Send` and can't be `tokio::spawn`'d.
    fn run_with_actor<F, T>(body: impl FnOnce(Host) -> F) -> T
    where
        F: std::future::Future<Output = T>,
    {
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        rt.block_on(async move {
            let verifier = crate::signing::SignatureVerifier::new(
                crate::signing::TrustStore::new(),
                crate::signing::SignatureMode::Optional,
            );
            let (host, actor) = Host::new(
                PluginLoaderConfig {
                    allow_unsigned: true,
                    skip_wasm_validation: true,
                    ..Default::default()
                },
                verifier,
                crate::blocklist::Blocklist::new(),
            );
            tokio::select! {
                result = body(host) => result,
                _ = actor => panic!("actor exited before test body finished"),
            }
        })
    }

    #[test]
    fn health_returns_ok_with_uptime() {
        run_with_actor(|host| async move {
            let id = Uuid::new_v4();
            let response = handle(&host, Request::Health { id }).await;
            assert!(matches!(response, Response::HealthOk { id: echoed, .. } if echoed == id));
        });
    }

    #[test]
    fn load_then_unload_round_trips_through_handle() {
        run_with_actor(|host| async move {
            let load_id = Uuid::new_v4();
            let load_response = handle(
                &host,
                Request::LoadPlugin {
                    id: load_id,
                    plugin_name: "test-plugin".into(),
                    version: "1.0.0".into(),
                    permissions: serde_json::json!({}),
                    wasm_b64: b64_minimal(),
                    signature_b64: None,
                    publisher_key_id: None,
                },
            )
            .await;
            match load_response {
                Response::Ok { id: echoed, body } => {
                    assert_eq!(echoed, load_id);
                    assert_eq!(body["plugin_name"], "test-plugin");
                }
                other => panic!("expected Ok, got {:?}", other),
            }

            let unload_id = Uuid::new_v4();
            let unload_response = handle(
                &host,
                Request::UnloadPlugin {
                    id: unload_id,
                    plugin_name: "test-plugin".into(),
                },
            )
            .await;
            match unload_response {
                Response::Ok { id: echoed, body } => {
                    assert_eq!(echoed, unload_id);
                    assert_eq!(body["detached"], true);
                }
                other => panic!("expected Ok, got {:?}", other),
            }
        });
    }

    #[test]
    fn load_with_invalid_base64_returns_error_code() {
        run_with_actor(|host| async move {
            let id = Uuid::new_v4();
            let response = handle(
                &host,
                Request::LoadPlugin {
                    id,
                    plugin_name: "p".into(),
                    version: "1.0.0".into(),
                    permissions: serde_json::json!({}),
                    wasm_b64: "not-valid-base64-!!!".into(),
                    signature_b64: None,
                    publisher_key_id: None,
                },
            )
            .await;
            match response {
                Response::Error {
                    id: echoed, code, ..
                } => {
                    assert_eq!(echoed, id);
                    assert_eq!(code, "invalid_base64");
                }
                other => panic!("expected Error, got {:?}", other),
            }
        });
    }

    #[test]
    fn double_load_returns_already_loaded() {
        run_with_actor(|host| async move {
            let make_load = |id| Request::LoadPlugin {
                id,
                plugin_name: "dup".into(),
                version: "1.0.0".into(),
                permissions: serde_json::json!({}),
                wasm_b64: b64_minimal(),
                signature_b64: None,
                publisher_key_id: None,
            };
            let _first = handle(&host, make_load(Uuid::new_v4())).await;
            let second = handle(&host, make_load(Uuid::new_v4())).await;
            match second {
                Response::Error { code, .. } => assert_eq!(code, "already_loaded"),
                other => panic!("expected Error, got {:?}", other),
            }
        });
    }

    #[test]
    fn invoke_unknown_plugin_returns_not_loaded() {
        run_with_actor(|host| async move {
            let id = Uuid::new_v4();
            let response = handle(
                &host,
                Request::Invoke {
                    id,
                    plugin_name: "missing".into(),
                    function: "fn".into(),
                    input: vec![],
                },
            )
            .await;
            match response {
                Response::Error { code, .. } => assert_eq!(code, "not_loaded"),
                other => panic!("expected Error, got {:?}", other),
            }
        });
    }

    #[test]
    fn unload_unknown_plugin_is_ok_with_detached_false() {
        // Idempotent detach — easier for callers than tracking
        // exact load state.
        run_with_actor(|host| async move {
            let id = Uuid::new_v4();
            let response = handle(
                &host,
                Request::UnloadPlugin {
                    id,
                    plugin_name: "ghost".into(),
                },
            )
            .await;
            match response {
                Response::Ok { body, .. } => assert_eq!(body["detached"], false),
                other => panic!("expected Ok, got {:?}", other),
            }
        });
    }

    #[test]
    fn invoke_after_load_returns_loader_result() {
        // The minimal WASM module has no exported functions, so
        // execute_plugin_function will surface a function-not-found
        // error from the loader. We're checking that the *path*
        // works end-to-end — load → invoke → structured error —
        // not that the invoke succeeds.
        run_with_actor(|host| async move {
            let _ = handle(
                &host,
                Request::LoadPlugin {
                    id: Uuid::new_v4(),
                    plugin_name: "p".into(),
                    version: "1.0.0".into(),
                    permissions: serde_json::json!({}),
                    wasm_b64: b64_minimal(),
                    signature_b64: None,
                    publisher_key_id: None,
                },
            )
            .await;

            let id = Uuid::new_v4();
            let response = handle(
                &host,
                Request::Invoke {
                    id,
                    plugin_name: "p".into(),
                    function: "missing_fn".into(),
                    input: vec![],
                },
            )
            .await;

            // Either loader_error (function not found bubbles up
            // as a PluginLoaderError) or plugin_execution_error
            // (loader returned a failure result rather than
            // erroring) — both are valid downstream paths. What
            // matters is we got *some* structured Error rather
            // than a panic or a bare Ok.
            match response {
                Response::Error { code, .. } => {
                    assert!(
                        matches!(code.as_str(), "loader_error" | "plugin_execution_error"),
                        "expected loader_error or plugin_execution_error, got {}",
                        code
                    );
                }
                other => panic!("expected Error, got {:?}", other),
            }
        });
    }
}
