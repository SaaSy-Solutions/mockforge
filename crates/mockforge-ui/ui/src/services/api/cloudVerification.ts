/**
 * Cloud-mode request verification client (#390).
 *
 * Wraps the registry's `/api/v1/workspaces/{id}/request-log/*` surface,
 * which mirrors the local `/__mockforge/verification/*` API but sources
 * its log entries from the workspace's `runtime_captures` table instead
 * of the in-process ring buffer. Body-pattern + header matching only
 * works against deployments with the recorder enabled.
 */
import { fetchJsonWithErrorBody } from './client';
import { isCloudMode } from '../../utils/cloudMode';
import type { VerificationRequest, VerificationCount, VerificationResult } from '../../types';

export interface TimeWindow {
  /** RFC3339 timestamp; defaults to now() - 1h on the server. */
  since?: string;
  /** RFC3339 timestamp; defaults to now() on the server. */
  until?: string;
}

export interface WorkspaceCaptureStatus {
  /** Whether at least one capture row was written in the last hour. */
  has_captures: boolean;
  /** Total capture rows in the last hour (rough sanity-check number). */
  recent_capture_count: number;
}

class CloudVerificationApi {
  private guard(method: string): void {
    if (!isCloudMode()) {
      throw new Error(`Cloud verification ${method} only works in cloud mode.`);
    }
  }

  async status(workspaceId: string): Promise<WorkspaceCaptureStatus> {
    this.guard('status');
    return fetchJsonWithErrorBody(
      `/api/v1/workspaces/${workspaceId}/request-log/status`,
    ) as Promise<WorkspaceCaptureStatus>;
  }

  async verify(
    workspaceId: string,
    pattern: VerificationRequest,
    expected: VerificationCount,
    window?: TimeWindow,
  ): Promise<VerificationResult> {
    this.guard('verify');
    return fetchJsonWithErrorBody(
      `/api/v1/workspaces/${workspaceId}/request-log/verify`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ pattern, expected, ...window }),
      },
    ) as Promise<VerificationResult>;
  }

  async count(
    workspaceId: string,
    pattern: VerificationRequest,
    window?: TimeWindow,
  ): Promise<{ count: number }> {
    this.guard('count');
    return fetchJsonWithErrorBody(
      `/api/v1/workspaces/${workspaceId}/request-log/count`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ pattern, ...window }),
      },
    ) as Promise<{ count: number }>;
  }

  async verifySequence(
    workspaceId: string,
    patterns: VerificationRequest[],
    window?: TimeWindow,
  ): Promise<VerificationResult> {
    this.guard('verifySequence');
    return fetchJsonWithErrorBody(
      `/api/v1/workspaces/${workspaceId}/request-log/sequence`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ patterns, ...window }),
      },
    ) as Promise<VerificationResult>;
  }

  async verifyNever(
    workspaceId: string,
    pattern: VerificationRequest,
    window?: TimeWindow,
  ): Promise<VerificationResult> {
    this.guard('verifyNever');
    return fetchJsonWithErrorBody(
      `/api/v1/workspaces/${workspaceId}/request-log/never`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ pattern, ...window }),
      },
    ) as Promise<VerificationResult>;
  }

  async verifyAtLeast(
    workspaceId: string,
    pattern: VerificationRequest,
    min: number,
    window?: TimeWindow,
  ): Promise<VerificationResult> {
    this.guard('verifyAtLeast');
    return fetchJsonWithErrorBody(
      `/api/v1/workspaces/${workspaceId}/request-log/at-least`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ pattern, min, ...window }),
      },
    ) as Promise<VerificationResult>;
  }
}

export const cloudVerificationApi = new CloudVerificationApi();
