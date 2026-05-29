# Plugin Marketplace — Production Architecture

**Status**: ✅ Implemented (Rust, in `mockforge-registry-server`)

> **History / correction (2026-05):** earlier revisions of this document described a
> TypeScript/Express/Prisma backend under `plugin-marketplace/backend/src/*.ts`. **That
> backend never existed.** The marketplace is implemented in Rust as part of the
> multi-tenant registry server (`crates/mockforge-registry-server`). This document was
> rewritten to describe the actual implementation. See issue #667.

## Overview

The plugin / template / scenario marketplace is served by `mockforge-registry-server`
(the SaaS registry, deployed on Fly as the `app.mockforge.dev` / registry API). It is a
cloud capability: the publish/search/install/review endpoints live **only** on the
registry server, not on the embedded admin server that ships with the local
`mockforge serve --admin` binary.

The admin UI (`crates/mockforge-ui/ui`) talks to these endpoints over `/api/v1/...` when
running in cloud mode (`VITE_MOCKFORGE_MODE=cloud`); requests carry the JWT via
`authenticatedFetch` (`src/utils/apiClient.ts`).

## Endpoints

| Endpoint | Handler |
|----------|---------|
| `POST /api/v1/plugins/publish` | `handlers::plugins::publish_plugin` |
| `GET  /api/v1/plugins/search` (and details/versions) | `handlers::plugins` |
| `POST /api/v1/marketplace/templates/publish` | `handlers::templates::publish_template` |
| `POST /api/v1/marketplace/templates/search` | `handlers::templates::search_templates` |
| `POST /api/v1/marketplace/scenarios/publish` | `handlers::scenarios::publish_scenario` |
| `POST /api/v1/marketplace/scenarios/search` | `handlers::scenarios` |

Routes are registered in `crates/mockforge-registry-server/src/routes.rs`.

## Implemented capabilities

### 1. Publishing (`handlers/plugins.rs`, `handlers/templates.rs`, `handlers/scenarios.rs`)

- **Auth + scopes**: `AuthUser` extractor + `ScopedAuth::require_scope(TokenScope::PublishPackages)`
  for plugins; org-context resolution (`resolve_org_context`) for templates/scenarios.
- **Input validation** (`src/validation.rs`): name, semver version, checksum format, base64
  payload, and WASM-file validation for plugins.
- **Integrity**: SHA-256 of the uploaded bytes is recomputed server-side and compared to the
  client-supplied checksum; mismatch is rejected.
- **Per-plan limits**: org `limits_json` enforces `max_templates_published`,
  `max_scenarios_published`, and a `storage_gb` quota tracked via `UsageCounter`.
- **Versioning**: `create_plugin_version` records each version with its download URL,
  checksum, file size, and optional SBOM; supports update-in-place of an existing package.

### 2. Binary storage (`src/storage.rs`)

`PluginStorage` is S3-compatible with a local-filesystem fallback:

- **S3 backend** via `aws-sdk-s3`. Honors a custom endpoint (`S3_ENDPOINT` / `AWS_ENDPOINT_URL_S3`)
  with explicit credentials, or the default AWS credential-provider chain. Performs a
  connectivity/health check on startup.
- **Local fallback**: if no usable bucket is configured or the S3 health check fails, falls
  back to a local directory (`STORAGE_PATH`, default `./data/storage`). Used for dev/test.
- Key components are sanitized before use in S3 keys or file paths.

Relevant env: `S3_BUCKET` / `BUCKET_NAME`, `S3_REGION` / `AWS_REGION`, `S3_ENDPOINT`,
`AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`.

### 3. Rate limiting (`src/middleware/rate_limit.rs`, `org_rate_limit.rs`)

Per-IP and per-org rate limiting middleware, with optional Redis (`REDIS_URL`) for shared
state across instances and an in-memory fallback. Default per-minute budget is configurable
via `RATE_LIMIT_PER_MINUTE`.

### 4. Reviews & moderation (`handlers/reviews.rs`, `handlers/template_reviews.rs`, plugin moderation)

- Submit / vote / respond to reviews; live star counts via the `template_stars` table.
- Plugin moderation surface (verify / takedown / restore) used by the admin
  `PluginModerationPage`.

## Configuration (env)

Required:

```bash
DATABASE_URL=postgres://...        # Postgres; server auto-runs migrations on startup
JWT_SECRET=...                     # JWT signing secret (>= 32 chars)
```

Common optional:

```bash
PORT=8080                          # default 8080
STORAGE_PATH=./data/storage        # local storage dir when S3 is not configured
S3_BUCKET=mockforge-plugins        # enables S3 backend
AWS_REGION=us-east-1
REDIS_URL=redis://localhost:6379   # enables Redis-backed rate limiting
MAX_PLUGIN_SIZE=52428800           # 50 MiB
RATE_LIMIT_PER_MINUTE=60
PROMETHEUS_SCRAPE_TOKEN=...         # require bearer auth on /metrics (issue #647)
```

## Testing

End-to-end coverage lives in
`crates/mockforge-registry-server/tests/marketplace_e2e.rs` (register → create org →
publish → search → install → review for plugins, templates, and scenarios). These tests
are `#[ignore]`d because they require a running registry + Postgres:

```bash
# bring up Postgres + registry-server with DATABASE_URL/JWT_SECRET, then:
REGISTRY_URL=http://localhost:8080 \
  cargo test -p mockforge-registry-server --test marketplace_e2e -- --ignored
```

> **Known issue:** template/scenario *search* currently does not return a just-published
> org-scoped item even though the row persists correctly — tracked separately. Plugin
> publish/search/install/review passes end-to-end.

## Local vs. cloud

The marketplace is a registry (cloud) feature. In local self-hosted mode
(`mockforge serve --admin`) the embedded admin server does not host these `/api/v1/...`
marketplace routes; the admin UI surfaces publishing only when authenticated against the
registry. There is no local marketplace backend, by design — a single shared registry is
the source of truth for published artifacts.
