/**
 * Cloud-mode conformance dispatch (#391).
 *
 * The local Conformance page hits `/api/conformance/run` directly. In
 * cloud mode there is no equivalent ad-hoc endpoint — runs are
 * test_suite-driven through the existing test_runs lifecycle. This
 * module wraps that flow so the page can dispatch through a single
 * call:
 *
 *   1. Create a transient `kind='conformance'` test_suite under the
 *      active workspace, with the form's config blob.
 *   2. Trigger a run via `POST /api/v1/test-suites/{id}/runs`.
 *   3. Tail SSE via the existing `cloudTestRunsApi.streamRunEvents`,
 *      translating registry event types back into the
 *      `ConformanceProgress` shape the page already understands.
 *   4. On terminal status, fetch the test_run and pull the structured
 *      ConformanceReport out of `summary.report`.
 *
 * The transient suite is created per-run so the UI doesn't require a
 * "manage conformance suites" UX up front. A future iteration can add
 * a "save this configuration" button that promotes a transient suite
 * to a named one.
 */
import { cloudTestRunsApi, type TestSuite, type TestRun } from './api/cloudTestRuns';
import { isCloudMode } from '../utils/cloudMode';
import type {
  ConformanceProgress,
  ConformanceReport,
  ConformanceRunRequest,
} from '../types/conformance';

export interface StartCloudConformanceResult {
  /** The transient test_suite created to hold this run's config. */
  suite: TestSuite;
  /** The triggered test_run row. */
  run: TestRun;
}

/**
 * Build the `test_suite.config` JSON blob the cloud test-runner expects
 * for a conformance run. Field names match the keys the
 * test-runner's `run_cloud_conformance` extractor reads.
 */
function buildConformanceConfig(req: ConformanceRunRequest): Record<string, unknown> {
  // The cloud runner pulls categories as a comma-separated string and
  // headers as a `Vec<String>` of "Name: value" lines, matching the
  // CLI flag shapes that mockforge-bench already accepts.
  const categories = req.categories?.length ? req.categories.join(',') : undefined;
  const conformance_headers = req.custom_headers?.length
    ? req.custom_headers.map(([k, v]) => `${k}: ${v}`)
    : undefined;
  // Trim — empty-string custom_checks_yaml would still hit the
  // server-side parser, which is wasteful and would surface a noisy
  // "EOF reached while parsing YAML" error.
  const customChecksYaml = req.custom_checks_yaml?.trim();
  return {
    use_cloud_api: true,
    target_url: req.target_url,
    base_path: req.base_path,
    conformance_api_key: req.api_key,
    conformance_basic_auth: req.basic_auth,
    conformance_categories: categories,
    conformance_headers,
    conformance_all_operations: req.all_operations ?? false,
    conformance_delay_ms: req.request_delay_ms ?? 0,
    skip_tls_verify: req.skip_tls_verify ?? false,
    ...(customChecksYaml ? { custom_checks_yaml: customChecksYaml } : {}),
  };
}

class CloudConformanceApi {
  private guard(method: string): void {
    if (!isCloudMode()) {
      throw new Error(`Cloud conformance ${method} only works in cloud mode.`);
    }
  }

  /**
   * Create a transient suite + trigger a run for a fresh conformance
   * configuration. Returns the suite + run so the caller can attach
   * an SSE stream to `run.id`.
   */
  async startRun(
    workspaceId: string,
    request: ConformanceRunRequest,
  ): Promise<StartCloudConformanceResult> {
    this.guard('startRun');
    const ts = new Date().toISOString().replace(/[:.]/g, '-');
    const suite = await cloudTestRunsApi.createSuite(workspaceId, {
      name: `Ad-hoc conformance ${ts}`,
      description: `Conformance run against ${request.target_url}`,
      kind: 'conformance',
      config: buildConformanceConfig(request),
    });
    const run = await cloudTestRunsApi.triggerRun(suite.id, { triggered_by: 'manual' });
    return { suite, run };
  }

  /**
   * Open an SSE stream for a cloud conformance run. The runner emits
   * `started` / `check_completed` / `finished` event types matching
   * the local SSE shape; this wrapper subscribes to each and forwards
   * them through `onEvent` so callers don't have to care that the
   * underlying transport is the generic test_run_events stream.
   */
  streamProgress(
    runId: string,
    onEvent: (event: ConformanceProgress) => void,
    onError?: (error: Event) => void,
  ): EventSource {
    this.guard('streamProgress');
    const source = cloudTestRunsApi.streamRunEvents(runId);

    const handle = (eventType: 'started' | 'check_completed' | 'finished' | 'error') => {
      source.addEventListener(eventType, (e: MessageEvent) => {
        try {
          const wrapper = JSON.parse(e.data) as { payload?: unknown };
          const payload = (wrapper?.payload ?? {}) as Record<string, unknown>;
          // The runner emits `{type, ...}` payloads; the SSE wrapper
          // adds `seq`/`occurred_at`/`event_type` siblings. Strip
          // those so the callback sees the same shape as the local
          // streamConformanceProgress path.
          if (typeof payload === 'object' && payload !== null && 'type' in payload) {
            onEvent(payload as unknown as ConformanceProgress);
          }
        } catch {
          /* ignore malformed events */
        }
      });
    };
    handle('started');
    handle('check_completed');
    handle('finished');
    handle('error');

    if (onError) {
      source.onerror = onError;
    }
    return source;
  }

  /**
   * Fetch the terminal `ConformanceReport` from the test_run summary
   * once the run is done. The runner stuffs the full report JSON into
   * `summary.report` plus a duplicate copy in the `finished` SSE
   * event — this is the polling fallback for clients that miss the
   * SSE event.
   */
  async getReport(runId: string): Promise<ConformanceReport | null> {
    this.guard('getReport');
    const run = await cloudTestRunsApi.getRun(runId);
    const summary = run.summary as Record<string, unknown> | null;
    if (!summary) return null;
    const report = summary.report as ConformanceReport | undefined;
    return report ?? null;
  }

  async getRun(runId: string): Promise<TestRun> {
    this.guard('getRun');
    return cloudTestRunsApi.getRun(runId);
  }
}

export const cloudConformanceApi = new CloudConformanceApi();
