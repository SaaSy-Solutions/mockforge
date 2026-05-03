# Cloud Showcase + Learning Hub — Design

Cloud-enablement plan for `showcase` and `learning-hub` nav items. Tracks task #12 in the cloud-enablement plan.

## Goal

Move community/learning pages from local-only to cloud as the canonical home. These aren't direct revenue features — they drive retention, activation, and discovery. Pricing dial: none directly; success metric is feature adoption among existing paid users.

## What exists

- **Pages**: `ShowcasePage`, `LearningHubPage` (local-only).
- **Cloud already**: `template-marketplace`, `scenario-marketplace`, `plugin-registry` — community-content pages that already work in cloud mode.
- **Registry routes**: zero showcase/learning routes today.

## Why these need to be cloud-native (not local)

These pages are inherently social: showcased projects, course progress, comments, likes, shared bookmarks. None of that makes sense in a local-only mode where the user is the only "user." The OSS local build can keep stub versions that link out to the cloud-hosted content, or simply hide the nav items.

## What's missing

### Showcase

A community gallery of customer-built mocks, scenarios, and integrations. Each entry has:
- Title, description, screenshots, demo URL (often a hosted-mock).
- Author (org + user).
- Tags (industry, protocol, pattern).
- Likes/saves count.
- Optional source-of-truth link (GitHub, blog post).
- Featured flag (curated by MockForge team).

### Learning Hub

A structured learning surface:
- Tutorials: ordered lesson sequences with code snippets and embedded mocks.
- Recipes: short, copyable patterns ("how to mock a paginated endpoint with random delay").
- Per-user progress tracking (lessons completed, badges earned).
- Search across both.

Both pages are mostly content-management with light social features. Building them from scratch would be heavy; the right move is to keep the data model lean and lean on existing crates where possible.

## Cloud architecture

### Proposed routes

```
# Showcase
GET    /api/v1/showcase/entries                                # list with filters
GET    /api/v1/showcase/entries/{slug}
POST   /api/v1/showcase/entries                                 # submit (auth required)
PATCH  /api/v1/showcase/entries/{id}                            # author edits
DELETE /api/v1/showcase/entries/{id}                            # author deletes
POST   /api/v1/showcase/entries/{id}/like
DELETE /api/v1/showcase/entries/{id}/like
POST   /api/v1/admin/showcase/entries/{id}/feature              # MockForge team only

# Learning Hub
GET    /api/v1/learning/tracks                                  # list of tutorials
GET    /api/v1/learning/tracks/{slug}                           # full tutorial with lessons
GET    /api/v1/learning/recipes
GET    /api/v1/learning/recipes/{slug}
POST   /api/v1/learning/progress                                # mark lesson complete
GET    /api/v1/learning/progress                                # current user's progress
POST   /api/v1/admin/learning/tracks                            # admin authoring
POST   /api/v1/admin/learning/recipes
```

## Data model

```sql
CREATE TABLE showcase_entries (
    id UUID PRIMARY KEY,
    slug TEXT UNIQUE NOT NULL,
    org_id UUID REFERENCES organizations(id),
    submitted_by UUID REFERENCES users(id),
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    body TEXT,                                  -- markdown
    screenshots TEXT[],                         -- blob URLs
    demo_url TEXT,
    source_url TEXT,
    tags TEXT[],
    is_featured BOOLEAN NOT NULL DEFAULT FALSE,
    is_published BOOLEAN NOT NULL DEFAULT FALSE,
    likes_count INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE showcase_likes (
    user_id UUID NOT NULL REFERENCES users(id),
    entry_id UUID NOT NULL REFERENCES showcase_entries(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, entry_id)
);

CREATE TABLE learning_tracks (
    id UUID PRIMARY KEY,
    slug TEXT UNIQUE NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    body TEXT,                                   -- track-level intro markdown
    is_published BOOLEAN NOT NULL DEFAULT FALSE,
    sort_order INT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE learning_lessons (
    id UUID PRIMARY KEY,
    track_id UUID NOT NULL REFERENCES learning_tracks(id) ON DELETE CASCADE,
    slug TEXT NOT NULL,
    title TEXT NOT NULL,
    body TEXT NOT NULL,                          -- markdown with code snippets
    sort_order INT NOT NULL,
    UNIQUE (track_id, slug)
);

CREATE TABLE learning_recipes (
    id UUID PRIMARY KEY,
    slug TEXT UNIQUE NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    body TEXT NOT NULL,
    tags TEXT[],
    is_published BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE learning_progress (
    user_id UUID NOT NULL REFERENCES users(id),
    lesson_id UUID NOT NULL REFERENCES learning_lessons(id) ON DELETE CASCADE,
    completed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, lesson_id)
);
```

## Plan tiers

These are free for everyone, including unauthenticated users for read access. No tier gating — community features need maximum reach.

Authoring (submitting showcase entries) is gated to authenticated users only, regardless of plan.

## UI changes

1. `AppShell.tsx:217` — add `'showcase'`, `'learning-hub'` to `cloudNavItemIds`.
2. **ShowcasePage rewrite**:
   - Card grid with tag filters and featured/most-liked sort.
   - Detail view: full body, screenshots carousel, like button, demo-mock launcher.
   - "Submit yours" CTA → modal form with screenshot upload, demo URL field.
3. **LearningHubPage rewrite**:
   - Track grid (e.g., "Mocking Basics," "Advanced Scenarios," "AI Studio Workflow").
   - Track detail = ordered lesson list with progress checks.
   - Embedded mock previews (live runs against demo workspace).
   - Recipes search + tag filter as a sibling tab.

## Effort estimate

| Phase | Scope | Estimate |
|-------|-------|----------|
| 1 | Schema migration | ~1 day |
| 2 | Showcase CRUD + likes + featuring | ~1.5 days |
| 3 | Learning tracks/lessons/recipes CRUD + progress tracking | ~1.5 days |
| 4 | Admin authoring tools (re-use admin guards) | ~1 day |
| 5 | UI rewrites for both pages | ~3 days |
| 6 | Initial content seed (5 tracks, 20 recipes, 10 showcase entries) | ~2 days |
| 7 | E2E (submit → moderate → publish → like → progress) | ~1 day |

Total: ~11 working days for v1.

## Decisions

### No tier gating, no paywalls

**Decision: free for everyone, including read access without auth.** Discovery is the whole point; gating it kills the funnel. Login required only for authoring/likes/progress.

### Markdown bodies, not WYSIWYG

**Decision: markdown.** Simpler authoring, simpler rendering, well-understood by the developer audience. Render with the same library used elsewhere in the UI.

### Initial content matters more than the platform

**Decision: invest 2 days in seed content as part of v1.** A community gallery with 0 entries fails to launch. Same for learning hub. Pre-write tracks for: AI Studio (#1), Hosted mocks, Workspace federation, Chaos campaigns (#7), Test execution (#4).

## Out of scope for v1

- Comments / threaded discussion on showcase entries (link out to GitHub Discussions for now).
- User profiles / public author pages.
- Course completion certificates.
- Premium / paid courses.
- Embedded interactive code playground (link to web playground if it exists; otherwise plain code blocks).

## Open questions

1. Should authored showcase entries appear in `template-marketplace` automatically? Probably no — different curation models. But cross-link them.
2. Learning progress tied to user or to org? Recommend user (each individual learns at their own pace), but display org-wide adoption as an insight on the org-admin dashboard.
3. Moderation: do we pre-approve submissions or post-publish? Pre-approve for v1 (lower abuse risk), shift to post-publish + flagging if volume warrants it.
