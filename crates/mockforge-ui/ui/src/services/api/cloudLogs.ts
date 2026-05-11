/**
 * Cloud logs API client (#462).
 *
 * Reads workspace-scoped request logs from the registry's `runtime_captures`
 * mirror. Returns the same `RequestLog` shape the local `/__mockforge/logs`
 * endpoint produces so the `LogsPage` UI renders either source.
 *
 * Captures land in `runtime_captures` from two writers today:
 *   - `--cloud-ship` (local mockforge sending to cloud) — populates
 *     `workspace_id` and is visible here immediately.
 *   - Hosted-mock in-container shipper — does NOT yet populate
 *     `workspace_id`. Those rows are invisible to this endpoint until the
 *     shipper backfill lands. Tracked separately.
 */
import { fetchJsonWithErrorBody } from './client';
import { isCloudMode } from '../../utils/cloudMode';
import type { RequestLog } from '../../types';

export interface CloudLogsQuery {
  method?: string;
  path?: string;
  status?: string;
  limit?: number;
}

class CloudLogsApi {
  private guard(method: string): void {
    if (!isCloudMode()) {
      throw new Error(`Cloud logs ${method} only works in cloud mode.`);
    }
  }

  async getLogs(workspaceId: string, params?: CloudLogsQuery): Promise<RequestLog[]> {
    this.guard('getLogs');
    const qp = new URLSearchParams();
    if (params?.method) qp.set('method', params.method);
    if (params?.path) qp.set('path', params.path);
    if (params?.status) qp.set('status', params.status);
    if (params?.limit != null) qp.set('limit', String(params.limit));
    const qs = qp.toString();
    const url = `/api/v1/workspaces/${workspaceId}/request-logs${qs ? `?${qs}` : ''}`;
    return fetchJsonWithErrorBody(url) as Promise<RequestLog[]>;
  }
}

export const cloudLogsApi = new CloudLogsApi();
