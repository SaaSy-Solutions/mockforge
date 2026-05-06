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
//! **Phase 2 scaffold.** Boots, listens on the Unix socket, and
//! handles `health` round-trips. The `load_plugin` / `unload_plugin`
//! / `invoke` handlers return `not_implemented` placeholders — wiring
//! them to the real `mockforge-plugin-loader::sandbox::PluginSandbox`
//! is the next deliverable.
//!
//! ## Wire protocol
//!
//! JSON requests / responses, one per Unix socket message, length-
//! prefixed. Protocol is intentionally simple and language-agnostic
//! so we can swap to protobuf later without breaking the on-the-wire
//! semantics. See [`protocol`].
//!
//! ## Why a separate crate from `mockforge-plugin-loader`?
//!
//! The loader is OSS — it ships in the local mockforge binary for
//! self-hosted users. The host is cloud-only: it owns IPC, signature
//! verification at boot, OTLP export to the cloud aggregator, and
//! the kill-switch poll. Keeping them split lets the OSS surface
//! stay small and lets us iterate on cloud-only concerns without
//! touching the OSS API.

pub mod handlers;
pub mod protocol;
pub mod server;

pub use protocol::{Request, Response};
pub use server::{run_server, ServerConfig};
