/**
 * Cloud consistency / Virtual Backends API client (#461).
 *
 * Backs the registry-side `/api/v1/consistency/*` and
 * `/api/v1/workspaces/{workspace_id}/consistency/*` routes added in
 * `mockforge-registry-server::handlers::consistency`.
 *
 * The response shapes deliberately mirror what the local
 * `consistencyApi` (services/api/consistency.ts) returns so the
 * `VirtualBackendsPage` only needs to swap which service it calls
 * — not refactor its data plumbing. Time-driven state transitions
 * the local engine runs aren't part of cloud yet; entities show their
 * applied initial state until manually re-applied.
 */
import { fetchJsonWithErrorBody } from './client';
import { isCloudMode } from '../../utils/cloudMode';

/** Server-side `LifecyclePreset` shape (consistency.rs). */
export interface CloudLifecyclePreset {
  id: string;
  name: string;
  description: string;
  initial_state: string;
  states: string[];
  affected_endpoints: string[];
}

/** Server-side `VirtualEntity` shape (consistency.rs). */
export interface CloudVirtualEntity {
  id: string;
  workspace_id: string;
  entity_type: string;
  entity_id: string;
  persona_id: string | null;
  current_state: string | null;
  data: Record<string, unknown>;
  seen_in_protocols: string[];
  created_at: string;
  updated_at: string;
}

export interface ApplyPresetRequest {
  preset: string;
  persona_id: string;
  /** Defaults to the preset id when omitted. */
  entity_type?: string;
  /** Defaults to `{persona_id}:{entity_type}` for idempotency. */
  entity_id?: string;
}

class CloudConsistencyApi {
  private guard(method: string): void {
    if (!isCloudMode()) {
      throw new Error(`Cloud consistency ${method} only works in cloud mode.`);
    }
  }

  async listLifecyclePresets(): Promise<CloudLifecyclePreset[]> {
    this.guard('listLifecyclePresets');
    return fetchJsonWithErrorBody(
      '/api/v1/consistency/lifecycle-presets',
    ) as Promise<CloudLifecyclePreset[]>;
  }

  async getLifecyclePreset(presetId: string): Promise<CloudLifecyclePreset> {
    this.guard('getLifecyclePreset');
    return fetchJsonWithErrorBody(
      `/api/v1/consistency/lifecycle-presets/${encodeURIComponent(presetId)}`,
    ) as Promise<CloudLifecyclePreset>;
  }

  async applyLifecyclePreset(
    workspaceId: string,
    body: ApplyPresetRequest,
  ): Promise<CloudVirtualEntity> {
    this.guard('applyLifecyclePreset');
    return fetchJsonWithErrorBody(
      `/api/v1/workspaces/${workspaceId}/consistency/lifecycle-presets/apply`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      },
    ) as Promise<CloudVirtualEntity>;
  }

  /**
   * List entities for a workspace. Returns the data wrapped in the same
   * `{workspace, entities, count}` envelope the local `consistencyApi`
   * uses — keeps `VirtualBackendsPage` from needing two code paths just
   * to unpack the response.
   */
  async listEntities(
    workspaceId: string,
    opts?: { entityType?: string; personaId?: string },
  ): Promise<{
    workspace: string;
    entities: Array<{
      entity_type: string;
      entity_id: string;
      data: Record<string, unknown>;
      seen_in_protocols: string[];
      created_at: string;
      updated_at: string;
      persona_id: string | null;
    }>;
    count: number;
  }> {
    this.guard('listEntities');
    const qs = new URLSearchParams();
    if (opts?.entityType) qs.set('entity_type', opts.entityType);
    if (opts?.personaId) qs.set('persona_id', opts.personaId);
    const suffix = qs.toString() ? `?${qs.toString()}` : '';
    const rows = (await fetchJsonWithErrorBody(
      `/api/v1/workspaces/${workspaceId}/consistency/entities${suffix}`,
    )) as CloudVirtualEntity[];

    // Adapt to the local-shape envelope. We drop the synthetic UUID/workspace_id
    // fields because the page never reads them; persona_id/current_state stay
    // on the row for any future use.
    return {
      workspace: workspaceId,
      count: rows.length,
      entities: rows.map((r) => ({
        entity_type: r.entity_type,
        entity_id: r.entity_id,
        data: r.data,
        seen_in_protocols: r.seen_in_protocols,
        created_at: r.created_at,
        updated_at: r.updated_at,
        persona_id: r.persona_id,
      })),
    };
  }

  async getEntity(id: string): Promise<CloudVirtualEntity> {
    this.guard('getEntity');
    return fetchJsonWithErrorBody(
      `/api/v1/consistency/entities/${encodeURIComponent(id)}`,
    ) as Promise<CloudVirtualEntity>;
  }
}

export const cloudConsistencyApi = new CloudConsistencyApi();
