//! [`Host`] — long-lived state that owns the plugin runtime via an
//! actor task.
//!
//! Why an actor (single-task ownership) rather than `Arc<Mutex<...>>`:
//! Wasmtime `Store`s are `Send` but not `Sync`, and `WasiCtx` carries
//! `dyn` trait objects that are also not `Sync`. That makes the
//! existing `PluginSandbox` (in `mockforge-plugin-loader`) `!Sync`,
//! which means we can't share it across `tokio::spawn`'d tasks via
//! `Arc`. The actor pattern sidesteps this entirely: a single
//! background task owns the sandbox, and the connection-handling
//! tasks send `Command`s to it over a `tokio::sync::mpsc` channel
//! and receive replies via `tokio::sync::oneshot`.
//!
//! Cost: all sandbox operations serialize through the actor's queue.
//! At our plugin counts (≤25 for Team tier per the trust RFC) and
//! the per-invocation latency budget (sub-50 ms), serialization
//! overhead is negligible compared to the WASM execution cost
//! itself.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use mockforge_plugin_core::{
    PluginAuthor, PluginContext, PluginId, PluginInfo, PluginManifest, PluginState, PluginVersion,
};
use mockforge_plugin_loader::{
    PluginLoadContext, PluginLoaderConfig, PluginLoaderError, PluginSandbox,
};
use tokio::sync::{mpsc, oneshot};

use crate::signing::{SignatureVerifier, VerificationError};

/// Actor channel capacity. Generous because cloud-plugins ops are
/// rare (load/unload at attach time, invoke once per request) and
/// the cost of dropping is high (request gets a 503).
const CHANNEL_CAPACITY: usize = 256;

/// Per-loaded-plugin bookkeeping the actor needs to keep around.
///
/// The tempfile holds the WASM bytes for the plugin's lifetime —
/// `PluginLoadContext` takes a filesystem path, and the loader
/// reads from it whenever the sandbox is created. Dropping the
/// `NamedTempFile` deletes the file, so we hang on to it until the
/// plugin is unloaded.
struct PluginEntry {
    /// PluginId derived from the request's `plugin_name`.
    plugin_id: PluginId,
    /// Pinned version, used when constructing per-invocation
    /// `PluginContext`.
    version: PluginVersion,
    /// Tempfile holding the WASM bytes — kept alive so the file
    /// doesn't get GC'd while the sandbox is loaded.
    _wasm_file: tempfile::NamedTempFile,
    /// Permission grant; saved here for future runtime enforcement.
    /// Currently unused beyond storing for diagnostic purposes.
    _permissions: serde_json::Value,
}

/// Long-lived plugin-host handle. `Send + Sync` — internally just an
/// `mpsc::Sender` and an `Arc<Instant>`. All real state lives in
/// the actor future returned by [`Host::new`], which the caller
/// drives on the current task.
#[derive(Clone)]
pub struct Host {
    started_at: Arc<Instant>,
    cmd_tx: mpsc::Sender<Command>,
}

/// Future returned by [`Host::new`] that owns the `PluginSandbox`
/// and processes commands on the current task. Caller drives it
/// with `tokio::select!` alongside the server loop.
///
/// Why not `tokio::spawn(actor)`: the underlying Wasmtime `Store`
/// inside `PluginSandbox` is `Send` but its embedded `WasiCtx`
/// carries `dyn` trait objects without `+ Send` bounds (in
/// wasmtime-wasi 36), making the actor's future `!Send`. Spawning
/// it on a multi-thread tokio runtime is therefore disallowed at
/// compile time. Running it inline on the main task — which can
/// itself be `!Send` under a current-thread runtime — sidesteps
/// the issue without requiring a `LocalSet` dance.
pub type HostActor = std::pin::Pin<Box<dyn std::future::Future<Output = ()>>>;

