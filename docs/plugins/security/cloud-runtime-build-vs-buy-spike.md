# Spike: Cloud Plugin Runtime — Build vs. Buy

| Field        | Value                                                       |
| ------------ | ----------------------------------------------------------- |
| **Status**   | Recommendation                                              |
| **Phase**    | Cloud Plugins Phase 1 — informs Phase 2 architecture        |
| **Author**   | Ray Clanan                                                  |
| **Created**  | 2026-05-05                                                  |
| **Companion**| `cloud-trust-permissions-rfc.md` (the trust model this serves) |

## TL;DR

**Build the runtime in-house. Constrain plugins to standard WASI 0.2 + `wasi:http`. Don't adopt any of the managed offerings as primary.**

All three buy options were evaluated against the trust model in the companion RFC. Each fails on at least one structural axis we cannot work around. The in-house path is what we already operate (Wasmtime via `mockforge-plugin-loader`) plus three discrete additions: a sidecar process, a custom `wasi:http/outgoing-handler` provider that enforces the egress allowlist, and a cosign verification step at attach + boot.

The non-obvious insight: **portability is what we actually want from "buy."** If our plugins are standard WASI components, we keep the option to host them on Spin, SpinKube, or vanilla Wasmtime later — without trapping ourselves in a vendor's roadmap today. Buy optionality without buying the runtime.

## Evaluation matrix

Verdicts on a 5-axis grading for each option. **Green** = native fit, **yellow** = workable with effort, **red** = structural mismatch.

