/**
 * Cloud fitness functions API client (#355).
 *
 * Wraps the registry's workspace-scoped fitness function CRUD plus the
 * trigger-run endpoint that enqueues a `kind=fitness_evaluation`
 * test_run. The runner-side FitnessExecutor evaluates against
 * `runtime_captures` and raises a fitness-source incident on failure.
 */
import { fetchJsonWithErrorBody } from './client';
import { isCloudMode } from '../../utils/cloudMode';
import type { TestRun } from './cloudTestRuns';

/** One of the four kinds the registry recognizes. */
export type CloudFitnessKind =
  | 'latency_threshold'
  | 'error_rate'
  | 'contract_stability'
  | 'custom_query';

export interface CloudFitnessFunction {
  id: string;
  workspace_id: string;
  name: string;
  kind: CloudFitnessKind | string;
  config: Record<string, unknown>;
  last_evaluated_at: string | null;
  last_status: string | null;
  created_at: string;
  updated_at: string;
}

export interface CreateCloudFitnessRequest {
  name: string;
  kind: CloudFitnessKind | string;
  config: Record<string, unknown>;
}

export interface UpdateCloudFitnessRequest {
  name?: string;
  config?: Record<string, unknown>;
}

class CloudFitnessApi {
  private guard(method: string): void {
    if (!isCloudMode()) {
      throw new Error(`Cloud fitness ${method} only works in cloud mode.`);
    }
  }

  async listForWorkspace(workspaceId: string): Promise<CloudFitnessFunction[]> {
    this.guard('listForWorkspace');
    return fetchJsonWithErrorBody(
      `/api/v1/workspaces/${encodeURIComponent(workspaceId)}/fitness-functions`,
    ) as Promise<CloudFitnessFunction[]>;
  }

  async create(
    workspaceId: string,
    body: CreateCloudFitnessRequest,
  ): Promise<CloudFitnessFunction> {
    this.guard('create');
    return fetchJsonWithErrorBody(
      `/api/v1/workspaces/${encodeURIComponent(workspaceId)}/fitness-functions`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      },
    ) as Promise<CloudFitnessFunction>;
  }

  async update(
    id: string,
    body: UpdateCloudFitnessRequest,
  ): Promise<CloudFitnessFunction> {
    this.guard('update');
    return fetchJsonWithErrorBody(`/api/v1/fitness-functions/${encodeURIComponent(id)}`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    }) as Promise<CloudFitnessFunction>;
  }

  async delete(id: string): Promise<{ deleted: boolean }> {
    this.guard('delete');
    return fetchJsonWithErrorBody(`/api/v1/fitness-functions/${encodeURIComponent(id)}`, {
      method: 'DELETE',
    }) as Promise<{ deleted: boolean }>;
  }

  /**
   * Enqueue a fitness_evaluation run. Live progress events flow over
   * cloudTestRunsApi.streamRunEvents(run.id).
   */
  async triggerRun(id: string): Promise<TestRun> {
    this.guard('triggerRun');
    return fetchJsonWithErrorBody(
      `/api/v1/fitness-functions/${encodeURIComponent(id)}/runs`,
      { method: 'POST' },
    ) as Promise<TestRun>;
  }
}

export const cloudFitnessApi = new CloudFitnessApi();
