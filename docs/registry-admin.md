# Registry Admin (OSS Single-Tenant)

The **Registry Admin** is the OSS counterpart to the multi-tenant SaaS
`mockforge-registry-server` binary. It exposes the same
`RegistryStore` trait surface — users, organizations, org members, API
tokens, audit logs — against a local SQLite database, embedded directly
in the `mockforge` admin UI.

Everything is shared code:

```
crates/mockforge-registry-core/       ← shared models, store trait, auth helpers
├── store/
│   ├── mod.rs                        ← RegistryStore trait (151 methods)
│   ├── postgres.rs                   ← PgRegistryStore (SaaS binary)
│   └── sqlite.rs                     ← SqliteRegistryStore (this doc)
└── migrations-sqlite/                ← 10-table OSS schema
```

## Enabling the registry admin

The registry admin is opt-in. Set the following environment variable
before starting the admin UI:

```bash
export MOCKFORGE_REGISTRY_DB_URL=sqlite://./mockforge-admin.db
```

URLs:

- `sqlite::memory:` — in-process only, discarded when the server stops.
  Useful for ephemeral CI.
- `sqlite://./mockforge-admin.db` — on-disk file in the current working
  directory.
- `sqlite:///var/lib/mockforge/admin.db` — absolute path.

When the variable is unset, the admin UI behaves exactly as before — no
new routes are mounted, no database file is touched, and the
`registry-admin` cargo feature compiles to a no-op.

## First-run bootstrap

The first time you enable the registry admin, there are zero users in
the database, so there's nobody to log in as. You have two options:

**Option 1: environment-variable bootstrap** (recommended)

```bash
export MOCKFORGE_REGISTRY_DB_URL=sqlite://./mockforge-admin.db
export MOCKFORGE_ADMIN_USERNAME=admin
export MOCKFORGE_ADMIN_EMAIL=admin@example.com
export MOCKFORGE_ADMIN_PASSWORD='pick a strong one'
export MOCKFORGE_ADMIN_JWT_SECRET=$(openssl rand -hex 32)
mockforge serve --admin --admin-port 9080 --spec ./openapi.json
```

On first startup, a verified admin user is created and a log line is
emitted:

```
INFO Bootstrapped admin user 'admin' (email: admin@example.com) from MOCKFORGE_ADMIN_* env vars
```

Subsequent startups skip the bootstrap if the user already exists.

**Option 2: HTTP register** (no env vars)

```bash
curl -X POST http://localhost:9080/api/admin/registry/auth/register \
  -H 'content-type: application/json' \
  -d '{"username":"admin","email":"admin@example.com","password":"pick a strong one"}'
```

Returns `{user, token}` — save the token.

> **Warning**: if you leave `MOCKFORGE_ADMIN_JWT_SECRET` unset, a loud
> warning appears in the log and tokens are signed with an empty secret.
> This is fine for local testing but **NOT** safe for production.

## Signing in via the UI

1. Navigate to `http://localhost:9080/registry-login` in your browser.
2. The page auto-probes `/api/admin/registry/health`; if the backend
   isn't enabled, you'll see a banner explaining how to turn it on.
3. Enter your username (or email) and password, hit **Sign in**.
4. You'll land on `/registry-admin` with three tabs:
   - **Signed in as**: your current user record.
   - **Look up**: find user by email, find org by slug.
   - **Create**: create an organization, then generate invitation
     links for it.

JWTs are stored in `localStorage` under
`mockforge_registry_admin_token`. Logging out clears this key and
returns you to the login page.

## HTTP API reference

All endpoints live under `/api/admin/registry/*`.

### Health

```
GET /api/admin/registry/health
→ { "status": "ok" }
```

### Auth

```
POST /api/admin/registry/auth/register   { username, email, password }
POST /api/admin/registry/auth/login      { identifier, password }   # identifier = username | email
GET  /api/admin/registry/auth/me         # requires Authorization: Bearer <jwt>
```

The login endpoint returns the same 401 "invalid credentials" response
for both "user not found" and "wrong password" — we don't leak
account existence. Both register and login return the same
`{ user, token }` shape, so the caller can assume successful auth
after either.

### Users

```
POST /api/admin/registry/users                              # admin create, expects already-hashed password
GET  /api/admin/registry/users/email/{email}
GET  /api/admin/registry/users/username/{username}
POST /api/admin/registry/users/{id}/verify
```

`POST /users` is distinct from `POST /auth/register`: the former
accepts a pre-computed bcrypt hash (for integrations with external
hashing systems), the latter bcrypts plaintext server-side.

### Organizations

