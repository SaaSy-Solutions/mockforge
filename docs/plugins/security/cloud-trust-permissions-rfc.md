# RFC: Cloud Plugins — Trust & Permission Model

| Field        | Value                                              |
| ------------ | -------------------------------------------------- |
| **Status**   | Draft                                              |
| **Phase**    | Cloud Plugins Phase 1 — gates Phases 2+            |
| **Scope**    | MockForge Cloud (multi-tenant SaaS) only           |
| **Author**   | Ray Clanan                                         |
| **Created**  | 2026-05-05                                         |
| **Supersedes** | Nothing — extends `docs/plugins/security/model.md` for the cloud context |

## 1. Background

MockForge today supports plugins as WASM modules loaded by `mockforge-plugin-loader` into a **single-tenant** mockforge process. Trust and permissions are configured by the operator via a manifest (`PluginCapabilities` in `mockforge-plugin-core/src/types.rs`) — they trust their own server, so the model is permissive: declare what you want, get it.

**MockForge Cloud changes the trust boundary.** Hosted mocks run a per-tenant `mockforge` container on a Fly.io machine (`ghcr.io/saasy-solutions/mockforge:latest`), but plugins haven't been wired into that runtime yet. Once they are, the operator (us — SaaSy Solutions) is no longer the same actor as the plugin author or the tenant. We need an explicit model for who can install what, what those plugins can do, and how a bad plugin gets stopped.

This RFC defines that model. It is the gating design document for Phases 2+ (runtime delivery, metering, UI). No runtime work begins until this is accepted.

## 2. Goals & non-goals

### Goals

- **Multi-tenant safety.** No plugin in tenant A can read, write, or signal to tenant B's data, traffic, or runtime state.
- **Tenant control.** An org admin decides which plugins attach to which hosted mocks, with what permissions, without our involvement.
- **Author accountability.** Every cloud-installed plugin is signed and traceable to a publisher key. Anonymous code does not run in the cloud runtime.
- **Operator override.** SaaSy Solutions can globally disable any plugin within minutes when a vulnerability or abuse is reported.
- **Defense in depth.** No single layer is load-bearing. WASM sandbox + per-plugin egress proxy + signature trust + audit must all fail before tenant data leaks.

### Non-goals

- **Full network policy DSL.** v1 ships a domain allowlist, not a full L3/L4 policy. Port-level controls, IP CIDR blocks, mTLS pinning come later if needed.
- **Cross-region plugin replication.** v1 plugins attach to a single hosted-mock deployment in a single region. Multi-region active-active is out of scope.
- **Compute on private VPCs.** v1 doesn't connect plugin egress to a tenant's VPC peering. Egress targets must be publicly resolvable.
- **Re-architecting the OSS plugin model.** The self-hosted experience stays as-is. Cloud constraints layer **on top of** the existing `PluginCapabilities`, never weakening it.

## 3. Threat model

### Actors

| Actor           | Trust level | Notes |
| --------------- | ----------- | ----- |
| **Operator** (SaaSy Solutions) | Highest | Runs the platform, holds platform signing key, can revoke any plugin globally |
| **Org admin**   | High in their own org | Decides what attaches to their hosted mocks, holds optional org-private signing keys |
| **Org member**  | Medium | Can use what the admin attached; cannot install new plugins |
| **Plugin publisher** | Untrusted code, trusted identity | Their key is verified, but their code runs in a sandbox |
| **Tenant traffic** (the API client hitting the hosted mock) | Untrusted | All inputs to plugins are treated as adversarial |
| **Other tenants** | Hostile | Assume an attacker has paid for an account and is attacking from the inside |

### Assets we protect

1. **Tenant data in transit.** Request bodies, headers, env-var values that flow through hosted mocks.
2. **Tenant secrets at rest.** BYOK keys, API tokens, recorded fixtures. Plugins must never see these unless explicitly granted.
3. **Cloud runtime control plane.** Plugins cannot escalate to read/write the hosted-mock's config, the registry, or other tenants' machines.
4. **Operator keys.** The platform signing root, the kill-switch revocation key. Compromise = total system compromise.

### Out of scope (we accept the residual risk)

- **Side-channel timing attacks** between tenants on the same Fly host. Mitigated structurally by Fly's microVM isolation; not by anything in this RFC.
- **DoS via legitimate load.** Quota limits handle this — see the metering RFC (separate doc, Phase 2).
- **Phishing of org admin credentials.** Out of scope; covered by the existing auth posture (2FA, etc.).

## 4. Permission grants — the data model

