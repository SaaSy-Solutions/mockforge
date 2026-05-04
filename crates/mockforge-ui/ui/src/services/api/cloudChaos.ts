/**
 * Cloud chaos API client (#7).
 *
 * Wraps chaos campaign CRUD + run trigger + report read paths against the
 * registry server. Run trigger goes through the test_runs lifecycle so
 * the live event stream is cloudTestRunsApi.streamRunEvents.
 */
import { fetchJsonWithErrorBody } from './client';
import { isCloudMode } from '../../utils/cloudMode';

export type ChaosTargetKind = 'hosted_mock' | 'external';

export interface ChaosCampaign {
  id: string;
  workspace_id: string;
  name: string;
  description: string | null;
  target_kind: ChaosTargetKind;
  target_ref: string;
  config: Record<string, unknown>;
  safety_config: Record<string, unknown>;
  created_by: string | null;
  created_at: string;
  updated_at: string;
}

export interface CreateCampaignRequest {
  name: string;
  description?: string;
  target_kind: ChaosTargetKind;
  target_ref: string;
  config: Record<string, unknown>;
  safety_config: Record<string, unknown>;
}

export interface ChaosCampaignReport {
  id: string;
  campaign_id: string;
  run_id: string;
  fault_count: number;
  aborted: boolean;
  abort_reason: string | null;
  summary: Record<string, unknown> | null;
  recommendations: Record<string, unknown> | null;
  created_at: string;
}

export interface ResiliencePattern {
  id: string;
  workspace_id: string | null;
  kind: string;
  name: string;
  config: Record<string, unknown>;
  created_at: string;
}

class CloudChaosApi {
  private guard(method: string): void {
    if (!isCloudMode()) {
      throw new Error(`Cloud chaos ${method} only works in cloud mode.`);
    }
  }

  async listCampaigns(workspaceId: string): Promise<ChaosCampaign[]> {
    this.guard('listCampaigns');
    return fetchJsonWithErrorBody(
      `/api/v1/workspaces/${workspaceId}/chaos-campaigns`,
    ) as Promise<ChaosCampaign[]>;
  }

  async createCampaign(
    workspaceId: string,
    body: CreateCampaignRequest,
  ): Promise<ChaosCampaign> {
    this.guard('createCampaign');
    return fetchJsonWithErrorBody(
      `/api/v1/workspaces/${workspaceId}/chaos-campaigns`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      },
    ) as Promise<ChaosCampaign>;
  }

  async getCampaign(id: string): Promise<ChaosCampaign> {
    this.guard('getCampaign');
    return fetchJsonWithErrorBody(
      `/api/v1/chaos-campaigns/${id}`,
    ) as Promise<ChaosCampaign>;
  }

  async deleteCampaign(id: string): Promise<{ deleted: boolean }> {
    this.guard('deleteCampaign');
    return fetchJsonWithErrorBody(`/api/v1/chaos-campaigns/${id}`, {
      method: 'DELETE',
    }) as Promise<{ deleted: boolean }>;
  }

  async listReports(id: string): Promise<ChaosCampaignReport[]> {
    this.guard('listReports');
    return fetchJsonWithErrorBody(
      `/api/v1/chaos-campaigns/${id}/reports`,
    ) as Promise<ChaosCampaignReport[]>;
  }

  /** Returns the test_runs row created for this campaign run. */
  async triggerRun(
    id: string,
  ): Promise<{ id: string; status: string; kind: string }> {
    this.guard('triggerRun');
    return fetchJsonWithErrorBody(`/api/v1/chaos-campaigns/${id}/runs`, {
      method: 'POST',
    }) as Promise<{ id: string; status: string; kind: string }>;
  }

  async listResiliencePatterns(workspaceId: string): Promise<ResiliencePattern[]> {
    this.guard('listResiliencePatterns');
    return fetchJsonWithErrorBody(
      `/api/v1/workspaces/${workspaceId}/resilience-patterns`,
    ) as Promise<ResiliencePattern[]>;
  }
}

export const cloudChaosApi = new CloudChaosApi();
