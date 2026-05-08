/**
 * Cloud smoke-test API (#392).
 *
 * Wraps `POST /api/v1/hosted-mocks/{deployment_id}/smoke-runs`. The
 * trigger creates a `kind='smoke'` test_run and dispatches it to the
 * runner queue. Live progress is streamed back through the existing
 * `cloudTestRunsApi.streamRunEvents(runId)` SSE endpoint as
 * `route_pass` / `route_fail` / `route_skipped` / `log` events — see
 * the runner-side `SmokeTestExecutor` for the event payload shapes.
 *
 * No separate `listSmokeRuns` here; smoke runs surface in the same
 * `cloudTestRunsApi.listOrgRuns(...)` history as every other kind, so
 * the cloud-test-runs page already shows them with no extra wiring.
 */
import { fetchJsonWithErrorBody } from './client';
import { isCloudMode } from '../../utils/cloudMode';
import type { TestRun } from './cloudTestRuns';

/**
 * Optional overrides accepted by the trigger endpoint. All fields fall
 * through to the runner executor's defaults when omitted (5000 ms
 * latency budget, GET-only methods).
 */
export interface TriggerSmokeRunRequest {
  /** Per-route latency assertion ceiling, in milliseconds. */
  latencyBudgetMs?: number;
  /**
   * HTTP methods to probe. Default `['GET']`. POST/PUT/PATCH would need
   * a body source; only GET has been thought through end-to-end in v1.
   */
  methods?: string[];
}

class CloudSmokeApi {
  private guard(method: string): void {
    if (!isCloudMode()) {
      throw new Error(`Cloud smoke ${method} only works in cloud mode.`);
    }
  }

  /**
   * Queue a smoke run against a hosted-mock deployment. Returns the
   * `TestRun` row in `queued` status; subscribe to live events via
   * `cloudTestRunsApi.streamRunEvents(run.id)`.
   *
   * Pre-flight checks the registry handler enforces (any of these
   * surface as a 4xx with a clear message rather than a queued-but-
   * doomed run):
   *   - Deployment must be in `running` status.
   *   - Deployment must have `deployment_url` (a public base URL).
   *   - Deployment must have `openapi_spec_url` (route source).
   *   - Plan must allow test execution (`max_concurrent_runs > 0`).
   *   - Org's in-flight run cap (shared with every other test_run
   *     kind) must have headroom.
   */
  async triggerRun(
    deploymentId: string,
    body: TriggerSmokeRunRequest = {},
  ): Promise<TestRun> {
    this.guard('triggerRun');
    return fetchJsonWithErrorBody(
      `/api/v1/hosted-mocks/${encodeURIComponent(deploymentId)}/smoke-runs`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      },
    ) as Promise<TestRun>;
  }
}

export const cloudSmokeApi = new CloudSmokeApi();