impl Host {
    /// Construct a new host. Returns the handle (cheap to clone,
    /// shareable across spawned connection tasks) and the actor
    /// future the caller must drive on the current task.
    ///
    /// The verifier is consulted on every LoadPlugin to enforce
    /// the policy from `cloud-trust-permissions-rfc.md` §7.2 step
    /// 3. Pass [`SignatureVerifier::new(TrustStore::new(),
    /// SignatureMode::Optional)`] for the loosest behavior — the
    /// existing test fixtures use that to keep the tests focused
    /// on lifecycle rather than signing.
    ///
    /// [`SignatureVerifier::new(TrustStore::new(), SignatureMode::Optional)`]: crate::signing::SignatureVerifier::new
    pub fn new(
        loader_config: PluginLoaderConfig,
        verifier: SignatureVerifier,
    ) -> (Self, HostActor) {
        let (cmd_tx, cmd_rx) = mpsc::channel(CHANNEL_CAPACITY);
        let started_at = Arc::new(Instant::now());

        let actor: HostActor = Box::pin(actor_loop(loader_config, verifier, cmd_rx));

        (Self { started_at, cmd_tx }, actor)
    }

    /// Process uptime in whole seconds.
    pub fn uptime_secs(&self) -> u64 {
        self.started_at.elapsed().as_secs()
    }

    /// Load a plugin from inline WASM bytes. The bytes are written
    /// to a tempfile inside the actor so the loader (which takes a
    /// filesystem path) can `Module::from_file` it. Returns the
    /// loaded plugin's `PluginId` on success.
    ///
    /// The actor verifies the signature (if any) against its
    /// `SignatureVerifier` before any bytes touch the loader.
    /// `signature_b64` and `publisher_key_id` must be passed
    /// together or both `None`.
    pub async fn load_plugin(
        &self,
        plugin_name: &str,
        version_str: &str,
        permissions: serde_json::Value,
        wasm_bytes: Vec<u8>,
        signature_b64: Option<String>,
        publisher_key_id: Option<String>,
    ) -> Result<PluginId, HostError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.cmd_tx
            .send(Command::Load {
                plugin_name: plugin_name.to_string(),
                version: version_str.to_string(),
                permissions,
                wasm_bytes,
                signature_b64,
                publisher_key_id,
                reply: reply_tx,
            })
            .await
            .map_err(|_| HostError::ActorGone)?;
        reply_rx.await.map_err(|_| HostError::ActorGone)?
    }

    /// Unload a plugin. Idempotent: detaching a plugin that isn't
    /// loaded returns `Ok(false)`.
    pub async fn unload_plugin(&self, plugin_name: &str) -> Result<bool, HostError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.cmd_tx
            .send(Command::Unload {
                plugin_name: plugin_name.to_string(),
                reply: reply_tx,
            })
            .await
            .map_err(|_| HostError::ActorGone)?;
        reply_rx.await.map_err(|_| HostError::ActorGone)?
    }

    /// Invoke a function on a loaded plugin.
    pub async fn invoke(
        &self,
        plugin_name: &str,
        function: &str,
        input: Vec<u8>,
    ) -> Result<serde_json::Value, HostError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.cmd_tx
            .send(Command::Invoke {
                plugin_name: plugin_name.to_string(),
                function: function.to_string(),
                input,
                reply: reply_tx,
            })
            .await
            .map_err(|_| HostError::ActorGone)?;
        reply_rx.await.map_err(|_| HostError::ActorGone)?
    }

    /// List currently-loaded plugins. For diagnostics.
    pub async fn loaded_plugins(&self) -> Result<Vec<(String, PluginId)>, HostError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.cmd_tx
            .send(Command::ListPlugins { reply: reply_tx })
            .await
            .map_err(|_| HostError::ActorGone)?;
        reply_rx.await.map_err(|_| HostError::ActorGone)
    }
}

