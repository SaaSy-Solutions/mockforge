# SSO Completion — Design Spec (#746)

**Date:** 2026-05-30
**Issue:** [#746](https://github.com/SaaSy-Solutions/mockforge/issues/746)
**Status:** Design approved; implementation pending.

## Context & correction

Issue #746 as filed claims "Team tier markets SSO but no SSO is implemented." **That is wrong.** A full SAML 2.0 SSO backend and an admin config UI already exist and ship today:

- **SAML backend** — `crates/mockforge-registry-server/src/handlers/sso.rs` (~1278 lines): SP metadata, AuthnRequest/login initiation, ACS (assertion consumer), SLO, RSA signature verification via `ring`, replay protection (`saml_assertion_ids`), `NotBefore`/`NotOnOrAfter` validation, email-based account linking, 8-hour SSO sessions.
- **Config + schema** — `sso_configurations` and `sso_sessions` tables (per-org, **Team-plan gated**); model `crates/mockforge-registry-core/src/models/sso.rs` carries both SAML and OIDC fields plus an `attribute_mapping` JSON.
- **Admin config UI** — `crates/mockforge-ui/ui/src/pages/OrganizationPage.tsx` `SSOTab` (≈ line 1171): set IdP entity ID / SSO URL / SLO URL / X.509 cert, enable/disable/delete.

**The real gaps:**
1. The SAML **login loop is not closed in the UI** — no "Sign in with SSO" entry on the login page, and no `/auth/sso/callback` route to consume the token the backend redirects with (`/auth/sso/callback?token=…&org_slug=…`). End users can configure SAML but cannot log in through it.
2. **OIDC is unimplemented** — config fields (`oidc_issuer_url`, `oidc_client_id`, `oidc_client_secret`) exist but only SAML has handlers.
3. **No end-to-end verification** — with no UI entry point, the SAML flow has likely never run against a real IdP.

This spec covers **Scope B**: close the SAML login loop **and** add OIDC, with **email-then-slug** discovery.

## Goals / non-goals

**Goals**
- A user whose org has SSO configured can log in via SAML **or** OIDC end-to-end from the product.
- Org admins can configure either protocol (incl. an email domain for discovery) from the existing `SSOTab`.
- The flow is verified (unit + a mock-IdP e2e).

**Non-goals (YAGNI)**
- SCIM / directory sync, multiple SSO configs per org, IdP-initiated-only flows beyond what SAML already supports, SLO for OIDC (logout stays local + SAML SLO as today).
- Custom per-attribute role mapping beyond the existing `attribute_mapping` JSON (JIT users are always added as `Member`, matching current SAML behavior).

## Architecture

One `sso_configurations` row per org, `provider ∈ {saml, oidc}`, Team-gated. OIDC mirrors the proven SAML pipeline so both protocols converge on one frontend callback:

```
login page
  → enter work email → GET /api/v1/sso/discover?email=…
       ├ domain match → { org_slug, provider }
       └ no match     → prompt org slug
  → redirect GET /api/v1/sso/{saml|oidc}/login/{org_slug}
  → IdP authenticates
  → backend ACS (SAML) / callback (OIDC):
       verify identity → find_or_create_sso_user(email, org) → issue MockForge JWT
  → 302 /auth/sso/callback?token=…&org_slug=…
  → frontend stores token, hydrates /users/me, enters app
```

## Components

### 1. Schema — one migration
Add `email_domain VARCHAR` (nullable) to `sso_configurations`, with an index for domain→org lookup. Pick the next non-colliding migration prefix **at implementation time** (match against `gh pr list` + the migration-guard hook; do not assume a "free" prefix from main — concurrent PRs race).

### 2. Backend — shared logic
- Generalize `find_or_create_user_from_saml` → `find_or_create_sso_user(state, email, username, org)`, used by both protocols. Identical JIT behavior: link by email if the user exists (ensure Member of the org); else create + mark verified + add as `Member`.
- `GET /api/v1/sso/discover?email=…` (public, pre-auth): map the email domain to `{ org_slug, provider }` or 404. Enumeration-resistant — reveal nothing beyond what the redirect needs; rate-limit.
- Emit an **audit event on successful SSO login** (today only config changes are audited).

### 3. Backend — OIDC handlers (mirror SAML routes)
- `GET /api/v1/sso/oidc/login/{org_slug}` — OIDC discovery from `oidc_issuer_url` (`.well-known/openid-configuration`), build authorization-code URL (scopes `openid email profile`), store CSRF `state` + `nonce` in Redis (15-min TTL, reuse the OAuth pattern), redirect to the IdP.
- `GET /api/v1/sso/oidc/callback/{org_slug}` — verify `state`, exchange `code`, **validate the ID token** (JWKS signature from discovery, `iss`/`aud`/`exp`/`nonce`), extract email + name → `find_or_create_sso_user` → issue MockForge JWT → 302 `/auth/sso/callback?token=…`.
- **Dependency:** add the `openidconnect` crate. Hand-rolling OIDC discovery/JWKS/ID-token validation for an auth path is error-prone; this is the standard, audited choice. Accept the compile-time cost (workspace dep).
- Extend `create_sso_config` to accept OIDC fields + `email_domain`; validate required fields per provider.

### 4. Frontend
- **Login page** (`components/auth/…`): "Sign in with SSO" → work-email prompt → `GET /sso/discover` → matched: `window.location = /api/v1/sso/{provider}/login/{slug}`; unmatched: prompt org slug → same redirect.
- **New route `/auth/sso/callback`**: read `token` (+ `org_slug` / `error`) from the query string, persist via the existing `useAuthStore` (same path as OAuth/login token handling), hydrate `/users/me`, route into the app; on `error`, redirect to login with a toast.
- **`SSOTab`** (`OrganizationPage.tsx`): add a SAML/OIDC provider toggle, OIDC fields (issuer URL, client id, client secret), and the `email_domain` field — reuse existing form patterns (product-ui conventions: label-above, zod, inline errors).

## Data model deltas
- `sso_configurations.email_domain VARCHAR NULL` (indexed). All other OIDC fields already exist.
- No change to `sso_sessions` or `User`.

## Error handling
Invalid/expired/unsigned assertions, ID-token validation failure, unknown org/domain, disabled SSO, non-Team plan, IdP-returned errors → redirect to `/login?sso_error=…` with a user-readable toast; never leak raw IdP/crypto errors. JIT-provisioning requires an email claim; missing email → explicit error.

## Security
- SAML: existing replay protection (`saml_assertion_ids`), signature + timestamp validation.
- OIDC: CSRF `state` + `nonce`, JWKS signature validation, `iss`/`aud`/`exp` checks.
- Team-plan gating enforced at both config-save and login-initiation.
- Discovery endpoint is enumeration-resistant and rate-limited.
- Audit every successful SSO login (org, user, provider, IP/UA).

## Testing / verification
- **Unit:** discovery domain→org lookup; OIDC ID-token validation against a test JWKS (valid, wrong-aud, expired, bad-nonce, bad-sig); `find_or_create_sso_user` (link vs create paths).
- **E2e (mock IdP):** SAML — POST a canned signed assertion to ACS and assert a session + redirect; OIDC — stand up a stub issuer (discovery + JWKS + token) and drive the callback. This is the highest-effort piece; if the mock-OIDC harness proves heavy, fall back to thorough unit coverage of validation + a documented manual checklist against an Okta/Azure dev tenant, and say so explicitly (no silent gap).

## Build order
1. Migration: `email_domain`.
2. Shared `find_or_create_sso_user` + `/sso/discover`.
3. OIDC `login` + `callback` (+ `openidconnect` dep).
4. Extend config-save for OIDC + `email_domain`.
5. Frontend: login SSO entry + `/auth/sso/callback`.
6. Frontend: `SSOTab` OIDC + domain fields.
7. Tests: unit + mock-IdP e2e.
8. Correct issue #746 (done as part of this design).

## Effort
Medium. Backend OIDC + shared/discovery is the bulk; frontend login-loop is small-medium; the mock-IdP e2e is the swing factor.
