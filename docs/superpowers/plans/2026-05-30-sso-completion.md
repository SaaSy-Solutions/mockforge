# SSO Completion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make SSO end-user-usable — close the SAML login loop in the UI and add OIDC — so the Team-tier "SSO" claim is true.

**Architecture:** Reuse the existing per-org, Team-gated SSO foundation (`sso_configurations`, `handlers/sso.rs` SAML, `OrganizationPage` SSOTab). Add OIDC handlers mirroring the OAuth2 callback→JWT pattern (`handlers/oauth.rs`), generalize JIT provisioning so SAML+OIDC share it, add email-domain discovery, and wire the frontend login entry + `/auth/sso/callback`. Both protocols converge on one frontend callback.

**Tech Stack:** Rust (axum, sqlx, `oauth2`, **new: `openidconnect`**, jsonwebtoken, redis), React 19 + TS (Zustand, TanStack Query, react-router v7), Postgres.

**Spec:** `docs/superpowers/specs/2026-05-30-sso-completion-design.md`

**Reference patterns to mirror (read these first):**
- SAML login init + ACS + `find_or_create_user_from_saml`: `crates/mockforge-registry-server/src/handlers/sso.rs` (login init, ACS, provisioning at :1174-1224).
- OAuth2 authorize + callback (closest OIDC template): `crates/mockforge-registry-server/src/handlers/oauth.rs:49-256` (CSRF state in Redis, code exchange, find-or-create, token-pair issuance).
- SSO config CRUD: `handlers/sso.rs` `create_sso_config`/`get_sso_config`; model `crates/mockforge-registry-core/src/models/sso.rs`.
- JWT issuance: `crates/mockforge-registry-core/src/auth.rs` `create_token_pair`/`create_access_token`.
- Routes: `crates/mockforge-registry-server/src/routes.rs` (public SAML routes + authenticated SSO config routes).
- Frontend auth: `crates/mockforge-ui/ui/src/stores/useAuthStore.ts`, `services/authApi.ts`, `components/auth/LoginForm.tsx`; SSOTab in `pages/OrganizationPage.tsx:1171`.

**Conventions:** Per-crate verification only (`cargo clippy -p mockforge-registry-server --all-targets --all-features -- -D warnings`, `cargo test -p mockforge-registry-server`). UI: `pnpm type-check` + `pnpm lint` in `crates/mockforge-ui/ui`. Run cargo with sandbox off (aws-lc) + `set -o pipefail`. Commit per task. Use the product-ui skill conventions for the React work.

---

### Task 1: Migration — `email_domain` on `sso_configurations`

**Files:**
- Create: `crates/mockforge-registry-server/migrations/<NEXT_PREFIX>_sso_email_domain.sql`

**CRITICAL — migration prefix:** Do NOT assume a prefix. Run `ls crates/mockforge-registry-server/migrations/ | sort | tail -3` AND `gh pr list --repo SaaSy-Solutions/mockforge --state open --json headRefName,files` to confirm no concurrent migration. The pre-commit `check-migration-collisions` hook will block a collision. Pick the next free `20250101000NNN` prefix.

- [ ] **Step 1:** Write the migration:
```sql
-- Email-domain discovery for SSO: map a work-email domain to the org's IdP.
ALTER TABLE sso_configurations
    ADD COLUMN IF NOT EXISTS email_domain VARCHAR(255);

-- Unique so one domain maps to at most one org (discovery is deterministic).
-- Partial: only enforced for rows that set a domain.
CREATE UNIQUE INDEX IF NOT EXISTS idx_sso_email_domain
    ON sso_configurations (lower(email_domain))
    WHERE email_domain IS NOT NULL;
```
- [ ] **Step 2:** Verify it parses against a scratch DB if one is available (`sqlx migrate run` with `DATABASE_URL`), else rely on CI. Commit.
```bash
git add crates/mockforge-registry-server/migrations/
git commit -m "feat(registry): add email_domain to sso_configurations for SSO discovery (#746)"
```

---

### Task 2: Model + store — `email_domain` field & domain lookup

**Files:**
- Modify: `crates/mockforge-registry-core/src/models/sso.rs` (add `pub email_domain: Option<String>` to `SSOConfiguration`, after the oidc fields ~:56; ensure `FromRow` + any explicit `INSERT`/`SELECT`/`upsert` SQL includes the column).
- Modify: store layer — find where `create_sso_config`/`find_by_org` SQL lives (grep `sso_configurations` in `crates/mockforge-registry-server/src` and `mockforge-registry-core`). Add a query `find_org_slug_by_email_domain(pool, domain) -> Option<(org_slug, provider)>`.