/// Internal command type for the actor channel. Each variant
/// carries a `oneshot::Sender` for the reply so callers can
/// `.await` results.
enum Command {
    Load {
        plugin_name: String,
        version: String,
        permissions: serde_json::Value,
        wasm_bytes: Vec<u8>,
        signature_b64: Option<String>,
        publisher_key_id: Option<String>,
        reply: oneshot::Sender<Result<PluginId, HostError>>,
    },
    Unload {
        plugin_name: String,
        reply: oneshot::Sender<Result<bool, HostError>>,
    },
    Invoke {
        plugin_name: String,
        function: String,
        input: Vec<u8>,
        reply: oneshot::Sender<Result<serde_json::Value, HostError>>,
    },
    ListPlugins {
        reply: oneshot::Sender<Vec<(String, PluginId)>>,
    },
}

/// Actor task. Owns the `PluginSandbox` and the plugin map for the
/// lifetime of the host process. Returns when the channel closes
/// (all `Host` handles dropped) or the runtime shuts down.
async fn actor_loop(
    loader_config: PluginLoaderConfig,
    verifier: SignatureVerifier,
    mut cmd_rx: mpsc::Receiver<Command>,
) {
    let sandbox = PluginSandbox::new(loader_config);
    let mut plugins: HashMap<String, PluginEntry> = HashMap::new();
    tracing::info!(
        mode = ?verifier.mode(),
        trusted_keys = verifier.trusted_key_count(),
        "plugin-host actor started"
    );

    while let Some(cmd) = cmd_rx.recv().await {
        match cmd {
            Command::Load {
                plugin_name,
                version,
                permissions,
                wasm_bytes,
                signature_b64,
                publisher_key_id,
                reply,
            } => {
                let result = handle_load(
                    &sandbox,
                    &verifier,
                    &mut plugins,
                    &plugin_name,
                    &version,
                    permissions,
                    wasm_bytes,
                    signature_b64.as_deref(),
                    publisher_key_id.as_deref(),
                )
                .await;
                let _ = reply.send(result);
            }
            Command::Unload { plugin_name, reply } => {
                let result = handle_unload(&sandbox, &mut plugins, &plugin_name).await;
                let _ = reply.send(result);
            }
            Command::Invoke {
                plugin_name,
                function,
                input,
                reply,
            } => {
                let result =
                    handle_invoke(&sandbox, &plugins, &plugin_name, &function, input).await;
                let _ = reply.send(result);
            }
            Command::ListPlugins { reply } => {
                let snapshot: Vec<(String, PluginId)> = plugins
                    .iter()
                    .map(|(name, entry)| (name.clone(), entry.plugin_id.clone()))
                    .collect();
                let _ = reply.send(snapshot);
            }
        }
    }

    tracing::info!("plugin-host actor exiting (channel closed)");
}

