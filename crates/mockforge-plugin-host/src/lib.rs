//! # MockForge Plugin Host
//!
//! Sidecar binary that runs alongside `mockforge` in the same Fly
//! machine, hosts the WASM plugin runtime, and exposes a Unix-socket
//! IPC interface that main mockforge calls into for each plugin
//! invocation.
//!
//! See `docs/plugins/security/cloud-trust-permissions-rfc.md` and
//! `docs/plugins/security/cloud-runtime-build-vs-buy-spike.md` for
//! the design.
//!
//! ## Status
//!
//! **Phase 2 — IPC handlers wired.** Health/LoadPlugin/UnloadPlugin/
//! Invoke all dispatch through to [`Host`], which owns a real
//! `mockforge_plugin_loader::sandbox::PluginSandbox`. WASM bytes
//! are inlined as base64 in the LoadPlugin request; the registry-
//! fetch path lands separately so signature verification can hook
//! in upstream.
//!
//! ## Wire protocol
//!
//! Newline-delimited JSON, request/response tagged by `id` for
//! multiplexing on a single connection. Protocol is intentionally
//! simple and language-agnostic so we can swap to protobuf later
//! without breaking the on-the-wire semantics. See [`protocol`].
//!
//! ## Why a separate crate from `mockforge-plugin-loader`?
//!
//! The loader is OSS — it ships in the local mockforge binary for
//! self-hosted users. The host is cloud-only: it owns IPC, signature
//! verification at boot, OTLP export to the cloud aggregator, and
//! the kill-switch poll. Keeping them split lets the OSS surface
//! stay small and lets us iterate on cloud-only concerns without
//! touching the OSS API.

pub mod blocklist;
pub mod handlers;
pub mod host;
pub mod metering;
pub mod protocol;
pub mod server;
pub mod signing;
pub mod trust_root_cache;

pub use blocklist::{run_poll_loop, Blocklist, BlocklistConfig, BlocklistEntry, PollError};
pub use host::{Host, HostActor, HostError};
pub use metering::{run_exporter, ExportedMetric, ExporterConfig};
pub use protocol::{Request, Response};
pub use server::{run_server, ServerConfig};
pub use signing::{
    SignatureMode, SignatureVerifier, TrustStore, TrustStoreError, VerificationError,
    VerificationOutcome,
};
pub use trust_root_cache::{
    run_trust_root_refresh_loop, validate_trust_roots_url, RefreshError, TrustRootCacheConfig,
    DEFAULT_REFRESH_INTERVAL_SECS,
};
