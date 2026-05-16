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

  /**
   * Open a Server-Sent Events stream for the job. The server emits:
   *   - `status_update` — `data: <CloudTestGenerationJob>` whenever the
   *     row's shape changes (status / started_at / finished_at /
   *     has-result / has-error).
   *   - `ping` — keep-alive when nothing has changed (1s cadence).
   *   - `not_found` — job disappeared mid-stream.
   *   - `stream_error` — terminal upstream error.
   *
   * Returns the raw `EventSource` so the caller owns lifecycle. Use
   * {@link subscribeToJobStream} for the common case of mapping
   * `status_update` payloads back to typed jobs + auto-close on
   * terminal status.
   */
  jobStream(workspaceId: string, jobId: string): EventSource {
    const url = `${this.base(workspaceId)}/jobs/${encodeURIComponent(jobId)}/stream`;
    return new EventSource(url, { withCredentials: true });
  }
}

/**
 * Convenience wrapper around {@link CloudTestGeneratorApiService.jobStream}
 * that:
 *   - parses `status_update` payloads back to {@link CloudTestGenerationJob}
 *   - swallows `ping` heartbeats
 *   - closes the EventSource on terminal events (`not_found`,
 *     `stream_error`, or a job whose status is succeeded/failed/cancelled)
 *
 * Returns a teardown callback the caller can invoke to abort early
 * (e.g. on component unmount).
 */
export function subscribeToJobStream(
  workspaceId: string,
  jobId: string,
  handlers: {
    onUpdate?: (job: CloudTestGenerationJob) => void;
    onError?: (reason: string) => void;
    onComplete?: () => void;
  },
): () => void {
  const es = cloudTestGeneratorApi.jobStream(workspaceId, jobId);
  let closed = false;
  const close = () => {
    if (closed) return;
    closed = true;
    es.close();
    handlers.onComplete?.();
  };

  es.addEventListener('status_update', (ev) => {
    try {
      const job = JSON.parse((ev as MessageEvent).data) as CloudTestGenerationJob;
      handlers.onUpdate?.(job);
      if (
        job.status === 'succeeded' ||
        job.status === 'failed' ||
        job.status === 'cancelled'
      ) {
        close();
      }
    } catch (err) {
      handlers.onError?.(err instanceof Error ? err.message : 'parse error');
      close();
    }
  });

  es.addEventListener('not_found', () => {
    handlers.onError?.('Job not found');
    close();
  });

  es.addEventListener('stream_error', (ev) => {
    try {
      const body = JSON.parse((ev as MessageEvent).data) as { error?: string };
      handlers.onError?.(body.error ?? 'stream error');
    } catch {
      handlers.onError?.('stream error');
    }
    close();
  });

  // EventSource auto-reconnects on transport blips. Surface the notice
  // once but don't close — the browser will retry; the caller can call
  // the returned teardown to give up.
  es.onerror = () => {
    handlers.onError?.('stream connection error (auto-retrying)');
  };

  return close;
}

export { CloudTestGeneratorApiService };
export const cloudTestGeneratorApi = new CloudTestGeneratorApiService();