// Many arguments because the function is the
// destructured-Command equivalent — splitting into a struct
// would just shift names around without saving a line.
#[allow(clippy::too_many_arguments)]
async fn handle_load(
    sandbox: &PluginSandbox,
    verifier: &SignatureVerifier,
    plugins: &mut HashMap<String, PluginEntry>,
    plugin_name: &str,
    version_str: &str,
    permissions: serde_json::Value,
    wasm_bytes: Vec<u8>,
    signature_b64: Option<&str>,
    publisher_key_id: Option<&str>,
) -> Result<PluginId, HostError> {
    // Verify the signature *before* the loader sees the bytes.
    // Bypass-via-loader-bug is impossible if the bytes never
    // reach the loader on a verification failure.
    let outcome = verifier.verify(&wasm_bytes, signature_b64, publisher_key_id)?;
    match &outcome {
        crate::signing::VerificationOutcome::Verified { key_id } => {
            tracing::info!(plugin_name, version = version_str, key_id, "plugin signature verified");
        }
        crate::signing::VerificationOutcome::SkippedUnsigned => {
            tracing::warn!(
                plugin_name,
                version = version_str,
                "plugin loaded without signature (Optional mode)"
            );
        }
    }

    if plugins.contains_key(plugin_name) {
        return Err(HostError::AlreadyLoaded {
            plugin_name: plugin_name.to_string(),
        });
    }

    let version = PluginVersion::parse(version_str).map_err(|err| HostError::InvalidVersion {
        version: version_str.to_string(),
        err,
    })?;
    let plugin_id = PluginId::new(plugin_name);

    let mut wasm_file = tempfile::NamedTempFile::new().map_err(|err| HostError::Io {
        what: "creating tempfile",
        err,
    })?;
    std::io::Write::write_all(&mut wasm_file, &wasm_bytes).map_err(|err| HostError::Io {
        what: "writing wasm bytes",
        err,
    })?;
    let path = wasm_file.path().to_path_buf();

    let manifest = build_synthetic_manifest(plugin_name, version.clone());
    let load_ctx = PluginLoadContext::new(
        plugin_id.clone(),
        manifest,
        path.to_string_lossy().into_owned(),
        PluginLoaderConfig::default(),
    );

    let instance = sandbox.create_plugin_instance(&load_ctx).await?;
    debug_assert_eq!(instance.state, PluginState::Ready);

    plugins.insert(
        plugin_name.to_string(),
        PluginEntry {
            plugin_id: plugin_id.clone(),
            version,
            _wasm_file: wasm_file,
            _permissions: permissions,
        },
    );

    Ok(plugin_id)
}

async fn handle_unload(
    sandbox: &PluginSandbox,
    plugins: &mut HashMap<String, PluginEntry>,
    plugin_name: &str,
) -> Result<bool, HostError> {
    let Some(entry) = plugins.remove(plugin_name) else {
        return Ok(false);
    };
    sandbox.destroy_sandbox(&entry.plugin_id).await?;
    Ok(true)
}

async fn handle_invoke(
    sandbox: &PluginSandbox,
    plugins: &HashMap<String, PluginEntry>,
    plugin_name: &str,
    function: &str,
    input: Vec<u8>,
) -> Result<serde_json::Value, HostError> {
    let entry = plugins.get(plugin_name).ok_or_else(|| HostError::NotLoaded {
        plugin_name: plugin_name.to_string(),
    })?;

    let ctx = PluginContext::new(entry.plugin_id.clone(), entry.version.clone());
    let result = sandbox
        .execute_plugin_function(&entry.plugin_id, function, &ctx, &input)
        .await?;

    // `error()` borrows but `data()` consumes — capture the error
    // first so we can still consume on the success path.
    let error_message = result.error().map(str::to_string);
    if let Some(message) = error_message {
        return Err(HostError::PluginExecution {
            plugin_name: plugin_name.to_string(),
            message,
        });
    }
    Ok(result.data().unwrap_or(serde_json::Value::Null))
}

