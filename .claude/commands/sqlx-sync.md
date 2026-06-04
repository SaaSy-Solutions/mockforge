---
allowed-tools: Bash, Read, Grep
description: Regenerate the SQLx offline query cache after changing mockforge-collab queries
---

# /sqlx-sync — Regenerate SQLx Query Cache

`mockforge-collab` uses SQLx in offline mode. After editing any `sqlx::query!`
/ `query_as!` macro, the cached `.sqlx/` query metadata goes stale and CI (and
offline builds) fail with "query not found in cache". This regenerates it.

## Steps

1. Confirm collab queries actually changed:
   ```bash
   git diff --name-only HEAD | grep -E 'crates/mockforge-collab/' || echo "no collab changes detected"
   ```
2. Regenerate the cache:
   ```bash
   make sqlx-prepare
   ```
   (Equivalent to `cargo sqlx prepare` in `crates/mockforge-collab` with the
   workspace `DATABASE_URL` / offline settings.)
3. Stage the regenerated cache so it lands in the same commit as the query change:
   ```bash
   git add crates/mockforge-collab/.sqlx
   git status --short crates/mockforge-collab/.sqlx
   ```
4. Confirm the crate still builds offline:
   ```bash
   SQLX_OFFLINE=true cargo build -p mockforge-collab
   ```

## Rules
- Always commit the regenerated `.sqlx/` cache WITH the query change, never separately.
- If `make sqlx-prepare` needs a live DB, ensure the dev Postgres is up first
  (see the registry/collab local recipes in memory).
