/**
 * Cloud snapshots API client (#10 Time Travel).
 *
 * Backs `/api/v1/workspaces/{workspace_id}/snapshots` + `/api/v1/snapshots/{id}`
 * routes. Capture is synchronous; the response includes the manifest.
 */
import { fetchJsonWithErrorBody } from './client';
import { isCloudMode } from '../../utils/cloudMode';

export type SnapshotStatus = 'capturing' | 'ready' | 'failed' | 'expired';

export interface Snapshot {
  id: string;
  org_id: string;
  workspace_id: string;
  hosted_deployment_id: string | null;
  name: string | null;
  description: string | null;
  status: SnapshotStatus;
  storage_url: string | null;
  size_bytes: number | null;
  manifest: Record<string, unknown> | null;
  triggered_by: string;
  triggered_by_user: string | null;
  captured_at: string | null;
  expires_at: string | null;
  created_at: string;
}

export interface CaptureSnapshotRequest {
  name?: string;
  description?: string;
  hosted_deployment_id?: string;
}

export interface SnapshotResourceDiff {
  added: unknown[];
  removed: unknown[];
  changed: { from: unknown; to: unknown }[];
}

export interface SnapshotDiff {
  snapshot_id: string;
  against_kind: 'current' | 'snapshot';
  against_snapshot_id: string | null;
  services: SnapshotResourceDiff;
  fixtures: SnapshotResourceDiff;
  flows: SnapshotResourceDiff;
  environments: SnapshotResourceDiff;
  chaos_campaigns: SnapshotResourceDiff;
}

export interface SnapshotRestoreResult {
  snapshot_id: string;
  workspace_id: string;
  environments: { created: number; skipped_existing: number };
  chaos_campaigns: { created: number; skipped_existing: number };
  errors: { kind: string; name?: string; error: string }[];
  note: string;
}

class CloudSnapshotsApi {
  private guard(method: string): void {
    if (!isCloudMode()) {
      throw new Error(`Cloud snapshots ${method} only works in cloud mode.`);
    }
  }

  async listForWorkspace(workspaceId: string, limit?: number): Promise<Snapshot[]> {
    this.guard('listForWorkspace');
    const qs = limit ? `?limit=${limit}` : '';
    return fetchJsonWithErrorBody(
      `/api/v1/workspaces/${workspaceId}/snapshots${qs}`,
    ) as Promise<Snapshot[]>;
  }

  async capture(workspaceId: string, body: CaptureSnapshotRequest): Promise<Snapshot> {
    this.guard('capture');
    return fetchJsonWithErrorBody(
      `/api/v1/workspaces/${workspaceId}/snapshots`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      },
    ) as Promise<Snapshot>;
  }

  async get(id: string): Promise<Snapshot> {
    this.guard('get');
    return fetchJsonWithErrorBody(
      `/api/v1/snapshots/${id}`,
    ) as Promise<Snapshot>;
  }

  async delete(id: string): Promise<{ deleted: boolean }> {
    this.guard('delete');
    return fetchJsonWithErrorBody(`/api/v1/snapshots/${id}`, {
      method: 'DELETE',
    }) as Promise<{ deleted: boolean }>;
  }

  /** against = 'current' or another snapshot UUID. */
  async diff(id: string, against: string = 'current'): Promise<SnapshotDiff> {
    this.guard('diff');
    return fetchJsonWithErrorBody(
      `/api/v1/snapshots/${id}/diff?against=${encodeURIComponent(against)}`,
    ) as Promise<SnapshotDiff>;
  }

  async restore(id: string): Promise<SnapshotRestoreResult> {
    this.guard('restore');
    return fetchJsonWithErrorBody(`/api/v1/snapshots/${id}/restore`, {
      method: 'POST',
    }) as Promise<SnapshotRestoreResult>;
  }
}

export const cloudSnapshotsApi = new CloudSnapshotsApi();
