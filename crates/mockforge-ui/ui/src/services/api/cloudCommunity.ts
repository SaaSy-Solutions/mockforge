/**
 * Cloud-mode adapter for the Community pages (Showcase + Learning Hub).
 *
 * The local `communityApi` calls embedded `/__mockforge/community/*`
 * endpoints whose data shape (`ShowcaseProject`, `LearningResource`)
 * predates the cloud schema. The cloud registry exposes richer but
 * differently-shaped resources at `/api/v1/showcase/*` and
 * `/api/v1/learning/*`. This adapter speaks the cloud routes, then maps
 * responses into the legacy shapes so `ShowcasePage` and
 * `LearningHubPage` can render unchanged.
 *
 * Mappings worth flagging:
 *   - `ShowcaseEntry.slug` → `ShowcaseProject.id` (slug is the public lookup key)
 *   - `ShowcaseEntry.likes_count` → `stats.stars` (closest analog the cloud has)
 *   - `tags[0]` → `category` (cloud has no separate category column)
 *   - LearningTrack and LearningRecipe both flatten to `LearningResource[]`,
 *     distinguished by `resource_type`: `guide` for tracks, `example` for recipes.
 */
import { fetchJsonWithErrorBody } from './client';
import { isCloudMode } from '../../utils/cloudMode';
import type {
  ApiResponse,
  LearningResource,
  ShowcaseProject,
  SuccessStory,
} from '../communityApi';

// --- Cloud schema types (mirror registry-server) ---------------------------

export interface ShowcaseEntry {
  id: string;
  slug: string;
  org_id: string | null;
  submitted_by: string | null;
  title: string;
  description: string;
  body: string | null;
  screenshots: string[];
  demo_url: string | null;
  source_url: string | null;
  tags: string[];
  is_featured: boolean;
  is_published: boolean;
  likes_count: number;
  created_at: string;
  updated_at: string;
}

export interface LearningTrack {
  id: string;
  slug: string;
  title: string;
  description: string | null;
  body: string | null;
  is_published: boolean;
  sort_order: number;
  created_at: string;
  updated_at: string;
}

export interface LearningLesson {
  id: string;
  track_id: string;
  slug: string;
  title: string;
  body: string | null;
  sort_order: number;
  created_at: string;
  updated_at: string;
}

export interface LearningRecipe {
  id: string;
  slug: string;
  title: string;
  description: string | null;
  body: string;
  tags: string[];
  is_published: boolean;
  created_at: string;
  updated_at: string;
}

export interface TrackDetail extends LearningTrack {
  lessons: LearningLesson[];
}

// --- Adapters --------------------------------------------------------------

export function showcaseEntryToProject(entry: ShowcaseEntry): ShowcaseProject {
  return {
    id: entry.slug,
    title: entry.title,
    author: entry.submitted_by ? 'Community' : 'MockForge',
    description: entry.description,
    category: entry.tags[0] ?? 'general',
    tags: entry.tags,
    featured: entry.is_featured,
    screenshot: entry.screenshots[0],
    demo_url: entry.demo_url ?? undefined,
    source_url: entry.source_url ?? undefined,
    stats: {
      downloads: 0,
      stars: entry.likes_count,
      forks: 0,
      rating: 0,
    },
    testimonials: [],
    created_at: entry.created_at,
    updated_at: entry.updated_at,
  };
}

export function trackToLearningResource(track: LearningTrack): LearningResource {
  return {
    id: track.slug,
    title: track.title,
    description: track.description ?? '',
    category: 'tracks',
    resource_type: 'guide',
    difficulty: 'beginner',
    tags: [],
    author: 'MockForge',
    views: 0,
    rating: 0,
    code_examples: [],
    created_at: track.created_at,
    updated_at: track.updated_at,
  };
}

export function recipeToLearningResource(recipe: LearningRecipe): LearningResource {
  return {
    id: recipe.slug,
    title: recipe.title,
    description: recipe.description ?? '',
    category: 'recipes',
    resource_type: 'example',
    difficulty: 'intermediate',
    tags: recipe.tags,
    author: 'MockForge',
    views: 0,
    rating: 0,
    code_examples: recipe.body
      ? [{ title: recipe.title, language: 'markdown', code: recipe.body }]
      : [],
    created_at: recipe.created_at,
    updated_at: recipe.updated_at,
  };
}

function ok<T>(data: T): ApiResponse<T> {
  return { success: true, data };
}

function fail<T>(error: string): ApiResponse<T> {
  return { success: false, error };
}

// --- Client ----------------------------------------------------------------

class CloudCommunityApi {
  private guard(method: string): void {
    if (!isCloudMode()) {
      throw new Error(`Cloud community ${method} only works in cloud mode.`);
    }
  }

  async getShowcaseProjects(params?: {
    category?: string;
    featured?: boolean;
    limit?: number;
  }): Promise<ApiResponse<ShowcaseProject[]>> {
    this.guard('getShowcaseProjects');
    try {
      const qs = new URLSearchParams();
      // Cloud routes filter by tag, not category. ShowcasePage uses category as
      // a tag-like dimension (we map tags[0] → category), so it's a fair pass-through.
      if (params?.category && params.category !== 'all') qs.set('tag', params.category);
      if (params?.limit) qs.set('limit', String(params.limit));
      const suffix = qs.toString() ? `?${qs}` : '';
      const entries = (await fetchJsonWithErrorBody(
        `/api/v1/showcase/entries${suffix}`,
      )) as ShowcaseEntry[];
      let projects = entries.map(showcaseEntryToProject);
      if (params?.featured) projects = projects.filter((p) => p.featured);
      return ok(projects);
    } catch (e) {
      return fail((e as Error).message);
    }
  }

