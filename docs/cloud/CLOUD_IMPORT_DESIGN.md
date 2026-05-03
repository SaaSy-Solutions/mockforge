# Cloud Import — Design

Cloud-enablement plan for the `import` nav item. Tracks task #13 in the cloud-enablement plan.

## Goal

Move the `import` page from local-only to cloud. This is the smallest task in the plan because **the cloud routes already exist** (`/api/v1/import/preview`, `/api/v1/workspaces/{id}/import`, `/api/v1/workspaces/{id}/autocomplete`). The work is purely UI wiring.

## What exists

- **Page**: `ImportPage.tsx` (currently local-only, hits `/__mockforge/import/*`).
- **Cloud handlers** in `mockforge-registry-server::handlers::workspace_import`:
  - `POST /api/v1/import/preview` — preview an OpenAPI spec / collection / HAR before importing.
  - `POST /api/v1/workspaces/{workspace_id}/import` — actually import into a workspace.
  - `POST /api/v1/workspaces/{workspace_id}/autocomplete` — schema-aware autocomplete during edit.
- These are already used by other parts of the cloud UI — the `import` *page* just hasn't been wired yet.

## What's missing

1. **Nav allowlist.** Add `'import'` to `cloudNavItemIds` in `AppShell.tsx:217`.
2. **UI cloud-mode wiring.** Switch `ImportPage.tsx` to call the cloud routes when `isCloudMode()` is true.
3. **Workspace selector.** Cloud import requires a target `workspace_id`; the local version doesn't. Add a workspace dropdown at the top of the page in cloud mode.
4. **File upload to cloud.** Today the local page reads from a local file path. Cloud needs an actual file upload — multipart POST with the spec content in the body.
5. **Preview-then-import flow.** Use `/preview` to show what will be imported (count of routes, fixtures, etc.), then call `/import` on confirmation. The local UI already has this shape; reuse it.

## Plan tiers

No special gating. Import is part of every plan, including Free. Quotas apply to what's being imported into (workspace count, fixture count) — those are pre-existing plan limits, not import-specific.

## UI changes

1. `AppShell.tsx:217` — add `'import'` to `cloudNavItemIds`.
2. **ImportPage in cloud mode**:
   - Workspace selector at top.
   - File upload widget (drag-drop or browse) instead of file-path input.
   - Source-format detector: OpenAPI 3.x, Postman collection, HAR, Insomnia.
   - Preview pane: routes/fixtures/auth schemes that will be imported.
   - Conflict resolution: skip / overwrite / suffix on name collision.
   - Confirm button → `POST /api/v1/workspaces/{id}/import`.
3. **Import history**: small list at the bottom of the page showing recent imports (workspace, source, count, when, by whom). Optional v1.

## Effort estimate

| Phase | Scope | Estimate |
|-------|-------|----------|
| 1 | Add `'import'` to cloudNavItemIds | ~5 min |
| 2 | Workspace selector wiring | ~1 hour |
| 3 | File upload widget + multipart POST | ~2 hours |
| 4 | Preview/confirm flow against cloud endpoints | ~3 hours |
| 5 | Conflict resolution UI | ~2 hours |
| 6 | Import history display (optional) | ~2 hours |
| 7 | E2E (upload → preview → import → verify in workspace) | ~2 hours |

Total: **~1.5 working days for v1.** Second-smallest task in the plan after #11.

## Decisions

### No new backend work

**Decision: do not add new handlers.** The existing `workspace_import::*` endpoints already cover preview, import, and autocomplete. Adding new endpoints would be redundant. If features like batch-import or scheduled import come up, design those separately.

### Multipart upload, not URL-based

**Decision: multipart POST.** The local version optionally accepts a URL; cloud should support both, but multipart is the primary path because most users have a local file. URL-based import requires the cloud to fetch arbitrary URLs (egress + SSRF concerns).

### Cloud import does not support importing OSS plugins

**Decision: out of scope.** Plugins go through `plugin-registry` (already cloud). Don't double-up.

## Out of scope for v1

- Bulk import (multiple specs in one run).
- Scheduled re-import (re-pull from a watched URL).
- Git-watched specs (auto-import on PR merge).
- Import from CI artifacts.
- Two-way sync (export back to source format).

## Open questions

1. Should cloud import support fetching by URL? Yes for public URLs; SSRF protection lives in the registry server's HTTP client. Recommend allowing `https://` URLs only, with a denylist for private IP ranges.
2. Conflict resolution defaults: skip vs. overwrite. Recommend skip-by-default with a per-row override in the preview UI.
3. File size cap? Cloud should reject specs over ~5 MB to prevent abuse. Postman collections can get large but rarely above that.
