//! # MockForge Plugin Egress Proxy
//!
//! HTTP forward proxy that gates outbound plugin traffic through a
//! per-plugin hostname allowlist and a hard denylist for cloud
//! metadata IPs + RFC1918 ranges. Sits as the third process in a
//! cloud-plugins-enabled hosted-mock Fly machine, alongside main
//! `mockforge` and `mockforge-plugin-host`.
//!
//! See `docs/plugins/security/cloud-trust-permissions-rfc.md` §5 for
//! the threat model and §6 for the egress policy semantics.
//!
//! ## What this proxy enforces
//!
//! - **Hostname allowlist**: a request to `Host: api.example.com` is
//!   allowed only if `api.example.com` matches an entry in the
//!   plugin's grant. Wildcards (`*.example.com`) are supported and
//!   match one or more leftmost labels.
//! - **Hard denylist** (always-on, can't be opted out of):
//!   - Cloud metadata: `169.254.169.254`, `fd00:ec2::254`,
//!     `metadata.google.internal`
//!   - RFC1918: `10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`
//!   - Loopback: `127.0.0.0/8`, `::1/128`
//!   - Link-local: `169.254.0.0/16`, `fe80::/10`
//!   - The MockForge registry itself (so plugins can't loop back).
//! - **HTTP CONNECT** for HTTPS pass-through. We don't MITM TLS —
//!   that would break cert pinning and is itself a security risk.
//!   The CONNECT host header is checked against the allowlist
//!   before the tunnel opens.
//!
//! ## What this proxy explicitly doesn't do
//!
//! - **Body inspection.** Once a request is allowed, body content
//!   is opaque — we accept that determined plugins can exfiltrate
//!   via legitimate traffic to allowed hosts (RFC §5.4). Audit +
//!   signature trust + revocation are the answer to that.
//! - **Per-route policy.** All requests through a plugin share the
//!   same allowlist. No "this rewrite-rule may call X, that one Y."

pub mod denylist;
pub mod policy;
pub mod proxy;

pub use denylist::is_denied_target;
pub use policy::{HostPolicy, PolicyDecision};
pub use proxy::{run_proxy, ProxyConfig};
