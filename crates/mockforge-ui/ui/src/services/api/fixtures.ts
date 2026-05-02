/**
 * Fixtures API service — cloud-aware wrapper over /api/v1/fixtures (hosted)
 * and /__mockforge/fixtures (local dev). The two backends expose slightly
 * different shapes, so this module normalises the calls.
 */
import type { FixtureInfo } from '../../types';
import { FixturesResponseSchema } from '../../schemas/api';
import { fetchJson, fetchJsonWithValidation, authenticatedFetch } from './client';
import { isCloudMode } from '../../utils/cloudMode';

const isCloud = isCloudMode();
const CLOUD_BASE = '/api/v1/fixtures';
const LOCAL_BASE = '/__mockforge/fixtures';
const FIXTURE_API_BASE = isCloud ? CLOUD_BASE : LOCAL_BASE;

export interface FixtureCreatePayload {
  name: string;
  path?: string;
  method?: string;
  description?: string;
  protocol?: string;
  tags?: string[];
  content?: unknown;
}

export interface FixtureUpdatePayload {
  name?: string;
  path?: string;
  method?: string;
  description?: string;
  protocol?: string;
  tags?: string[];
  content?: unknown;
}

class FixturesApiService {
  constructor() {
    // Preserve `this` when consumers destructure methods.
    this.getFixtures = this.getFixtures.bind(this);
    this.createFixture = this.createFixture.bind(this);
    this.updateFixture = this.updateFixture.bind(this);
    this.deleteFixture = this.deleteFixture.bind(this);
    this.deleteFixturesBulk = this.deleteFixturesBulk.bind(this);
    this.downloadFixture = this.downloadFixture.bind(this);
    this.renameFixture = this.renameFixture.bind(this);
    this.moveFixture = this.moveFixture.bind(this);
  }

  async getFixtures(): Promise<FixtureInfo[]> {
    if (isCloud) {
      return fetchJson(FIXTURE_API_BASE) as Promise<FixtureInfo[]>;
    }
    return fetchJsonWithValidation<FixtureInfo[]>(
      FIXTURE_API_BASE,
      FixturesResponseSchema
    );
  }

  async createFixture(payload: FixtureCreatePayload): Promise<FixtureInfo> {
    return fetchJson(FIXTURE_API_BASE, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload),
    }) as Promise<FixtureInfo>;
  }

  async updateFixture(
    fixtureId: string,
    payload: FixtureUpdatePayload
  ): Promise<FixtureInfo> {
    if (!isCloud) {
      throw new Error(
        'Editing fixture content is only supported on the hosted backend; use rename/move locally.'
      );
    }
    return fetchJson(`${FIXTURE_API_BASE}/${fixtureId}`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload),
    }) as Promise<FixtureInfo>;
  }

  async deleteFixture(fixtureId: string): Promise<void> {
    return fetchJson(`${FIXTURE_API_BASE}/${fixtureId}`, {
      method: 'DELETE',
    }) as Promise<void>;
  }

  async deleteFixturesBulk(fixtureIds: string[]): Promise<void> {
    if (isCloud) {
      // Cloud has no bulk endpoint; fan out individual DELETE calls instead.
      await Promise.all(fixtureIds.map((id) => this.deleteFixture(id)));
      return;
    }
    return fetchJson(`${LOCAL_BASE}/bulk`, {
      method: 'DELETE',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ fixture_ids: fixtureIds }),
    }) as Promise<void>;
  }

  async downloadFixture(fixture: FixtureInfo): Promise<{ blob: Blob; filename: string }> {
    if (isCloud) {
      // No dedicated download endpoint on cloud — synthesize a JSON blob
      // from the stored `content` (falling back to the full fixture record
      // so the user always gets something useful).
      const payload =
        fixture.content ?? {
          id: fixture.id,
          name: fixture.name,
          path: fixture.path,
          method: fixture.method,
          description: fixture.description,
          tags: fixture.tags,
          protocol: fixture.protocol,
        };
      const blob = new Blob([JSON.stringify(payload, null, 2)], {
        type: 'application/json',
      });
      const baseName = fixture.name || fixture.id;
      return { blob, filename: `${baseName}.json` };
    }

    const response = await authenticatedFetch(`${LOCAL_BASE}/${fixture.id}/download`);
    if (!response.ok) {
      if (response.status === 401) throw new Error('Authentication required');
      if (response.status === 403) throw new Error('Access denied');
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    const blob = await response.blob();
    const contentDisposition = response.headers.get('Content-Disposition');
    const filenameMatch = contentDisposition?.match(/filename="?([^"]+)"?/);
    const filename = filenameMatch?.[1] || `${fixture.id}.json`;
    return { blob, filename };
  }

  async renameFixture(fixtureId: string, newName: string): Promise<void> {
    if (isCloud) {
      await this.updateFixture(fixtureId, { name: newName });
      return;
    }
    return fetchJson(`${LOCAL_BASE}/${fixtureId}/rename`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ new_name: newName }),
    }) as Promise<void>;
  }

  async moveFixture(fixtureId: string, newPath: string): Promise<void> {
    if (isCloud) {
      await this.updateFixture(fixtureId, { path: newPath });
      return;
    }
    return fetchJson(`${LOCAL_BASE}/${fixtureId}/move`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ new_path: newPath }),
    }) as Promise<void>;
  }
}

export { FixturesApiService };
