/**
 * Cloud recorder + behavioral cloning API client (#6).
 *
 * Wraps capture session CRUD, member management, training trigger,
 * replay trigger, and clone model read paths.
 */
import { fetchJsonWithErrorBody } from './client';
import { isCloudMode } from '../../utils/cloudMode';

export interface CaptureSession {
  id: string;
  workspace_id: string;
  name: string;
  description: string | null;
  created_by: string | null;
  created_at: string;
  updated_at: string;
}

export interface CloneModel {
  id: string;
  org_id: string;
  workspace_id: string;
  source_session_id: string | null;
  name: string;
  status: string;
  artifact_url: string | null;
  metrics: Record<string, unknown> | null;
  runner_seconds: number | null;
  created_at: string;
}

class CloudRecorderApi {
  private guard(method: string): void {
    if (!isCloudMode()) {
      throw new Error(`Cloud recorder ${method} only works in cloud mode.`);
    }
  }

  async listSessions(workspaceId: string): Promise<CaptureSession[]> {
    this.guard('listSessions');
    return fetchJsonWithErrorBody(
      `/api/v1/workspaces/${workspaceId}/capture-sessions`,
    ) as Promise<CaptureSession[]>;
  }

  async createSession(
    workspaceId: string,
    body: { name: string; description?: string },
  ): Promise<CaptureSession> {
    this.guard('createSession');
    return fetchJsonWithErrorBody(
      `/api/v1/workspaces/${workspaceId}/capture-sessions`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      },
    ) as Promise<CaptureSession>;
  }

  async deleteSession(id: string): Promise<{ deleted: boolean }> {
    this.guard('deleteSession');
    return fetchJsonWithErrorBody(`/api/v1/capture-sessions/${id}`, {
      method: 'DELETE',
    }) as Promise<{ deleted: boolean }>;
  }

  async trainClone(
    sessionId: string,
    body: { name: string },
  ): Promise<CloneModel> {
    this.guard('trainClone');
    return fetchJsonWithErrorBody(
      `/api/v1/capture-sessions/${sessionId}/train`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      },
    ) as Promise<CloneModel>;
  }

  async replaySession(
    sessionId: string,
    body: { target_url?: string; synthetic_captures?: number } = {},
  ): Promise<{ id: string; status: string; kind: string }> {
    this.guard('replaySession');
    return fetchJsonWithErrorBody(
      `/api/v1/capture-sessions/${sessionId}/replay`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      },
    ) as Promise<{ id: string; status: string; kind: string }>;
  }

  async listCloneModels(workspaceId: string): Promise<CloneModel[]> {
    this.guard('listCloneModels');
    return fetchJsonWithErrorBody(
      `/api/v1/workspaces/${workspaceId}/clone-models`,
    ) as Promise<CloneModel[]>;
  }

  async getCloneModel(id: string): Promise<CloneModel> {
    this.guard('getCloneModel');
    return fetchJsonWithErrorBody(
      `/api/v1/clone-models/${id}`,
    ) as Promise<CloneModel>;
  }

  async deleteCloneModel(id: string): Promise<{ deleted: boolean }> {
    this.guard('deleteCloneModel');
    return fetchJsonWithErrorBody(`/api/v1/clone-models/${id}`, {
      method: 'DELETE',
    }) as Promise<{ deleted: boolean }>;
  }
}

export const cloudRecorderApi = new CloudRecorderApi();