| Axis | Fastly Compute | Fermyon Cloud (managed) | Spin OSS (self-host) | wasmCloud | Build (Wasmtime + sidecar on Fly) |
| ---- | -------------- | ----------------------- | -------------------- | --------- | --------------------------------- |
| **Hostile multi-tenant isolation** | 🟡 needs 1 service / tenant; account-cap unclear | 🟡 needs 1 app / tenant; Growth caps at 100 apps | 🟡 1 Spin instance / Fly machine = OK | 🔴 ADR-0007 explicitly rules out hostile multi-tenancy on a shared host | 🟢 already 1 Fly machine / tenant |
| **Per-tenant egress allowlist (hostnames + RFC1918 deny)** | 🔴 ACLs IP/CIDR only, no hostname allowlist | 🟡 `allowed_outbound_hosts` per component, but **no IP-level deny — DNS for an allowed host could resolve to RFC1918** | 🟡 same as Fermyon Cloud (Spin's allowlist) | 🔴 no built-in egress allowlist; "write a custom provider" | 🟢 we control the proxy entirely |
| **Signature verification (publish + attach + boot)** | 🔴 not platform-native; bring your own | 🔴 not Spin-native; bring your own (cosign) | 🔴 not Spin-native; bring your own (cosign) | 🟡 cosign + Sigstore exists; revocation story is weak | 🟢 we control all three checkpoints |
| **Per-invocation OTLP metering for billing** | 🟡 logs exist; no native "billable usage" pipe | 🟡 OTLP traces yes; per-component wall-time histogram unconfirmed | 🟡 OTLP traces yes; per-component wall-time histogram unconfirmed | 🟢 native OTLP + per-actor counters | 🟢 we instrument what we need |
| **Vendor lock-in / portability** | 🔴 proprietary host ABI (`fastly_http_req`); not portable | 🔴 commercial product **acquired by Akamai Dec 2025**; pricing page redirects, roadmap unclear | 🟢 plain WASI 0.2; portable | 🟡 standard WASI 0.2 components are portable; `wasmcloud:*` interfaces are not | 🟢 we define plugin shape; can target standard WASI for portability |

## Per-option findings

### Fastly Compute — reject

**The blocker is per-tenant egress.** Fastly's outbound model is "declared backends per service." ACLs are IP/CIDR-only ([docs](https://www.fastly.com/documentation/guides/security/access-control-lists/)) — no hostname allowlist primitive. To get per-tenant egress policy we'd need one Fastly service per tenant, but the services-per-account ceiling is not publicly documented ("contact sales"). Combined with a proprietary host ABI (`fastly_http_req`, etc.), plugins built against the Fastly SDK are not portable to Spin or Wasmtime without rewriting.

Bonus negative: **no documented public reference of Fastly Compute being used as a hostile-multi-tenant customer-plugin runtime.** Almost all Fastly Compute customers run their *own* WASM (CDN logic), not their *customers'* WASM. We'd be charting territory the docs don't acknowledge.

Pricing for our shape is reasonable (~$40–50 / customer / month at Team-tier volume, 50M req/mo with 5–25ms compute), but the structural problems above mean cost is not the deciding factor.

### Fermyon Cloud (managed) — reject

**Fermyon was acquired by Akamai on December 1, 2025.** `fermyon.com/pricing` 301-redirects to Akamai Functions. The OSS projects (Spin, SpinKube) remain CNCF / Bytecode Alliance and are unaffected, but "Fermyon Cloud" as a discrete product is being absorbed. Treat any commercial commitment as a bet on Akamai's roadmap, which is not yet public.

Even ignoring the acquisition: the documented quotas don't fit a SaaS where we're the customer holding our customers' code. Growth tier caps at 100 apps. **500 outbound req/hr/app is a non-starter** for any plugin that calls external APIs. Enterprise sales would be required from day one.

### Spin OSS (self-hosted) — credible second-place; not primary

This is the one buy option that survives the constraint check. Spin is Apache-2.0 OSS, runs on Wasmtime, components target standard WASI 0.2, and components written for Spin can be composed across Wasmtime and (with care) wasmCloud ([InfoWorld](https://www.infoworld.com/article/2335330/spin-20-shines-on-wasm-component-composition-portability.html)). SpinKube is a CNCF Sandbox project as of January 2025.

The reasons it's not primary:
1. **Egress allowlist is hostname-only.** [CVE-2024-32980](https://github.com/spinframework/spin/security/advisories/GHSA-f3h7-gpjj-wcvh) (fixed in Spin 2.4.3) was a real-world `Host`-header escape against `"self"` outbound. We'd still need an IP-level firewall layer below Spin to deny RFC1918 + cloud metadata, which means we're already running our own proxy — at which point the marginal value of Spin shrinks.
2. **Signature verification is not platform-native.** Spin signs its *own releases* (SIP-012, cosign), but does not verify component signatures at instantiation. We'd write that ourselves anyway.
3. **No per-component CPU/memory caps in the manifest.** Wasmtime's `ResourceLimiter` and `epoch_interruption` are the right primitives, but Spin doesn't expose them per-component. We'd embed Wasmtime directly to use them — which is the build path.
4. **No public reference of Spin being used for genuinely-multi-tenant-hostile-customer-code at scale.** Same gap as Fastly.

**Where Spin does fit:** as the **portability target** for our plugin shape. If we write our manifest format to align with Spin's component manifest, customers' plugins would run on a vanilla Spin runtime with minor work. That's the optionality argument.

### wasmCloud — reject

The project's own [ADR-0007](https://wasmcloud.github.io/adr/0007-tenancy.html) states the host struct is the smallest unit of tenancy and explicitly rejects sharing one host across mutually-distrusting tenants. That alone disqualifies it for our threat model — we'd need one wasmCloud host per customer, which collapses most of the value prop.

Bonus negatives:
- No built-in egress allowlist; the Policy Service does start-time gating, not per-request URL filtering.
- The widely-cited American Express FaaS case study [is not actually in production yet](https://thenewstack.io/amexs-faas-uses-webassembly-instead-of-containers/).
- The runtime is mid-rewrite. The November 2025 [`wash-runtime` post](https://wasmcloud.com/blog/2025-11-05-introducing-the-next-generation-wasmcloud-runtime/) deprecates capability providers and the NATS lattice — the architecture you'd be evaluating today won't be the architecture you're operating two years from now.
- Operational complexity (NATS cluster + wadm + Policy Service + OCI registry) is meaningful for a small team.

## The build path (Phase 2 architecture, refined)

The recommendation tightens the in-house plan to a specific shape:

```
┌─────────────────────────── Fly machine (one per customer) ────────────────┐
│                                                                            │
│  ┌─────────────┐    Unix      ┌──────────────┐     HTTP    ┌─────────────┐│
│  │  mockforge  │   socket     │ plugin-host  │   (only     │  egress-    ││
│  │  (main)     │◄────────────►│              │   wasi:     │  proxy      ││
│  │             │              │  Wasmtime    │   http)     │  (allowlist ││
│  │             │              │  Engine      │             │  enforced)  ││
│  └─────────────┘              └──────┬───────┘             └──────┬──────┘│
│                                      │                            │       │
│                                      ▼                            │       │
│                          one Wasmtime Store                       │       │
│                          per plugin instance                      │       │
│                          (ResourceLimiter, epoch)                 │       │
└───────────────────────────────────────────────────────────────────┼───────┘
                                                                    │
                                                                    ▼
                                                       only allowlisted hosts
                                                       (RFC1918/metadata blocked)
```

Components we build (vs. import):
- **plugin-host process** — embeds Wasmtime, loads signed WASM components, hosts the `wasi:http/outgoing-handler` impl that funnels all egress through the proxy. ~600 LOC Rust.
- **egress-proxy process** — tiny HTTP forward proxy with the per-plugin allowlist + hard denylist (RFC1918, 169.254.169.254, etc.). ~200 LOC Rust.
- **Plugin manifest extensions** in `mockforge-plugin-core` for the cloud-specific fields the trust RFC defined (env scope, request/response access, stateful flag).
- **Cosign verification path** in the plugin-host — verifies the WASM binary signature against the publisher key (or org-trust-root) on attach and on boot, before instantiating.
- **OTLP exporter for plugin invocations** — wraps each call site, emits wall-time + memory-peak.

Components we reuse:
- `mockforge-plugin-loader` — already uses Wasmtime; existing memory-tracking is the foundation for billing metering.
- `mockforge-plugin-core::types::PluginCapabilities` — extend with cloud fields per the trust RFC.
- The existing `UserPublicKey` + `verify_sbom_attestation` path for publisher-signed plugins.
- Fly machine isolation for the per-tenant boundary (already trusted; we don't reinvent it).

**Effort estimate stays at ~3 weeks for Phase 2** — same as the original plan. The spike doesn't change the build budget; it confirms it.

## The portability constraint (the new piece)

Customers' plugins should be standard `wasi:http` components, **not** code that targets a MockForge-specific ABI. This is the cheap insurance policy:

1. If we ever want to migrate the runtime to Spin / SpinKube, customers' plugins keep running.
2. If a customer wants to test their plugin locally without our runtime, they can use `wasmtime serve` or any standard WASI runner.
3. If a Fastly-style edge runtime ever becomes a fit (e.g., for low-latency request rewriting), porting becomes a runtime config change, not a customer-code rewrite.

Concretely: our plugin SDK targets `wasi:http/incoming-handler` (the standard) plus a small bespoke `mockforge:plugin/host` interface for things WASI doesn't cover (env grants, plugin metadata). Anything that *can* be done in standard WASI, *must* be done in standard WASI.

The trust RFC's open question §10.1 ("Sidecar IPC protocol — `wasi-http` for plugin↔host vs. raw protobuf") is now answered: **`wasi:http` for plugin↔host, raw socket for host↔proxy.**

## What this spike unblocks

- **Task #3 (sidecar Fly proof)** — green-light to run, with the architecture above as the target.
- **Task #5 (schema migration)** — `permissions_json` shape from the trust RFC is the contract; this spike adds nothing new.
- **Task #6 (control-plane API)** — same.
- **Task #7 (wall-time accounting)** — bumped up in priority. Embedding Wasmtime directly means we own the metering layer fully; this work is the foundation.

## What this spike does *not* unblock

- The runtime build itself. Still gated on **Task #8 (go/no-go after Phase 0 demand signal)**.
- The metering RFC (separate doc, Phase 2). The shape of "what counts as a billable invocation" is its own design.

## Open questions (defer to Phase 2 design)

1. **Wasmtime tuning.** `epoch_interruption` interval, fuel budget per invocation, memory cap defaults — calibrate against Phase 0 use-case data (what *kind* of plugins do beta users actually want?).
2. **Cold start budget.** Trust RFC §10.2 said "preload at boot." This holds, but with `wasi:http` we can also pre-instantiate Stores per attached plugin; benchmark which is cheaper for our workload.
3. **Fly machine right-sizing.** The current shared-cpu-1x (256MB) hosted-mock floor needs revisiting if plugin-host + egress-proxy + customer's mockforge are all on the same machine. Probably need to bump the floor to shared-cpu-2x for cloud-plugin-enabled deployments. Pricing implication for the Pro tier — flag for product.
4. **Per-region failover.** The portability constraint means we *could* offload to Akamai/Fastly edge in a region we don't have a Fly presence in, as long as the plugin is standard `wasi:http`. Don't build for this; just don't accidentally close the door.

## Decision log

| Date | Decision | By |
| ---- | -------- | -- |
| 2026-05-05 | Build, with portability constraint to standard WASI 0.2 + `wasi:http`. All three buy options rejected. | Ray Clanan |