### 4.1 Two-step model

Permissions are the **intersection** of:

1. **Manifest declaration** (publisher) — what the plugin needs to function, in `PluginCapabilities`
2. **Attach grant** (org admin) — what this org permits this plugin to do *on this hosted mock*

Plugin runs with `manifest ∩ grant`. The grant cannot widen beyond manifest. The manifest cannot do anything without a grant.

This is deliberate: the publisher cannot escalate without admin consent (manifest changes mean the plugin is held in a "needs re-approval" state on next version bump). The admin cannot grant something the plugin doesn't claim it needs (forces publishers to be honest about scope).

### 4.2 Grant payload schema

A grant is a row in `hosted_mock_plugins.permissions_json` (Phase 1 schema, Task #5). Strawman shape:

```json
{
  "egress": {
    "allow": ["*.stripe.com", "api.example.com"],
    "deny_all_others": true
  },
  "env": {
    "read": ["MY_PUBLIC_FLAG"],
    "write": []
  },
  "request": {
    "read_body": true,
    "modify_body": true,
    "read_headers": ["x-trace-id"],
    "modify_headers": ["x-rewritten-by"]
  },
  "response": {
    "read_body": true,
    "modify_body": true,
    "modify_status": false
  },
  "storage": {
    "kv_namespace": null
  }
}
```

Defaults are **deny-all** at every key. Empty grant = plugin loads but can do nothing. Admin must explicitly opt in to each capability the manifest claims.

### 4.3 What the manifest looks like (cloud extension)

We extend `PluginCapabilities` with cloud-specific fields. Existing self-hosted manifests stay valid — new fields are additive and optional, with cloud-mode-specific defaults:

```toml
[plugin.capabilities]
# Existing self-hosted fields (unchanged)
network = { allow_http = true, allowed_hosts = ["*.stripe.com"] }

# New cloud-specific fields
[plugin.capabilities.cloud]
# What request/response surface the plugin needs. Cloud mode requires
# this to be declared explicitly — the OSS world's "modify everything"
# default does not apply in cloud.
request_access = ["read_body", "modify_body"]
response_access = ["modify_body"]

# Env-var keys the plugin may read. Granted at attach-time per-key.
env_read = ["MY_PUBLIC_FLAG"]

# Whether the plugin maintains state across invocations.
stateful = false
```

## 5. Egress policy

### 5.1 Architecture

Plugin code never gets a raw socket. All network egress passes through a **forced HTTP proxy** running as a sibling process in the hosted-mock Fly machine (the sidecar from the architecture decision in the broader plan). The WASM runtime is configured with `wasi-http` pointing at `http://localhost:<proxy-port>`; outbound TCP/UDP is blocked at the WASI layer.

```
┌─────────────────────────── Fly machine ────────────────────────────┐
│                                                                    │
│  ┌─────────────┐     ┌──────────────┐      ┌──────────────────┐   │
│  │  mockforge  │────▶│ plugin-host  │─────▶│  egress-proxy    │   │
│  │  (main)     │ IPC │  (WASM      │ HTTP │  (host allowlist │   │
│  │             │     │  sandbox)    │      │  enforced here)   │   │
│  └─────────────┘     └──────────────┘      └────────┬─────────┘   │
│                                                     │             │
└─────────────────────────────────────────────────────┼─────────────┘
                                                      │
                                          ▼ only allowed hosts ▼
                                          public internet
```

### 5.2 Allowlist semantics

- Hosts are matched against the grant's `egress.allow` list.
- Wildcard prefix `*.example.com` matches `api.example.com` and `foo.bar.example.com` but **not** `example.com` (require explicit listing).
- Match is on the resolved hostname *before* connect — DNS resolution happens inside the proxy, not the plugin.
- Denied hosts return `HTTP 403` from the proxy with a structured body the plugin can detect (so a well-behaved plugin can degrade gracefully).
- Every allowed *and* denied request emits an audit event (see §8).

### 5.3 What we explicitly block

- **Cloud metadata endpoints.** `169.254.169.254`, `metadata.google.internal`, all standard cloud-provider IMDS. Hard-coded denylist *above* the allowlist.
- **Internal addresses.** RFC1918, link-local, loopback, fly-internal IPv6. Even if a tenant lists `192.168.1.5` we don't connect.
- **The registry.** `registry.mockforge.dev`, `app.mockforge.dev`. A plugin cannot call back to MockForge Cloud's own control plane.
- **Other tenants' hosted mocks.** `*.fly.dev` matching MockForge's app pattern. (Tenants who legitimately want to call their own hosted mock from a plugin can use the public domain.)

### 5.4 What we don't enforce in v1

- **Outbound payload inspection.** The proxy doesn't deep-inspect bodies. A determined plugin can exfiltrate via timing, DNS lookups (mitigated by routing DNS through proxy), or steganography in legitimate traffic to allowed hosts. We accept this as the price of allowing useful plugins; signature trust + audit + revocation is the answer.
- **Per-route egress rules.** All requests through a plugin share the same allowlist. No "this rewrite-rule may call X, that one Y."

## 6. Environment variable scope

### 6.1 Default

Plugins see **zero** environment variables. Not even `PATH`. The WASM sandbox is started with an empty environment.

### 6.2 Grants

Env access is per-key, read-only:

- The manifest declares `env_read = ["MY_FLAG", "OTHER_FLAG"]`.
- The grant lists which subset is allowed: `env.read = ["MY_FLAG"]`.
- Plugin code calls a host function `mockforge_env_get("MY_FLAG")` and gets the value or `null` if not granted.
- Direct WASI environment access is disabled.

### 6.3 What env vars are these?

Per-deployment env vars set by the org admin in the hosted-mock config UI. **Not** Fly machine env vars (which contain platform secrets). The plugin host reads from a designated `cloud_plugin_env_<deployment_id>` table in the registry, not from `std::env`.

### 6.4 Hard exclusions

- `MOCKFORGE_*` — platform internals, never granted regardless of admin opt-in
- `BYOK_*`, `STRIPE_*`, anything matching the secret-name patterns we store encrypted
- The Fly machine's runtime env (`FLY_*`)

These are denied at the host-function layer, even if the manifest and grant agree.

## 7. Signature trust

### 7.1 Two-tier trust

**Public marketplace plugins** must be signed by a key registered to the publisher's MockForge account (existing `UserPublicKey` model + `verify_sbom_attestation`). On install-to-cloud, the registry verifies the signature against the publisher's *currently active* keys (revoked keys reject).

**Org-private plugins** must be signed by a key the org admin has registered as an org-trust-root. New table `organization_trust_roots`:

```sql
CREATE TABLE organization_trust_roots (
    id UUID PRIMARY KEY,
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    public_key BYTEA NOT NULL,
    name VARCHAR(128) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    revoked_at TIMESTAMPTZ
);
```

Org-private plugins are not visible in the public marketplace, do not appear in `plugin-registry`, and can only be attached within the org that holds the signing root.

### 7.2 Verification points

A plugin's signature is verified at **three** points, redundantly:

1. **At publish** — registry rejects upload if signature fails (existing behavior).
2. **At attach** — `POST .../plugins` rejects if the plugin's current signature doesn't verify against an active key (publisher- or org-trust-root). Catches "key was revoked between publish and install."
3. **At runtime boot** — plugin-host on the Fly machine fetches signed WASM, verifies *again* before instantiating. Catches "blob was tampered between registry storage and machine pull."

The plugin-host bundles a small set of trusted root keys at build time so it can verify without round-tripping to the registry on cold start.

### 7.3 What signatures cover

- The WASM binary (canonical hash)
- The manifest (canonical TOML serialization)
- The plugin's claimed `name@version` identity

A signature mismatch between any of these and what's actually loaded is a hard fail. We do not fall back to "load anyway with a warning."

## 8. Revocation & kill-switch

### 8.1 Tiers of revocation

| Tier | Trigger | Effect | Latency target |
| ---- | ------- | ------ | -------------- |
| **Org-scoped detach** | Org admin clicks "Detach" in UI | Plugin removed from one deployment; other orgs unaffected | < 30s (one machine reload) |
| **Org-trust-root revoke** | Org admin revokes one of their signing keys | All plugins signed by that key fail to load on next boot in that org | < 5 min |
| **Publisher key revoke** | Publisher revokes a key (or operator does on their behalf) | All plugins signed by that key fail re-verification across all orgs that installed them. New attaches reject. Existing running instances continue until next boot. | < 15 min for new attaches, ≤ deployment lifetime for in-flight |
| **Global plugin disable** (kill-switch) | Operator pushes a revocation entry to a global blocklist | All instances of that `name@version` (or version range) are killed across all tenants on next plugin-host poll. New attaches reject globally. | < 5 min |
| **Global publisher ban** | Operator bans a publisher account | All their plugins go to global-disabled simultaneously | < 5 min |

### 8.2 Kill-switch implementation

Plugin-host polls a blocklist endpoint (`GET /api/internal/plugin-blocklist`) every 60s. The blocklist is an append-only signed manifest of `(name, version_range, reason, revoked_at)` entries. On a hit, the host:

1. Stops accepting new invocations of the matched plugin
2. Logs an audit event with `reason`
3. Returns 503 from any in-flight invocation
4. On next mockforge restart, the plugin is not reloaded

The blocklist itself is signed by the platform root key. A compromised plugin-host that ignores the blocklist still gets caught at re-attach time when the registry checks it.

### 8.3 Audit trail

Every revocation event is a row in the existing `audit_logs` table — adds new variants to the `audit_event_type` enum (e.g. `plugin_revoked`, `plugin_blocklist_hit`). Org admins see their org's events; operator sees all.

## 9. Operator key management

The platform signing root (used to sign the blocklist and the trusted-root bundle baked into plugin-host) lives in HSM-backed storage. Two-of-three quorum required for any signing operation. Rotation procedure:

1. Generate new key
2. Plugin-host releases ship with both old and new public keys (overlap window)
3. After all hosted mocks are updated, old key is retired
4. Old key remains in the verifier set for 90 days for replay protection

This is high-stakes and out of scope for the first runtime ship. v1 can use a single offline key with manual rotation; HSM + quorum is a hardening item before GA.

## 10. Open questions

These are decisions worth pressure-testing before this RFC is accepted. Listed in rough order of how blocking they are.

1. **Sidecar IPC protocol.** Unix socket + length-prefixed protobuf is the obvious choice, but `wasi-http` is becoming the WASM standard for HTTP-shaped APIs. Picking `wasi-http` for the request/response surface (not just egress) means we get tooling and ecosystem alignment. Tradeoff: more abstraction, slower for in-process calls. **Recommendation:** `wasi-http` for plugin ↔ host, raw socket for host ↔ proxy.
2. **Cold-start budget.** Loading + verifying a WASM module takes ~50–500ms depending on size. If a hosted mock attaches 5 plugins, a 2.5s cold start is unacceptable. **Recommendation:** preload all attached plugins at machine boot (already in the broader plan), keep them resident for the deployment's lifetime.
3. **Blocklist freshness.** 60s poll is OK for "detach" actions but slow for an active exploit. **Recommendation:** add a push channel via the existing OTLP connection so global kill-switch is sub-second; keep poll as the failsafe.
4. **Org-trust-root key recovery.** What happens if an org loses their private key? Currently: their private plugins are unrunnable until re-signed by a new key. This is correct for security but operationally painful. **Recommendation:** support multiple active trust roots per org (already in the schema as separate rows), document key-rotation as the recovery path.
5. **Audit retention for plugin invocations.** Per-invocation logs would dwarf everything else in `audit_log`. **Recommendation:** sample at 1% by default, configurable per-org, full audit only on revocation events. Detailed metering goes to OTLP, not the audit log.
6. **Should we ship "permission preview" in the UI?** When attaching a plugin, show the admin a diff: "this plugin requests A, B, C — you're granting A, C." Probably yes, but it's UI work that can ship in Phase 3 — not load-bearing for the trust model.
7. **What's our position on plugins that need persistent state?** v1 says `stateful = false` only. Persistent KV would be a separate add-on with its own quota model. Calling this out here so we don't accidentally ship a state-leaking shared store. **Recommendation:** explicitly reject `stateful = true` in v1 attach.
8. **Multi-region.** All of the above assumes one region per deployment. When we go multi-region, does each region's plugin-host trust the same blocklist? Almost certainly yes, but the rotation story changes. **Defer** to multi-region planning.

## 11. What this RFC unblocks (and what it doesn't)

**Unblocks:**
- Phase 1 schema migration (Task #5) — `hosted_mock_plugins.permissions_json` shape is now defined.
- Phase 1 control-plane API (Task #6) — attach-time grant validation is now specified.
- Phase 2 runtime delivery (gated on Task #8 go/no-go) — egress proxy, signature verification, kill-switch are now scoped tasks rather than open questions.

**Does not unblock:**
- The build-vs-buy decision (Task #2). If we delegate to Fastly Compute@Edge or Spin, several layers in this RFC (sandbox, sidecar, env scope) are managed by them — we'd need a thinner trust layer focused on signature/grant/audit only.
- The metering RFC (separate doc, Phase 2). Quota enforcement is its own design.

## 12. Decision log

| Date | Decision | By |
| ---- | -------- | -- |
| 2026-05-05 | Draft created                                              | Ray Clanan |
