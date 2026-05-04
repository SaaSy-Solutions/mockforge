/**
 * Cloud recorder proxy API — Phase 5.
 *
 * Wraps `/api/v1/cloud-runs/recorder-proxy/*`. Sessions are pinned to
 * an upstream URL; users point their clients at the returned proxy
 * path and the registry forwards each request, persisting both halves
 * for inspection.
 */
import { fetchJsonWithErrorBody } from './client';
import { isCloudMode } from '../../utils/cloudMode';

export interface CloudProxySession {
  id: string;
  org_id: string;
  workspace_id: string | null;
  /** Treat as sensitive — anyone with the token can drive the proxy. */
  session_token: string;
  upstream_url: string;
  name: string | null;
  created_by: string | null;
  created_at: string;
  expires_at: string;
  revoked_at: string | null;
  capture_count: number;
  total_bytes: number;
}

export interface SessionWithProxyUrl extends CloudProxySession {
  /** Concatenated path the user pastes into their client. */
  proxy_path: string;
}

export interface CreateSessionRequest {
  upstream_url: string;
  workspace_id?: string;
  name?: string;
  ttl_hours?: number;
}

export interface CloudProxyCapture {
  id: number;
  session_id: string;
  org_id: string;
  occurred_at: string;
  method: string;
  path: string;
  query_string: string | null;
  request_headers: string;
  request_body: string | null;
  request_body_encoding: string;
  request_body_truncated: boolean;
  request_size_bytes: number;
  response_status: number | null;
  response_headers: string | null;
  response_body: string | null;
  response_body_encoding: string | null;
  response_body_truncated: boolean;
  response_size_bytes: number | null;
  duration_ms: number;
  upstream_error: string | null;
  client_ip: string | null;
}

class CloudProxyApi {
  private guard(method: string): void {
    if (!isCloudMode()) {
      throw new Error(`Cloud proxy ${method} only works in cloud mode.`);
    }
  }

  async createSession(body: CreateSessionRequest): Promise<SessionWithProxyUrl> {
    this.guard('createSession');
    return fetchJsonWithErrorBody('/api/v1/cloud-runs/recorder-proxy/sessions', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    }) as Promise<SessionWithProxyUrl>;
  }

  async listSessions(limit?: number): Promise<CloudProxySession[]> {
    this.guard('listSessions');
    const qs = limit ? `?limit=${limit}` : '';
    return fetchJsonWithErrorBody(
      `/api/v1/cloud-runs/recorder-proxy/sessions${qs}`,
    ) as Promise<CloudProxySession[]>;
  }

  async getSession(id: string): Promise<CloudProxySession> {
    this.guard('getSession');
    return fetchJsonWithErrorBody(
      `/api/v1/cloud-runs/recorder-proxy/sessions/${id}`,
    ) as Promise<CloudProxySession>;
  }

  async deleteSession(id: string): Promise<void> {
    this.guard('deleteSession');
    await fetchJsonWithErrorBody(
      `/api/v1/cloud-runs/recorder-proxy/sessions/${id}`,
      { method: 'DELETE' },
    );
  }

  async listCaptures(sessionId: string, limit?: number): Promise<CloudProxyCapture[]> {
    this.guard('listCaptures');
    const qs = limit ? `?limit=${limit}` : '';
    return fetchJsonWithErrorBody(
      `/api/v1/cloud-runs/recorder-proxy/sessions/${sessionId}/captures${qs}`,
    ) as Promise<CloudProxyCapture[]>;
  }
}

export const cloudProxyApi = new CloudProxyApi();