```
POST /api/admin/registry/orgs                               # { name, slug, owner_id, plan }
GET  /api/admin/registry/orgs/slug/{slug}
```

Plans: `free`, `pro`, `team`. Unknown plans return 400.

### Org members (teams)

```
GET    /api/admin/registry/orgs/{org_id}/members
POST   /api/admin/registry/orgs/{org_id}/members            # { user_id, role }
PATCH  /api/admin/registry/orgs/{org_id}/members/{user_id}  # { role }
DELETE /api/admin/registry/orgs/{org_id}/members/{user_id}
```

Roles: `owner`, `admin`, `member`. Unknown roles return 400.

### Org quota

```
GET /api/admin/registry/orgs/{org_id}/quota
PUT /api/admin/registry/orgs/{org_id}/quota                # arbitrary JSON object
```

The quota body is stored under the reserved `org_settings[quota]` key
and can hold any JSON object. Typical shape:

```json
{
  "max_tokens": 10,
  "max_mocks": 100,
  "max_requests_per_minute": 600
}
```

`PUT /quota` rejects non-object bodies (arrays, strings, numbers) with
400.

### API tokens

```
POST /api/admin/registry/orgs/{org_id}/tokens              # { name, user_id?, scopes }
```

Scopes: `read:packages`, `publish:packages`, `deploy:mocks`,
`admin:org`, `read:usage`, `manage:billing`. The plaintext token is
returned **once** in the response — clients must save it immediately.

```json
{
  "token": "mfx_AbCdEfGhIjKlMnOpQrStUvWxYz1234567890==",
  "token_prefix": "mfx_AbCdEfGhIj",
  "name": "ci-token",
  "scopes": ["read:packages", "publish:packages"],
  ...
}
```

### Invitations

```
POST /api/admin/registry/orgs/{org_id}/invitations         # { email, role }
GET  /api/admin/registry/invitations/{token}
POST /api/admin/registry/invitations/{token}/accept        # { username, password }
```

The invitation `token` is a JSON-encoded payload containing a random
32-byte nonce, so it's unguessable. The nonce is also persisted
server-side (under `org_settings[invite:{nonce}]`) so forged tokens
fail the `get`/`accept` validation.

Accepting an invitation:
1. Creates the user account (rejects duplicate username/email with 409).
2. Marks them as `is_verified = true` (the invitee already owns the
   mailbox and consented to join by having the link).
3. Adds them to the org with the invited role.
4. Deletes the invitation (single use).
5. Returns a fresh JWT so the new user is logged in immediately.

## Backend vs OSS — feature matrix

| Feature                         | SaaS `mockforge-registry-server` | OSS registry admin |
|---------------------------------|:-:|:-:|
| Users, orgs, members            | ✅ | ✅ |
| API tokens + scopes             | ✅ | ✅ |
| Audit logging                   | ✅ | ✅ |
| Invitations                     | ✅ | ✅ |
| Org quota                       | ✅ | ✅ |
| Email verification              | ✅ | ✅ |
| Plugin / template / scenario marketplace | ✅ | — |
| Reviews                         | ✅ | — |
| SSO / SAML                      | ✅ | — |
| Stripe billing                  | ✅ | — |
| Hosted mocks (Fly.io orchestration) | ✅ | — |
| Federations                     | ✅ | — |
| Cloud workspaces                | ✅ | — |

The OSS registry admin intentionally returns empty lists / `None` for
the marketplace and SaaS-only trait methods — the handlers that consume
them are cfg-gated out of the `mockforge-ui` build anyway. If you need
any of the right-hand features, run the multi-tenant
`mockforge-registry-server` binary instead.

## Testing

The backend is exercised end-to-end by 29 integration tests in
`crates/mockforge-ui/src/registry_admin.rs` that spin up an in-memory
SQLite store and drive the axum router via
`tower::ServiceExt::oneshot` — no live HTTP server required.

```bash
cargo test -p mockforge-ui --lib registry_admin
```

The SQLite store itself has 14 integration tests in
`crates/mockforge-registry-core/src/store/sqlite.rs` covering user CRUD,
API token lifecycle (create/verify/list/delete), org + member CRUD,
quota get/set, audit logging, verification tokens, and the zeroed
analytics snapshot defaults:

```bash
cargo test -p mockforge-registry-core --features "postgres sqlite" --lib store::sqlite
```

The frontend has smoke tests under
`crates/mockforge-ui/ui/e2e/registry-admin.spec.ts` that verify the
login page renders, the backend-unavailable banner works, 401 errors
surface cleanly, and `/registry-admin` redirects to `/registry-login`
when unauthenticated:

```bash
pnpm --dir crates/mockforge-ui/ui test:e2e -- registry-admin
```
