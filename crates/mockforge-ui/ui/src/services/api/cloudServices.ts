/**
 * Cloud Services API — CRUD wrapper over `/api/v1/services`.
 *
 * Only active in cloud mode (when `VITE_API_BASE_URL` is set). In self-hosted
 * mode the services page derives its data from `/__mockforge/routes` instead
 * and there is no persistence layer to talk to.
 */
import { fetchJson } from './client';

export const CLOUD_SERVICES_BASE = '/api/v1/services';

export interface CloudService {
  id: string;
  org_id: string;
  workspace_id?: string | null;
  name: string;
  description: string;
  base_url: string;
  enabled: boolean;
  tags: unknown;
  routes: unknown;
  created_by: string;
  created_at: string;
  updated_at: string;
}

export interface CloudServiceCreatePayload {
  name: string;
  description?: string;
  base_url?: string;
  workspace_id?: string | null;
}

export interface CloudServiceUpdatePayload {
  name?: string;
  description?: string;
  base_url?: string;
  enabled?: boolean;
  tags?: unknown;
  routes?: unknown;
  workspace_id?: string | null;
}

export interface CloudServiceListOptions {
  workspaceId?: string;
}

class CloudServicesApiService {
  constructor() {
    this.list = this.list.bind(this);
    this.get = this.get.bind(this);
    this.create = this.create.bind(this);
    this.update = this.update.bind(this);
    this.remove = this.remove.bind(this);
  }

  async list(options?: CloudServiceListOptions): Promise<CloudService[]> {
    const url = options?.workspaceId
      ? `${CLOUD_SERVICES_BASE}?workspace_id=${encodeURIComponent(options.workspaceId)}`
      : CLOUD_SERVICES_BASE;
    return fetchJson(url) as Promise<CloudService[]>;
  }

  async get(id: string): Promise<CloudService> {
    return fetchJson(`${CLOUD_SERVICES_BASE}/${id}`) as Promise<CloudService>;
  }

  async create(payload: CloudServiceCreatePayload): Promise<CloudService> {
    return fetchJson(CLOUD_SERVICES_BASE, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload),
    }) as Promise<CloudService>;
  }

  async update(id: string, payload: CloudServiceUpdatePayload): Promise<CloudService> {
    return fetchJson(`${CLOUD_SERVICES_BASE}/${id}`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload),
    }) as Promise<CloudService>;
  }

  async remove(id: string): Promise<void> {
    await fetchJson(`${CLOUD_SERVICES_BASE}/${id}`, {
      method: 'DELETE',
    });
  }
}

export { CloudServicesApiService };

/** Shared singleton. Co-located with the class so stores can import it
 *  directly without risking a barrel-import cycle. The barrel
 *  (`services/api/index.ts`) re-exports this same instance. */
export const cloudServicesApi = new CloudServicesApiService();
