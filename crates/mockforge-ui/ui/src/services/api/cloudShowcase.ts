/**
 * Showcase + Learning Hub API client (#12).
 *
 * Public read paths are open; admin authoring routes require an
 * authenticated user.
 */
import { fetchJsonWithErrorBody } from './client';
import { isCloudMode } from '../../utils/cloudMode';

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

export interface CreateShowcaseEntryRequest {
  slug: string;
  title: string;
  description: string;
  body?: string;
  screenshots?: string[];
  demo_url?: string;
  source_url?: string;
  tags?: string[];
}

export interface UpdateShowcaseEntryRequest {
  is_published?: boolean;
  is_featured?: boolean;
}

class CloudShowcaseApi {
  private guard(method: string): void {
    if (!isCloudMode()) {
      throw new Error(`Cloud showcase ${method} only works in cloud mode.`);
    }
  }

  /** Public — no auth needed. */
  async listEntries(opts?: { tag?: string; limit?: number }): Promise<ShowcaseEntry[]> {
    this.guard('listEntries');
    const params = new URLSearchParams();
    if (opts?.tag) params.set('tag', opts.tag);
    if (opts?.limit) params.set('limit', String(opts.limit));
    const qs = params.toString() ? `?${params}` : '';
    return fetchJsonWithErrorBody(
      `/api/v1/showcase/entries${qs}`,
    ) as Promise<ShowcaseEntry[]>;
  }

  async getEntry(slug: string): Promise<ShowcaseEntry> {
    this.guard('getEntry');
    return fetchJsonWithErrorBody(
      `/api/v1/showcase/entries/${encodeURIComponent(slug)}`,
    ) as Promise<ShowcaseEntry>;
  }

  // --- admin authoring -----------------------------------------------------

  /** Admin — returns every entry regardless of published status. */
  async adminList(): Promise<ShowcaseEntry[]> {
    this.guard('adminList');
    return fetchJsonWithErrorBody(
      '/api/v1/admin/showcase/entries',
    ) as Promise<ShowcaseEntry[]>;
  }

  async adminCreate(body: CreateShowcaseEntryRequest): Promise<ShowcaseEntry> {
    this.guard('adminCreate');
    return fetchJsonWithErrorBody('/api/v1/admin/showcase/entries', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    }) as Promise<ShowcaseEntry>;
  }

  async adminUpdate(
    id: string,
    body: UpdateShowcaseEntryRequest,
  ): Promise<ShowcaseEntry> {
    this.guard('adminUpdate');
    return fetchJsonWithErrorBody(`/api/v1/admin/showcase/entries/${id}`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    }) as Promise<ShowcaseEntry>;
  }

  async adminDelete(id: string): Promise<{ deleted: boolean }> {
    this.guard('adminDelete');
    return fetchJsonWithErrorBody(`/api/v1/admin/showcase/entries/${id}`, {
      method: 'DELETE',
    }) as Promise<{ deleted: boolean }>;
  }
}

export const cloudShowcaseApi = new CloudShowcaseApi();