- [ ] **Step 1 (test-first):** In `models/sso.rs` tests, add a unit test for a pure helper `normalize_email_domain(email: &str) -> Option<String>` (lowercases, takes substring after `@`, returns None if no `@`/empty).
```rust
#[test]
fn email_domain_normalization() {
    assert_eq!(normalize_email_domain("Jo@Acme.com"), Some("acme.com".to_string()));
    assert_eq!(normalize_email_domain("no-at-sign"), None);
    assert_eq!(normalize_email_domain("a@"), None);
}
```
- [ ] **Step 2:** Implement `normalize_email_domain`. Run `cargo test -p mockforge-registry-core sso`. Expected PASS.
- [ ] **Step 3:** Add the `email_domain` column to the struct + all SQL touching `sso_configurations` (INSERT columns/values, SELECT lists). Add the `find_org_slug_by_email_domain` query (join `sso_configurations` → `organizations` on `org_id`, `WHERE lower(email_domain)=lower($1) AND enabled=true`, return slug + provider).
- [ ] **Step 4:** `cargo build -p mockforge-registry-core -p mockforge-registry-server` (sandbox off). Fix SQL/struct mismatches. Commit.

---

### Task 3: Shared JIT provisioning (`find_or_create_sso_user`)

**Files:**
- Modify: `crates/mockforge-registry-server/src/handlers/sso.rs:1174-1224` — rename/generalize `find_or_create_user_from_saml` to `find_or_create_sso_user(state, email: &str, username: Option<&str>, org: &Organization) -> Result<User, ApiError>` (take email/username directly instead of `SAMLUserInfo`); update the SAML ACS caller to pass `user_info.email`/`user_info.username`.

- [ ] **Step 1:** Refactor the function signature to accept `email: &str` + `username: Option<&str>`. Body unchanged otherwise (link-by-email → ensure Member; else create+verify+add Member). Keep behavior identical.
- [ ] **Step 2:** Update the SAML ACS call site to pass the extracted email/username.
- [ ] **Step 3:** `cargo build -p mockforge-registry-server` (sandbox off). `cargo test -p mockforge-registry-server --lib sso`. Commit: "refactor(registry): generalize SSO JIT provisioning for SAML+OIDC (#746)".

---

### Task 4: Discovery endpoint `GET /api/v1/sso/discover`

**Files:**
- Modify: `crates/mockforge-registry-server/src/handlers/sso.rs` (add `discover_sso` handler + `SsoDiscoverResponse { org_slug: String, provider: String }`).
- Modify: `crates/mockforge-registry-server/src/routes.rs` (register `GET /api/v1/sso/discover` in the **public** router group, alongside the SAML public routes).

- [ ] **Step 1:** Handler: read `?email=` query, `normalize_email_domain`, call `find_org_slug_by_email_domain`. On match return 200 `{org_slug, provider}`; on no match return **404 with a generic body** (no domain confirmation). Rate-limit note: rely on the existing global rate-limit layer; do not echo the input.
- [ ] **Step 2:** Register the route (public). Verify it compiles. `cargo clippy -p mockforge-registry-server ...`. Commit: "feat(registry): SSO email-domain discovery endpoint (#746)".

---

### Task 5: Add `openidconnect` + OIDC login initiation

**Files:**
- Modify: root `Cargo.toml` (workspace deps) + `crates/mockforge-registry-server/Cargo.toml` — add `openidconnect = "3"` (verify latest 3.x; features for reqwest blocking off / async on). Mirror how `oauth2` is declared.
- Create: `crates/mockforge-registry-server/src/handlers/oidc.rs` (or add to `sso.rs` — prefer a new `oidc.rs` submodule for focus; wire `mod oidc;` in `handlers/mod.rs`).
- Modify: `routes.rs` — public `GET /api/v1/sso/oidc/login/{org_slug}` and `GET /api/v1/sso/oidc/callback/{org_slug}`.

