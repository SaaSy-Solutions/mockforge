/**
 * Cloud graph API client (#460).
 *
 * Read-only workspace dependency graph view. Returns nodes for every
 * `service` and `flow` in the workspace, clustered as a single workspace
 * cluster. Phase 1 returns no edges; cross-flow / service-call edge
 * derivation is a follow-up.
 *
 * The response shape mirrors the local `/__mockforge/graph` payload so the
 * existing `GraphPage` UI renders either mode unchanged.
 */
import { fetchJsonWithErrorBody } from './client';
import { isCloudMode } from '../../utils/cloudMode';
import type { GraphData } from '../../types/graph';

class CloudGraphApi {
  private guard(method: string): void {
    if (!isCloudMode()) {
      throw new Error(`Cloud graph ${method} only works in cloud mode.`);
    }
  }

  async getWorkspaceGraph(workspaceId: string): Promise<GraphData> {
    this.guard('getWorkspaceGraph');
    return fetchJsonWithErrorBody(
      `/api/v1/workspaces/${workspaceId}/graph`,
    ) as Promise<GraphData>;
  }
}

export const cloudGraphApi = new CloudGraphApi();