/// Errors from host operations. Each variant maps to a stable wire
/// `code` so callers can react programmatically without parsing
/// the human-readable message.
#[derive(Debug, thiserror::Error)]
pub enum HostError {
    /// Tried to load a plugin that's already loaded.
    #[error("plugin '{plugin_name}' is already loaded; detach before re-loading")]
    AlreadyLoaded {
        /// Plugin name that conflicted.
        plugin_name: String,
    },
    /// Tried to invoke / unload a plugin that isn't loaded.
    #[error("plugin '{plugin_name}' is not loaded")]
    NotLoaded {
        /// Plugin name that wasn't found.
        plugin_name: String,
    },
    /// `version` couldn't be parsed as a semver-shaped string.
    #[error("invalid version '{version}': {err}")]
    InvalidVersion {
        /// The string that failed to parse.
        version: String,
        /// Parse-error detail from `PluginVersion::parse`.
        err: String,
    },
    /// An I/O error materializing the WASM bytes to a tempfile.
    #[error("io error while {what}: {err}")]
    Io {
        /// Human-readable description of the operation that failed.
        what: &'static str,
        /// Underlying io::Error.
        #[source]
        err: std::io::Error,
    },
    /// The loader rejected the load or invocation.
    #[error("loader error: {0}")]
    Loader(#[from] PluginLoaderError),
    /// The plugin function returned an error/trap.
    #[error("plugin '{plugin_name}' execution failed: {message}")]
    PluginExecution {
        /// Plugin that failed.
        plugin_name: String,
        /// Error/trap message.
        message: String,
    },
    /// Base64-decode of the WASM bytes failed.
    #[error("invalid base64 wasm: {0}")]
    Base64(#[from] base64::DecodeError),
    /// Signature verification rejected the load.
    #[error("signature verification failed: {0}")]
    Signature(#[from] VerificationError),
    /// The actor task is gone — likely runtime shutdown or a
    /// panic in the actor. Caller should reconnect.
    #[error("plugin-host actor task is no longer running")]
    ActorGone,
}

impl HostError {
    /// Stable, machine-readable error code for the IPC `code` field.
    pub fn code(&self) -> &'static str {
        match self {
            HostError::AlreadyLoaded { .. } => "already_loaded",
            HostError::NotLoaded { .. } => "not_loaded",
            HostError::InvalidVersion { .. } => "invalid_version",
            HostError::Io { .. } => "io_error",
            HostError::Loader(_) => "loader_error",
            HostError::PluginExecution { .. } => "plugin_execution_error",
            HostError::Base64(_) => "invalid_base64",
            // Forward the verifier's specific code so callers can
            // distinguish "missing signature" from "wrong key" etc.
            HostError::Signature(err) => err.code(),
            HostError::ActorGone => "actor_gone",
        }
    }
}

/// Build a placeholder manifest for a plugin loaded over IPC.
///
/// The cloud trust model treats this as informational only — what
/// actually gates plugin behavior is the `permissions` grant from
/// the LoadPlugin request, enforced at the runtime boundary. A
/// future Phase 2 PR will fetch the *real* manifest from the
/// registry alongside the WASM bytes so we can validate the
/// `manifest ∩ grant` invariant inside the host.
fn build_synthetic_manifest(plugin_name: &str, version: PluginVersion) -> PluginManifest {
    let plugin_id = PluginId::new(plugin_name);
    let info = PluginInfo::new(
        plugin_id,
        version,
        plugin_name,
        "Cloud-loaded plugin (synthetic manifest)",
        PluginAuthor {
            name: "cloud-plugins-host".to_string(),
            email: None,
        },
    );
    PluginManifest::new(info)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Smallest valid WASM module bytes — `\0asm` + version 1.
    fn minimal_wasm() -> Vec<u8> {
        vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00]
    }

    fn loader_config() -> PluginLoaderConfig {
        PluginLoaderConfig {
            allow_unsigned: true,
            skip_wasm_validation: true,
            ..Default::default()
        }
    }

    /// Drive `body` and the actor future concurrently on a
    /// current-thread runtime. Tests use this so they don't need
    /// `tokio::main(flavor = "current_thread")` boilerplate.
    fn run_with_actor<F, T>(body: impl FnOnce(Host) -> F) -> T
    where
        F: std::future::Future<Output = T>,
    {
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        rt.block_on(async move {
            let verifier = SignatureVerifier::new(
                crate::signing::TrustStore::new(),
                crate::signing::SignatureMode::Optional,
            );
            let (host, actor) = Host::new(loader_config(), verifier);
            tokio::select! {
                result = body(host) => result,
                _ = actor => panic!("actor exited before test body finished"),
            }
        })
    }

    #[test]
    fn load_plugin_then_list_returns_one_entry() {
        run_with_actor(|host| async move {
            host.load_plugin(
                "test-plugin",
                "1.0.0",
                serde_json::json!({}),
                minimal_wasm(),
                None,
                None,
            )
            .await
            .unwrap();
            let loaded = host.loaded_plugins().await.unwrap();
            assert_eq!(loaded.len(), 1);
            assert_eq!(loaded[0].0, "test-plugin");
        });
    }

    #[test]
    fn load_plugin_twice_returns_already_loaded() {
        run_with_actor(|host| async move {
            host.load_plugin(
                "test-plugin",
                "1.0.0",
                serde_json::json!({}),
                minimal_wasm(),
                None,
                None,
            )
            .await
            .unwrap();
            let err = host
                .load_plugin(
                    "test-plugin",
                    "1.0.0",
                    serde_json::json!({}),
                    minimal_wasm(),
                    None,
                    None,
                )
                .await
                .unwrap_err();
            assert_eq!(err.code(), "already_loaded");
        });
    }

    #[test]
    fn unload_plugin_removes_entry() {
        run_with_actor(|host| async move {
            host.load_plugin(
                "test-plugin",
                "1.0.0",
                serde_json::json!({}),
                minimal_wasm(),
                None,
                None,
            )
            .await
            .unwrap();
            let detached = host.unload_plugin("test-plugin").await.unwrap();
            assert!(detached);
            assert!(host.loaded_plugins().await.unwrap().is_empty());
        });
    }

    #[test]
    fn unload_unknown_plugin_is_idempotent() {
        run_with_actor(|host| async move {
            let detached = host.unload_plugin("nope").await.unwrap();
            assert!(!detached);
        });
    }

    #[test]
    fn invoke_unknown_plugin_returns_not_loaded() {
        run_with_actor(|host| async move {
            let err = host.invoke("nope", "fn", vec![]).await.unwrap_err();
            assert_eq!(err.code(), "not_loaded");
        });
    }

    /// Drive a Host wired to a Required-mode verifier so we can
    /// observe end-to-end signature-rejection behavior.
    fn run_with_required_actor<F, T>(body: impl FnOnce(Host) -> F) -> T
    where
        F: std::future::Future<Output = T>,
    {
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        rt.block_on(async move {
            let verifier = SignatureVerifier::new(
                crate::signing::TrustStore::new(),
                crate::signing::SignatureMode::Required,
            );
            let (host, actor) = Host::new(loader_config(), verifier);
            tokio::select! {
                result = body(host) => result,
                _ = actor => panic!("actor exited before test body finished"),
            }
        })
    }

    #[test]
    fn load_in_required_mode_without_signature_is_rejected() {
        run_with_required_actor(|host| async move {
            let err = host
                .load_plugin(
                    "test-plugin",
                    "1.0.0",
                    serde_json::json!({}),
                    minimal_wasm(),
                    None,
                    None,
                )
                .await
                .unwrap_err();
            assert_eq!(err.code(), "signature_required");
        });
    }

    #[test]
    fn load_in_required_mode_with_unknown_publisher_key_is_rejected() {
        run_with_required_actor(|host| async move {
            // 64 zero bytes — base64-encoded — is the right shape
            // for an Ed25519 signature, but the publisher_key_id
            // isn't in the (empty) trust store.
            use base64::Engine;
            let sig_b64 = base64::engine::general_purpose::STANDARD.encode([0u8; 64]);
            let err = host
                .load_plugin(
                    "test-plugin",
                    "1.0.0",
                    serde_json::json!({}),
                    minimal_wasm(),
                    Some(sig_b64),
                    Some("unknown-key".to_string()),
                )
                .await
                .unwrap_err();
            assert_eq!(err.code(), "unknown_publisher_key");
        });
    }

    #[test]
    fn load_with_invalid_version_returns_invalid_version() {
        run_with_actor(|host| async move {
            let err = host
                .load_plugin(
                    "test-plugin",
                    "not-a-version",
                    serde_json::json!({}),
                    minimal_wasm(),
                    None,
                    None,
                )
                .await
                .unwrap_err();
            assert_eq!(err.code(), "invalid_version");
        });
    }
}
