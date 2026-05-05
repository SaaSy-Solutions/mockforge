/**
 * Adapter tests for cloudCommunity — proves the cloud schema maps
 * correctly into the legacy ShowcaseProject / LearningResource shapes
 * the existing pages consume.
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

import {
  showcaseEntryToProject,
  trackToLearningResource,
  recipeToLearningResource,
  cloudCommunityApi,
  type ShowcaseEntry,
  type LearningTrack,
  type LearningRecipe,
} from '../cloudCommunity';

const baseEntry: ShowcaseEntry = {
  id: 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa',
  slug: 'awesome-mock',
  org_id: null,
  submitted_by: 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb',
  title: 'Awesome Mock',
  description: 'demo',
  body: null,
  screenshots: ['https://example.com/a.png', 'https://example.com/b.png'],
  demo_url: 'https://demo',
  source_url: null,
  tags: ['payments', 'graphql'],
  is_featured: true,
  is_published: true,
  likes_count: 7,
  created_at: '2026-05-01T00:00:00Z',
  updated_at: '2026-05-02T00:00:00Z',
};

describe('showcaseEntryToProject', () => {
  it('uses slug as id and maps screenshots/featured/likes', () => {
    const p = showcaseEntryToProject(baseEntry);
    expect(p.id).toBe('awesome-mock');
    expect(p.featured).toBe(true);
    expect(p.screenshot).toBe('https://example.com/a.png');
    expect(p.stats.stars).toBe(7);
    expect(p.stats.downloads).toBe(0);
    expect(p.stats.rating).toBe(0);
    expect(p.testimonials).toEqual([]);
    expect(p.tags).toEqual(['payments', 'graphql']);
  });

  it('uses tags[0] as category, falling back to "general" for empty tags', () => {
    expect(showcaseEntryToProject(baseEntry).category).toBe('payments');
    const noTags = showcaseEntryToProject({ ...baseEntry, tags: [] });
    expect(noTags.category).toBe('general');
  });

  it('drops null demo/source urls instead of leaking null into the UI shape', () => {
    const p = showcaseEntryToProject({ ...baseEntry, demo_url: null, source_url: null });
    expect(p.demo_url).toBeUndefined();
    expect(p.source_url).toBeUndefined();
  });

  it('handles missing screenshots gracefully', () => {
    const p = showcaseEntryToProject({ ...baseEntry, screenshots: [] });
    expect(p.screenshot).toBeUndefined();
  });
});

describe('trackToLearningResource', () => {
  const track: LearningTrack = {
    id: 'cccccccc-cccc-cccc-cccc-cccccccccccc',
    slug: 'getting-started',
    title: 'Getting Started',
    description: 'first steps',
    body: null,
    is_published: true,
    sort_order: 0,
    created_at: '2026-05-01T00:00:00Z',
    updated_at: '2026-05-01T00:00:00Z',
  };

  it('maps a track to a guide-typed resource keyed by slug', () => {
    const r = trackToLearningResource(track);
    expect(r.id).toBe('getting-started');
    expect(r.resource_type).toBe('guide');
    expect(r.category).toBe('tracks');
    expect(r.description).toBe('first steps');
    expect(r.code_examples).toEqual([]);
  });

  it('substitutes empty description for null', () => {
    const r = trackToLearningResource({ ...track, description: null });
    expect(r.description).toBe('');
  });
});

describe('recipeToLearningResource', () => {
  const recipe: LearningRecipe = {
    id: 'dddddddd-dddd-dddd-dddd-dddddddddddd',
    slug: 'rest-pagination',
    title: 'REST Pagination',
    description: 'cursor + offset',
    body: '## Heading\n\nbody',
    tags: ['rest', 'pagination'],
    is_published: true,
    created_at: '2026-05-01T00:00:00Z',
    updated_at: '2026-05-01T00:00:00Z',
  };

  it('maps a recipe to an example-typed resource and surfaces body as a code example', () => {
    const r = recipeToLearningResource(recipe);
    expect(r.id).toBe('rest-pagination');
    expect(r.resource_type).toBe('example');
    expect(r.category).toBe('recipes');
    expect(r.tags).toEqual(['rest', 'pagination']);
    expect(r.code_examples).toHaveLength(1);
    expect(r.code_examples[0]).toMatchObject({
      title: 'REST Pagination',
      language: 'markdown',
      code: '## Heading\n\nbody',
    });
  });

  it('produces an empty code_examples list for a recipe with no body', () => {
    const r = recipeToLearningResource({ ...recipe, body: '' });
    expect(r.code_examples).toEqual([]);
  });
});

// --- isCloudMode guard -----------------------------------------------------

describe('CloudCommunityApi guard', () => {
  beforeEach(() => {
    vi.stubEnv('VITE_MOCKFORGE_MODE', 'local');
    vi.stubEnv('VITE_API_BASE_URL', '');
  });

  afterEach(() => {
    vi.unstubAllEnvs();
  });

  it('rejects calls when not running in cloud mode', async () => {
    await expect(cloudCommunityApi.getShowcaseProjects()).rejects.toThrow(/cloud mode/i);
    await expect(cloudCommunityApi.getLearningResources()).rejects.toThrow(/cloud mode/i);
    await expect(cloudCommunityApi.getShowcaseProject('x')).rejects.toThrow(/cloud mode/i);
  });

  it('refuses public submissions even in cloud mode (admin-only)', async () => {
    vi.stubEnv('VITE_MOCKFORGE_MODE', 'cloud');
    const r = await cloudCommunityApi.submitShowcaseProject({ title: 'x' });
    expect(r.success).toBe(false);
    expect(r.error).toMatch(/admin-curated/i);
  });
});