  async getShowcaseProject(slug: string): Promise<ApiResponse<ShowcaseProject>> {
    this.guard('getShowcaseProject');
    try {
      const entry = (await fetchJsonWithErrorBody(
        `/api/v1/showcase/entries/${encodeURIComponent(slug)}`,
      )) as ShowcaseEntry;
      return ok(showcaseEntryToProject(entry));
    } catch (e) {
      return fail((e as Error).message);
    }
  }

  async getShowcaseCategories(): Promise<ApiResponse<string[]>> {
    this.guard('getShowcaseCategories');
    try {
      const entries = (await fetchJsonWithErrorBody(
        '/api/v1/showcase/entries?limit=500',
      )) as ShowcaseEntry[];
      const tags = new Set<string>();
      for (const e of entries) for (const t of e.tags) tags.add(t);
      return ok(Array.from(tags).sort());
    } catch (e) {
      return fail((e as Error).message);
    }
  }

  // Cloud has no success-stories analog; the page hides the tab in cloud mode,
  // but if anything still calls this we return an empty list rather than throw.
  async getSuccessStories(_params?: {
    featured?: boolean;
    limit?: number;
  }): Promise<ApiResponse<SuccessStory[]>> {
    this.guard('getSuccessStories');
    return ok([]);
  }

  async submitShowcaseProject(
    _project: Partial<ShowcaseProject>,
  ): Promise<ApiResponse<string>> {
    // Public submission isn't part of the cloud surface; the admin authoring
    // route at /api/v1/admin/showcase/entries lives behind the curator UI.
    return fail('Showcase submissions are admin-curated in cloud mode.');
  }

  async getLearningResources(params?: {
    category?: string;
    type?: string;
    difficulty?: string;
    limit?: number;
  }): Promise<ApiResponse<LearningResource[]>> {
    this.guard('getLearningResources');
    try {
      const wantsTracks = !params?.type || params.type === 'all' || params.type === 'guide';
      const wantsRecipes = !params?.type || params.type === 'all' || params.type === 'example';
      const wantsByCategory = !params?.category || params.category === 'all';

      const includeTracks =
        wantsTracks && (wantsByCategory || params?.category === 'tracks');
      const includeRecipes =
        wantsRecipes && (wantsByCategory || params?.category === 'recipes');

      const tagQs = new URLSearchParams();
      // Recipes accept ?tag=; tracks don't filter by tag in the cloud API.
      // We treat any non-bucket category as a recipe tag filter.
      if (
        params?.category &&
        params.category !== 'all' &&
        params.category !== 'tracks' &&
        params.category !== 'recipes'
      ) {
        tagQs.set('tag', params.category);
      }
      const recipeSuffix = tagQs.toString() ? `?${tagQs}` : '';

      const [tracks, recipes] = await Promise.all([
        includeTracks
          ? (fetchJsonWithErrorBody('/api/v1/learning/tracks') as Promise<LearningTrack[]>)
          : Promise.resolve([] as LearningTrack[]),
        includeRecipes
          ? (fetchJsonWithErrorBody(
              `/api/v1/learning/recipes${recipeSuffix}`,
            ) as Promise<LearningRecipe[]>)
          : Promise.resolve([] as LearningRecipe[]),
      ]);

      let merged: LearningResource[] = [
        ...tracks.map(trackToLearningResource),
        ...recipes.map(recipeToLearningResource),
      ];
      if (params?.difficulty && params.difficulty !== 'all') {
        merged = merged.filter((r) => r.difficulty === params.difficulty);
      }
      if (params?.limit) merged = merged.slice(0, params.limit);
      return ok(merged);
    } catch (e) {
      return fail((e as Error).message);
    }
  }

  async getLearningResource(slug: string): Promise<ApiResponse<LearningResource>> {
    this.guard('getLearningResource');
    // We don't know up-front whether the slug is a track or recipe; try
    // recipe first (cheaper, no lessons join), then fall back to track.
    try {
      const recipe = (await fetchJsonWithErrorBody(
        `/api/v1/learning/recipes/${encodeURIComponent(slug)}`,
      )) as LearningRecipe;
      return ok(recipeToLearningResource(recipe));
    } catch {
      // fall through
    }
    try {
      const detail = (await fetchJsonWithErrorBody(
        `/api/v1/learning/tracks/${encodeURIComponent(slug)}`,
      )) as TrackDetail;
      const resource = trackToLearningResource(detail);
      // Surface ordered lesson titles + bodies as code examples so the
      // existing accordion view shows track contents.
      resource.code_examples = (detail.lessons ?? []).map((l) => ({
        title: l.title,
        language: 'markdown',
        code: l.body ?? '',
      }));
      return ok(resource);
    } catch (e) {
      return fail((e as Error).message);
    }
  }

  async getLearningCategories(): Promise<ApiResponse<string[]>> {
    this.guard('getLearningCategories');
    try {
      const recipes = (await fetchJsonWithErrorBody(
        '/api/v1/learning/recipes',
      )) as LearningRecipe[];
      const tags = new Set<string>(['tracks', 'recipes']);
      for (const r of recipes) for (const t of r.tags) tags.add(t);
      return ok(Array.from(tags).sort());
    } catch (e) {
      return fail((e as Error).message);
    }
  }
}

export const cloudCommunityApi = new CloudCommunityApi();
