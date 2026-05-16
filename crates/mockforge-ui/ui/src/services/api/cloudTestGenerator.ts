/**
 * Cloud Test Generator API client (#469) — Phase 1.
 *
 * Backs the registry's `/api/v1/workspaces/{workspace_id}/test-generation/jobs`
 * routes. Each row represents an async LLM job that, in Phase 2, will be
 * picked up by a background worker that calls the org's BYOK provider key
 * against a corpus of recent `runtime_captures` rows and writes generated
 * test scenarios into the `result` column.
 *
 * Phase 1 (this client) covers the 4 data-plane endpoints:
 * - `create` — POST a job, lands in 'queued' state
 * - `list`   — newest-first paginated listing (capped 100 server-side)
 * - `get`    — status poll for a specific job
 * - `cancel` — flips a queued/running job to 'cancelled'
 *
 * Polling cadence: a job stuck in 'queued' or 'running' should be polled
 * at ~2-second intervals. Terminal states ('succeeded' | 'failed' |
 * 'cancelled') are stable — stop polling and render the `result` blob
 * (success) or the `error` string (failed/cancelled).
 */
import { fetchJsonWithErrorBody } from './client';

export type TestGenerationJobStatus =
  | 'queued'
  | 'running'
  | 'succeeded'
  | 'failed'
  | 'cancelled';

export interface CloudTestGenerationJob {
  id: string;
  workspace_id: string;
  org_id: string;
  status: TestGenerationJobStatus;
  prompt: string;
  /** Free-form filter object — vocabulary owned by the Phase 2 worker. */
  captures_filter: Record<string, unknown>;
  /** Populated once status = 'succeeded'. */
  result: unknown | null;
  /** Populated once status = 'failed' or 'cancelled'. */
  error: string | null;
  queued_at: string;
  started_at: string | null;
  finished_at: string | null;
  created_by: string | null;
}

/** Request body for `POST .../jobs`. */
export interface CreateTestGenerationJobRequest {
  /** Empty allowed — Phase 2 worker falls back to a captures-derived default. */
  prompt?: string;
  /** JSON object filtering which captures to feed the LLM. Optional. */
  captures_filter?: Record<string, unknown>;
}

class CloudTestGeneratorApiService {
  private base(workspaceId: string): string {
    return `/api/v1/workspaces/${encodeURIComponent(workspaceId)}/test-generation`;
  }

  /** Create a new generation job. Returns the queued row. */
  async createJob(
    workspaceId: string,
    request: CreateTestGenerationJobRequest = {},
  ): Promise<CloudTestGenerationJob> {
    return fetchJsonWithErrorBody(`${this.base(workspaceId)}/jobs`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as Promise<CloudTestGenerationJob>;
  }

  /** List jobs for the workspace, newest first. Server caps at 100. */
  async listJobs(workspaceId: string): Promise<CloudTestGenerationJob[]> {
    return fetchJsonWithErrorBody(`${this.base(workspaceId)}/jobs`) as Promise<
      CloudTestGenerationJob[]
    >;
  }

  /** Status poll for a specific job. */
  async getJob(workspaceId: string, jobId: string): Promise<CloudTestGenerationJob> {
    return fetchJsonWithErrorBody(
      `${this.base(workspaceId)}/jobs/${encodeURIComponent(jobId)}`,
    ) as Promise<CloudTestGenerationJob>;
  }

  /**
   * Cancel a queued or running job. No-op on a terminal job. Returns
   * `{ cancelled: true }` on a state change, `{ cancelled: false }` if
   * the job was already terminal.
   */
  async cancelJob(workspaceId: string, jobId: string): Promise<{ cancelled: boolean }> {
    return fetchJsonWithErrorBody(
      `${this.base(workspaceId)}/jobs/${encodeURIComponent(jobId)}/cancel`,
      {
        method: 'POST',
      },
    ) as Promise<{ cancelled: boolean }>;
  }
}

export { CloudTestGeneratorApiService };
export const cloudTestGeneratorApi = new CloudTestGeneratorApiService();