- [ ] **Step 1:** `oidc_login` handler: load org by slug → load enabled OIDC `sso_configurations` (provider="oidc") → **Team-plan gate** (mirror SAML's check) → use `openidconnect` `CoreClient` from discovery (`IssuerUrl` = `oidc_issuer_url`, `ClientId`, `ClientSecret`) → generate `state` + `nonce` (PKCE optional) → store `{state, nonce, org_slug}` in Redis with 15-min TTL (mirror `oauth.rs` CSRF storage) → 302 to the authorization URL (scopes `openid email profile`).
- [ ] **Step 2:** Build. Register routes. `cargo clippy`. Commit: "feat(registry): OIDC login initiation (#746)". (Callback in Task 6 — login alone won't compile if it references the callback; if so, stub the callback to `todo!()`-free `ApiError::InvalidRequest("not yet")` placeholder and replace in Task 6, OR implement 5+6 together in one commit.)

---

### Task 6: OIDC callback — ID-token validation + provisioning

**Files:**
- Modify: `crates/mockforge-registry-server/src/handlers/oidc.rs`.

- [ ] **Step 1 (test-first, pure validation):** Add a unit test module for a pure helper `extract_identity_from_claims(claims) -> Result<(email, Option<username>), ApiError>` (pulls `email` from standard claims; `name`/`preferred_username` for username). Test: claims with email → Ok; without email → Err.
- [ ] **Step 2:** Implement `extract_identity_from_claims`. Test PASS.
- [ ] **Step 3:** `oidc_callback` handler: read `?code`+`?state` → verify `state` against Redis (consume it) → exchange code via `openidconnect` (`exchange_code`) → **verify ID token** with the discovered JWKS + stored `nonce` (`id_token.claims(&verifier, &nonce)`) — this enforces signature, `iss`, `aud`, `exp`. → `extract_identity_from_claims` → load org by slug → `find_or_create_sso_user` → `create_access_token` (1h, mirror SAML ACS redirect token) → 302 `"{app_base_url}/auth/sso/callback?token={token}&org_slug={slug}"`. On any error → 302 `"{app_base_url}/login?sso_error={code}"` (never leak crypto detail).
- [ ] **Step 4:** `cargo clippy -p mockforge-registry-server --all-targets --all-features -- -D warnings`; `cargo test -p mockforge-registry-server --lib oidc`. Commit: "feat(registry): OIDC callback with ID-token validation + JIT provisioning (#746)".

---

### Task 7: Extend config CRUD for OIDC + `email_domain`

**Files:**
- Modify: `handlers/sso.rs` `create_sso_config` request struct + handler.

- [ ] **Step 1:** Add `email_domain: Option<String>` and the OIDC fields (`oidc_issuer_url`, `oidc_client_id`, `oidc_client_secret`) to the create/update request and persist them. Validate per provider: SAML requires entity_id+sso_url+x509_cert; OIDC requires issuer_url+client_id+client_secret. Return `InvalidRequest` listing missing fields.
- [ ] **Step 2:** `cargo test -p mockforge-registry-server --lib sso`. Commit: "feat(registry): SSO config accepts OIDC fields + email_domain (#746)".

---

### Task 8: SSO-login audit event

**Files:**
- Modify: `handlers/sso.rs` (SAML ACS success path) + `handlers/oidc.rs` (callback success path).

- [ ] **Step 1:** After successful provisioning + before redirect, call `state.store.record_audit_event(org.id, Some(user.id), AuditEventType::<SsoLogin or existing Login variant>, "SSO login via {provider}", metadata{provider}, ip, ua)`. If no suitable `AuditEventType` variant exists, add one (grep the enum in `mockforge-registry-core/src/models`).
- [ ] **Step 2:** Build + test. Commit: "feat(registry): audit successful SSO logins (#746)".

---

### Task 9: Frontend — `/auth/sso/callback` route

**Files:**
- Create: `crates/mockforge-ui/ui/src/pages/SsoCallbackPage.tsx`.
- Modify: the app router (grep `createBrowserRouter`/`<Routes>` in `crates/mockforge-ui/ui/src` — likely `App.tsx` or `main.tsx`) to add `/auth/sso/callback`.
- Modify: `stores/useAuthStore.ts` if needed to expose a "set token from external redirect" path (mirror how OAuth/login stores the token).

- [ ] **Step 1:** `SsoCallbackPage`: on mount, read `token`/`org_slug`/`sso_error` from `useSearchParams`. If `sso_error` → redirect to `/login` + toast. Else persist `token` via the auth store (same as login success), call `/users/me` to hydrate, then `navigate` into the app. Show a minimal "Signing you in…" state (product-ui loading).
- [ ] **Step 2:** Register the route. `pnpm type-check` + `pnpm lint`. Commit.

---

### Task 10: Frontend — login-page SSO entry (email→discover→slug fallback)

**Files:**
- Modify: `crates/mockforge-ui/ui/src/components/auth/LoginForm.tsx` (+ `services/authApi.ts` for the discover call).

- [ ] **Step 1:** Add a "Sign in with SSO" affordance. Click → email input → `GET /api/v1/sso/discover?email=` → 200: `window.location = "/api/v1/sso/{provider}/login/{org_slug}"`; 404: reveal an org-slug input → on submit `window.location = "/api/v1/sso/saml/login/{slug}"` (default SAML; or call discover by slug if a slug→provider endpoint is added — for now default the slug path to a `/sso/login/{slug}` resolver if one exists, else SAML). Keep it minimal and product-ui consistent (label-above, inline errors).
- [ ] **Step 2:** `pnpm type-check` + `pnpm lint`. Commit.

---

### Task 11: Frontend — SSOTab OIDC + `email_domain` fields

**Files:**
- Modify: `crates/mockforge-ui/ui/src/pages/OrganizationPage.tsx` (`SSOTab`, ~:1171; `SSOConfig` interface + `saveSSOConfig` body ~:236-247).

- [ ] **Step 1:** Add a provider toggle (SAML | OIDC). When OIDC: show issuer URL / client ID / client secret. Always show `email_domain`. Extend `SSOConfig` type + `saveSSOConfig` payload. Reuse existing form/input patterns; secrets use password inputs.
- [ ] **Step 2:** `pnpm type-check` + `pnpm lint`. Commit.

---

### Task 12: Tests — unit + mock-IdP e2e

**Files:**
- Backend unit tests live inline (already added in Tasks 2/4/6). Ensure coverage: domain normalization, discovery lookup (needs DB → put under existing DB-backed test pattern or keep pure-helper level), ID-token identity extraction.
- Create: `crates/mockforge-registry-server/tests/sso_e2e.rs` (gated/`#[ignore]` per the repo's e2e convention — see existing `*_e2e.rs`).

- [ ] **Step 1:** SAML e2e: POST a canned signed assertion to `/sso/saml/acs/{slug}` against a test config; assert a session row + 302 to `/auth/sso/callback`. (Reuse any existing SAML test fixtures; if none, document the manual checklist instead and mark the e2e `#[ignore]` with a comment — no silent gap.)
- [ ] **Step 2:** OIDC e2e: stand up a stub issuer (axum test server serving `.well-known/openid-configuration`, JWKS, and a signed ID token) OR, if that proves heavy, cover ID-token validation at unit level with a hand-built JWKS + token and document the manual Okta/Azure checklist. State explicitly which path was taken.
- [ ] **Step 3:** Run the full crate suite. Commit.

---

### Task 13: Verify, docs, close

- [ ] **Step 1:** Full verification: `cargo fmt --all --check`; `cargo clippy -p mockforge-registry-server --all-targets --all-features -- -D warnings`; `cargo test -p mockforge-registry-server`; UI `pnpm type-check` + `pnpm lint`. If `mockforge-collab` SQL changed, `make sqlx-prepare` (N/A here — different crate).
- [ ] **Step 2:** Browser-verify the SSOTab + login SSO entry render (dev server) if feasible; otherwise note pending preview-env verification with a real IdP.
- [ ] **Step 3:** Update `docs/ENVIRONMENT_VARIABLES.md` if any new env vars (none expected — OIDC config is per-org in DB, not env). Open PR; auto-merge. Comment on #746 that the login loop + OIDC are implemented; the "SSO" pricing claim is now true.

---

## Self-Review notes
- **Spec coverage:** schema (T1), discovery (T2,T4), shared provisioning (T3), OIDC login+callback+validation (T5,T6), config OIDC+domain (T7), audit (T8), frontend callback (T9), login entry (T10), SSOTab (T11), tests (T12), verify+correct-issue (T13). All spec sections mapped.
- **Risk/uncertainty (flagged, not hidden):** (a) exact `openidconnect` API surface — confirm against docs.rs for the pinned version during T5/T6. (b) mock-OIDC e2e may be heavy → documented fallback to unit + manual checklist. (c) login-page slug-fallback assumes SAML default; if mixed SAML/OIDC by slug is needed, add a `GET /sso/resolve/{slug}→provider` endpoint (small) in T10.
- **No new env vars.** **No `unsafe`.** **Team-plan gating** reused at every login-init + config-save path.
