/**
 * Cloud flows API client (#9 + #14 collab).
 *
 * `flows` is the unified resource backing the Scenario / Orchestration /
 * State Machine / Chain editor pages. Each row has a kind discriminator
 * (`scenario | orchestration | state_machine | chain`) and a versioned
 * config (FlowVersion). Run trigger goes through the standard
 * `test_runs` lifecycle so cloud-mode pages reuse the cloudTestRuns
 * SSE stream for live progress.
 */
import { fetchJsonWithErrorBody } from './client';
import { isCloudMode } from '../../utils/cloudMode';

export type FlowKind =
  | 'scenario'
  | 'orchestration'
  | 'state_machine'
  | 'chain';

export interface Flow {
  id: string;
  workspace_id: string;
  kind: FlowKind;
  name: string;
  description: string | null;
  current_version_id: string | null;
  metadata: Record<string, unknown>;
  created_by: string | null;
  created_at: string;
  updated_at: string;
}

export interface FlowVersion {
  id: string;
  flow_id: string;
  version_number: number;
  config: Record<string, unknown>;
  changelog: string | null;
  created_by: string | null;
  created_at: string;
}

export interface CreateFlowRequest {
  kind: FlowKind;
  name: string;
  description?: string;
  initial_config: Record<string, unknown>;
}

export interface SaveFlowVersionRequest {
  config: Record<string, unknown>;
  changelog?: string;
  /** Default true — sets the new version as flow.current_version_id. */
  set_current?: boolean;
}

class CloudFlowsApi {
  private guard(method: string): void {
    if (!isCloudMode()) {
      throw new Error(`Cloud flows ${method} only works in cloud mode.`);
    }
  }

  async listForWorkspace(workspaceId: string, kind?: FlowKind): Promise<Flow[]> {
    this.guard('listForWorkspace');
    const qs = kind ? `?kind=${encodeURIComponent(kind)}` : '';
    return fetchJsonWithErrorBody(
      `/api/v1/workspaces/${workspaceId}/flows${qs}`,
    ) as Promise<Flow[]>;
  }

  async create(workspaceId: string, body: CreateFlowRequest): Promise<Flow> {
    this.guard('create');
    return fetchJsonWithErrorBody(`/api/v1/workspaces/${workspaceId}/flows`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    }) as Promise<Flow>;
  }

  async get(id: string): Promise<Flow> {
    this.guard('get');
    return fetchJsonWithErrorBody(`/api/v1/flows/${id}`) as Promise<Flow>;
  }

  async update(
    id: string,
    body: { name?: string; description?: string | null },
  ): Promise<Flow> {
    this.guard('update');
    return fetchJsonWithErrorBody(`/api/v1/flows/${id}`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    }) as Promise<Flow>;
  }

  async delete(id: string): Promise<{ deleted: boolean }> {
    this.guard('delete');
    return fetchJsonWithErrorBody(`/api/v1/flows/${id}`, {
      method: 'DELETE',
    }) as Promise<{ deleted: boolean }>;
  }

  async listVersions(id: string): Promise<FlowVersion[]> {
    this.guard('listVersions');
    return fetchJsonWithErrorBody(
      `/api/v1/flows/${id}/versions`,
    ) as Promise<FlowVersion[]>;
  }

  async getVersion(id: string, versionId: string): Promise<FlowVersion> {
    this.guard('getVersion');
    return fetchJsonWithErrorBody(
      `/api/v1/flows/${id}/versions/${versionId}`,
    ) as Promise<FlowVersion>;
  }

  async saveVersion(
    id: string,
    body: SaveFlowVersionRequest,
  ): Promise<FlowVersion> {
    this.guard('saveVersion');
    return fetchJsonWithErrorBody(`/api/v1/flows/${id}/versions`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    }) as Promise<FlowVersion>;
  }

  /**
   * Trigger a run. Returns the test_runs row; live events flow over
   * the cloudTestRunsApi.streamRunEvents(run.id) SSE stream.
   */
  async triggerRun(id: string): Promise<{ id: string; status: string; kind: string }> {
    this.guard('triggerRun');
    return fetchJsonWithErrorBody(`/api/v1/flows/${id}/runs`, {
      method: 'POST',
    }) as Promise<{ id: string; status: string; kind: string }>;
  }
}

export const cloudFlowsApi = new CloudFlowsApi();
