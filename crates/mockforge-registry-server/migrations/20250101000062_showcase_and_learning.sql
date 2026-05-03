-- Showcase + Learning Hub (cloud-enablement task #12 / Phase 1).
--
-- Community/retention features. Free for everyone — read access works
-- without auth, authoring/likes/progress require login. Schema-first
-- slice; handlers + UI rewrite land in follow-up slices.
--
-- See docs/cloud/CLOUD_SHOWCASE_LEARNING_DESIGN.md.

-- ===== Showcase ============================================================

CREATE TABLE IF NOT EXISTS showcase_entries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug TEXT NOT NULL UNIQUE,
    org_id UUID REFERENCES organizations(id) ON DELETE SET NULL,
    submitted_by UUID REFERENCES users(id) ON DELETE SET NULL,
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    body TEXT,                                  -- markdown
    screenshots TEXT[] NOT NULL DEFAULT '{}',
    demo_url TEXT,
    source_url TEXT,
    tags TEXT[] NOT NULL DEFAULT '{}',
    is_featured BOOLEAN NOT NULL DEFAULT FALSE,
    is_published BOOLEAN NOT NULL DEFAULT FALSE,
    likes_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
-- Public-facing list views: published only, optionally featured-first.
CREATE INDEX IF NOT EXISTS idx_showcase_published
    ON showcase_entries(is_published, is_featured DESC, likes_count DESC, created_at DESC)
    WHERE is_published = TRUE;
-- Tag search.
CREATE INDEX IF NOT EXISTS idx_showcase_tags
    ON showcase_entries USING GIN (tags);

CREATE TABLE IF NOT EXISTS showcase_likes (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    entry_id UUID NOT NULL REFERENCES showcase_entries(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, entry_id)
);
-- Lookup likes-by-user for the "things I've liked" view.
CREATE INDEX IF NOT EXISTS idx_showcase_likes_user
    ON showcase_likes(user_id, created_at DESC);

-- ===== Learning Hub ========================================================

CREATE TABLE IF NOT EXISTS learning_tracks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    description TEXT,
    body TEXT,                                  -- markdown intro
    is_published BOOLEAN NOT NULL DEFAULT FALSE,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_learning_tracks_published
    ON learning_tracks(is_published, sort_order)
    WHERE is_published = TRUE;

CREATE TABLE IF NOT EXISTS learning_lessons (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    track_id UUID NOT NULL REFERENCES learning_tracks(id) ON DELETE CASCADE,
    slug TEXT NOT NULL,
    title TEXT NOT NULL,
    body TEXT NOT NULL,                         -- markdown with code snippets
    sort_order INTEGER NOT NULL DEFAULT 0,
    UNIQUE (track_id, slug)
);
CREATE INDEX IF NOT EXISTS idx_learning_lessons_track
    ON learning_lessons(track_id, sort_order);

CREATE TABLE IF NOT EXISTS learning_recipes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    description TEXT,
    body TEXT NOT NULL,
    tags TEXT[] NOT NULL DEFAULT '{}',
    is_published BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_learning_recipes_published
    ON learning_recipes(is_published, created_at DESC)
    WHERE is_published = TRUE;
CREATE INDEX IF NOT EXISTS idx_learning_recipes_tags
    ON learning_recipes USING GIN (tags);

-- Per-user lesson completion tracking. The (user_id, lesson_id) composite
-- key is the natural identity; no surrogate id needed.
CREATE TABLE IF NOT EXISTS learning_progress (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    lesson_id UUID NOT NULL REFERENCES learning_lessons(id) ON DELETE CASCADE,
    completed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, lesson_id)
);
CREATE INDEX IF NOT EXISTS idx_learning_progress_user
    ON learning_progress(user_id, completed_at DESC);
