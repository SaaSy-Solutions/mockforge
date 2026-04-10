/**
 * Fixtures API service — fixture listing, deletion, download, rename, and move.
 */
import type { FixtureInfo } from '../../types';
import { FixturesResponseSchema } from '../../schemas/api';
import { fetchJson, fetchJsonWithValidation, authenticatedFetch } from './client';

const isCloud = !!import.meta.env.VITE_API_BASE_URL;
const FIXTURE_API_BASE = isCloud ? '/api/v1/fixtures' : '/__mockforge/fixtures';

class FixturesApiService {
  constructor() {
    // Bind all methods to ensure 'this' context is preserved
    this.getFixtures = this.getFixtures.bind(this);
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

  async deleteFixture(fixtureId: string): Promise<void> {
    return fetchJson(`${FIXTURE_API_BASE}/${fixtureId}`, {
      method: 'DELETE',
    }) as Promise<void>;
  }

  async deleteFixturesBulk(fixtureIds: string[]): Promise<void> {
    return fetchJson('/__mockforge/fixtures/bulk', {
      method: 'DELETE',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ fixture_ids: fixtureIds }),
    }) as Promise<void>;
  }

  async downloadFixture(fixtureId: string): Promise<Blob> {
    const response = await authenticatedFetch(`${FIXTURE_API_BASE}/${fixtureId}/download`);
    if (!response.ok) {
      if (response.status === 401) {
        throw new Error('Authentication required');
      }
      if (response.status === 403) {
        throw new Error('Access denied');
      }
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    return response.blob();
  }

  async renameFixture(fixtureId: string, newName: string): Promise<void> {
    return fetchJson(`${FIXTURE_API_BASE}/${fixtureId}/rename`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ new_name: newName }),
    }) as Promise<void>;
  }

  async moveFixture(fixtureId: string, newPath: string): Promise<void> {
    return fetchJson(`${FIXTURE_API_BASE}/${fixtureId}/move`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ new_path: newPath }),
    }) as Promise<void>;
  }
}

export { FixturesApiService };
