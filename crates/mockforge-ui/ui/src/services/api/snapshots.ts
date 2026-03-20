/**
 * Snapshots API service — snapshot listing, saving, loading, deleting.
 */
import { fetchJsonWithErrorBody } from './client';

class SnapshotsApiService {
  /**
   * List all snapshots for a workspace
   */
  async listSnapshots(workspace = 'default'): Promise<{
    workspace: string;
    snapshots: Array<{
      name: string;
      description: string | null;
      created_at: string;
      workspace: string;
      components: Record<string, boolean>;
    }>;
    count: number;
  }> {
    return fetchJsonWithErrorBody(`/api/v1/snapshots?workspace=${encodeURIComponent(workspace)}`) as Promise<{
      workspace: string;
      snapshots: Array<{
        name: string;
        description: string | null;
        created_at: string;
        workspace: string;
        components: Record<string, boolean>;
      }>;
      count: number;
    }>;
  }

  /**
   * Save a new snapshot
   */
  async saveSnapshot(
    name: string,
    workspace = 'default',
    description?: string
  ): Promise<{ success: boolean; manifest: Record<string, unknown> }> {
    return fetchJsonWithErrorBody(`/api/v1/snapshots?workspace=${encodeURIComponent(workspace)}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ name, description }),
    }) as Promise<{ success: boolean; manifest: Record<string, unknown> }>;
  }

  /**
   * Load a snapshot
   */
  async loadSnapshot(
    name: string,
    workspace = 'default'
  ): Promise<{ success: boolean; manifest: Record<string, unknown> }> {
    return fetchJsonWithErrorBody(`/api/v1/snapshots/${encodeURIComponent(name)}/load?workspace=${encodeURIComponent(workspace)}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({}),
    }) as Promise<{ success: boolean; manifest: Record<string, unknown> }>;
  }

  /**
   * Delete a snapshot
   */
  async deleteSnapshot(
    name: string,
    workspace = 'default'
  ): Promise<{ success: boolean; message: string }> {
    return fetchJsonWithErrorBody(`/api/v1/snapshots/${encodeURIComponent(name)}?workspace=${encodeURIComponent(workspace)}`, {
      method: 'DELETE',
    }) as Promise<{ success: boolean; message: string }>;
  }

  /**
   * Get snapshot info
   */
  async getSnapshotInfo(
    name: string,
    workspace = 'default'
  ): Promise<{ success: boolean; manifest: Record<string, unknown> }> {
    return fetchJsonWithErrorBody(
      `/api/v1/snapshots/${encodeURIComponent(name)}?workspace=${encodeURIComponent(workspace)}`
    ) as Promise<{ success: boolean; manifest: Record<string, unknown> }>;
  }
}

export { SnapshotsApiService };
