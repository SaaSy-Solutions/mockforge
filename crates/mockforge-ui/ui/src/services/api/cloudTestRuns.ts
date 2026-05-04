/**
 * Cloud test execution API (#4) — suites, runs, schedules, SSE.
 *
 * Wraps `/api/v1/test-suites/*` and `/api/v1/test-runs/*` and
 * `/api/v1/test-schedules/*`.
 */
import { fetchJsonWithErrorBody } from './client';
import { isCloudMode } from '../../utils/cloudMode';

export type TestRunStatus =
  | 'queued'
  | 'running'
  | 'passed'
  | 'failed'
  | 'cancelled'
  | 'errored';

export interface TestRun {
  id: string;
  suite_id: string;
  org_id: string;
  kind: string;
  triggered_by: string;
  triggered_by_user: string | null;
  status: TestRunStatus;
  queued_at: string;
  started_at: string | null;
  finished_at: string | null;
  runner_seconds: number | null;
  summary: Record<string, unknown> | null;
  git_ref: string | null;
  git_sha: string | null;
}

export interface TriggerRunRequest {
  triggered_by?: 'manual' | 'schedule' | 'ci' | 'webhook';
  git_ref?: string;
  git_sha?: string;
}

export interface TestSchedule {
  id: string;
  suite_id: string;
  cron: string;
  timezone: string;
  enabled: boolean;
  last_triggered_at: string | null;
  created_at: string;
  /** Computed by registry-server based on cron + tz; null for disabled rows. */
  next_fire_at: string | null;
}

export interface CreateScheduleRequest {
  cron: string;
  timezone?: string;
}

export interface TestRunStreamEvent {
  type: string;
  data: unknown;
}

class CloudTestRunsApi {
  private guard(method: string): void {
    if (!isCloudMode()) {
      throw new Error(`Cloud test runs ${method} only works in cloud mode.`);
    }
  }

  async triggerRun(suiteId: string, body: TriggerRunRequest = {}): Promise<TestRun> {
    this.guard('triggerRun');
    return fetchJsonWithErrorBody(`/api/v1/test-suites/${suiteId}/runs`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    }) as Promise<TestRun>;
  }

  async listSuiteRuns(suiteId: string, limit?: number): Promise<TestRun[]> {
    this.guard('listSuiteRuns');
    const qs = limit ? `?limit=${limit}` : '';
    return fetchJsonWithErrorBody(
      `/api/v1/test-suites/${suiteId}/runs${qs}`,
    ) as Promise<TestRun[]>;
  }

  async listOrgRuns(
    orgId: string,
    opts?: { status?: TestRunStatus; limit?: number },
  ): Promise<TestRun[]> {
    this.guard('listOrgRuns');
    const params = new URLSearchParams();
    if (opts?.status) params.set('status', opts.status);
    if (opts?.limit) params.set('limit', String(opts.limit));
    const qs = params.toString() ? `?${params}` : '';
    return fetchJsonWithErrorBody(
      `/api/v1/organizations/${orgId}/test-runs${qs}`,
    ) as Promise<TestRun[]>;
  }

  async getRun(id: string): Promise<TestRun> {
    this.guard('getRun');
    return fetchJsonWithErrorBody(
      `/api/v1/test-runs/${id}`,
    ) as Promise<TestRun>;
  }

  async cancelRun(id: string): Promise<TestRun> {
    this.guard('cancelRun');
    return fetchJsonWithErrorBody(`/api/v1/test-runs/${id}/cancel`, {
      method: 'POST',
    }) as Promise<TestRun>;
  }

  /**
   * Open an SSE stream of test_run_events. Caller owns the EventSource and
   * must close it when done. Each `event:` value matches the
   * `event_type` column in test_run_events (`step_start`, `step_pass`,
   * `log`, `metric`, etc.); a final `event: done` carries the run summary.
   */
  streamRunEvents(id: string): EventSource {
    this.guard('streamRunEvents');
    return new EventSource(`/api/v1/test-runs/${id}/stream`);
  }

  // --- schedules -----------------------------------------------------------

  async listSchedules(suiteId: string): Promise<TestSchedule[]> {
    this.guard('listSchedules');
    return fetchJsonWithErrorBody(
      `/api/v1/test-suites/${suiteId}/schedules`,
    ) as Promise<TestSchedule[]>;
  }

  async createSchedule(
    suiteId: string,
    body: CreateScheduleRequest,
  ): Promise<TestSchedule> {
    this.guard('createSchedule');
    return fetchJsonWithErrorBody(
      `/api/v1/test-suites/${suiteId}/schedules`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      },
    ) as Promise<TestSchedule>;
  }

  async setScheduleEnabled(id: string, enabled: boolean): Promise<TestSchedule> {
    this.guard('setScheduleEnabled');
    return fetchJsonWithErrorBody(`/api/v1/test-schedules/${id}`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ enabled }),
    }) as Promise<TestSchedule>;
  }

  async deleteSchedule(id: string): Promise<{ deleted: boolean }> {
    this.guard('deleteSchedule');
    return fetchJsonWithErrorBody(`/api/v1/test-schedules/${id}`, {
      method: 'DELETE',
    }) as Promise<{ deleted: boolean }>;
  }
}

export const cloudTestRunsApi = new CloudTestRunsApi();
