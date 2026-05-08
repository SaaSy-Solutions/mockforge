/**
 * Cloud-mode view of the Testing page (#392).
 *
 * Renders a deployment picker, latency-budget input, and trigger button.
 * Once a smoke run is queued, the inline detail panel streams live
 * `route_pass` / `route_fail` / `route_skipped` / `log` events from the
 * runner via the existing `cloudTestRunsApi.streamRunEvents(runId)` SSE
 * endpoint — smoke runs share the same `test_run_events` infrastructure
 * as every other test_run kind, so no separate streaming surface is
 * needed.
 *
 * Mounted by `TestingPage` when `isCloudMode()` returns true; the local
 * `/__mockforge/smoke` flow is kept untouched on the self-hosted path.
 */
import React, { useEffect, useMemo, useRef, useState } from 'react';
import {
  Play,
  CheckCircle,
  XCircle,
  Clock,
  AlertTriangle,
  RefreshCw,
  Server,
  SkipForward,
} from 'lucide-react';
import { PageHeader, ModernCard, ModernBadge, Section } from '../components/ui/DesignSystem';
import { Button } from '../components/ui/button';
import { Input } from '../components/ui/input';
import { fetchJsonWithErrorBody } from '../services/api/client';
import { cloudSmokeApi } from '../services/api/cloudSmoke';
import { cloudTestRunsApi, type TestRun } from '../services/api/cloudTestRuns';

/**
 * Subset of the hosted-mock row we need for the picker. The full type
 * lives in `HostedMocksPage.tsx` but we only render name, slug, status,
 * and the two URL fields the smoke endpoint needs to be runnable.
 */
interface DeploymentSummary {
  id: string;
  name: string;
  slug: string;
  status: string;
  deployment_url?: string | null;
  openapi_spec_url?: string | null;
}

interface RouteEventPayload {
  path?: string;
  method?: string;
  status?: number | null;
  latency_ms?: number;
  reason?: string;
}

interface StreamEvent {
  type: string;
  payload: Record<string, unknown>;
  receivedAt: string;
}

const DEFAULT_LATENCY_BUDGET_MS = 5_000;

export function CloudSmokeView() {
  const [deployments, setDeployments] = useState<DeploymentSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [latencyBudget, setLatencyBudget] = useState<number>(DEFAULT_LATENCY_BUDGET_MS);
  const [triggering, setTriggering] = useState(false);
  const [activeRun, setActiveRun] = useState<TestRun | null>(null);
  const [triggerError, setTriggerError] = useState<string | null>(null);

  const reloadDeployments = async () => {
    setLoading(true);
    setLoadError(null);
    try {
      const rows = (await fetchJsonWithErrorBody('/api/v1/hosted-mocks')) as DeploymentSummary[];
      // Sort by name for predictable picker order; the listing endpoint
      // doesn't promise any particular order.
      const sorted = [...rows].sort((a, b) => a.name.localeCompare(b.name));
      setDeployments(sorted);
      // Auto-select the first runnable deployment so a fresh visit lands
      // ready to fire — but don't override an explicit selection on
      // refresh.
      setSelectedId((prev) => prev ?? sorted.find(isRunnable)?.id ?? null);
    } catch (err) {
      setLoadError(err instanceof Error ? err.message : 'Failed to load deployments');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void reloadDeployments();
  }, []);

  const selected = useMemo(
    () => deployments.find((d) => d.id === selectedId) ?? null,
    [deployments, selectedId],
  );
  const selectedRunnable = selected ? isRunnable(selected) : false;
  const blockReason = selected ? unrunnableReason(selected) : 'Pick a deployment to begin';

  const handleTrigger = async () => {
    if (!selected || !selectedRunnable) return;
    setTriggering(true);
    setTriggerError(null);
    try {
      const run = await cloudSmokeApi.triggerRun(selected.id, {
        latencyBudgetMs: latencyBudget > 0 ? latencyBudget : undefined,
      });
      setActiveRun(run);
    } catch (err) {
      setTriggerError(err instanceof Error ? err.message : 'Trigger failed');
    } finally {
      setTriggering(false);
    }
  };

  return (
    <div className="space-y-6 p-6">
      <PageHeader
        title="Smoke tests (cloud)"
        description="Probe every declared route on a hosted-mock deployment. Each route gets a 2xx-class assertion plus a per-route latency budget."
        icon={Play}
      />

      <Section
        title="Pick a deployment"
        actions={
          <Button variant="ghost" size="sm" onClick={reloadDeployments} disabled={loading}>
            <RefreshCw className={`w-4 h-4 ${loading ? 'animate-spin' : ''}`} />
            <span className="ml-2">Reload</span>
          </Button>
        }
      >
        {loadError && (
          <ModernCard className="border-red-200 dark:border-red-900/30">
            <div className="flex items-start gap-3 text-red-700 dark:text-red-300">
              <AlertTriangle className="w-5 h-5 mt-0.5" />
              <div>
                <div className="font-medium">Couldn't load deployments</div>
                <div className="text-sm">{loadError}</div>
              </div>
            </div>
          </ModernCard>
        )}

        {!loadError && !loading && deployments.length === 0 && (
          <ModernCard>
            <div className="text-sm text-gray-500 dark:text-gray-400">
              No hosted-mock deployments visible to this account. Create one from the Hosted Mocks
              page first.
            </div>
          </ModernCard>
        )}

        {!loadError && deployments.length > 0 && (
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
            {deployments.map((dep) => {
              const runnable = isRunnable(dep);
              const isPicked = dep.id === selectedId;
              return (
                <button
                  key={dep.id}
                  type="button"
                  onClick={() => setSelectedId(dep.id)}
                  className={`text-left rounded-lg border p-4 transition-colors ${
                    isPicked
                      ? 'border-blue-500 bg-blue-50 dark:bg-blue-900/20'
                      : 'border-gray-200 dark:border-gray-700 hover:border-gray-300 dark:hover:border-gray-600'
                  }`}
                  aria-pressed={isPicked}
                >
                  <div className="flex items-start justify-between gap-2">
                    <div className="min-w-0">
                      <div className="font-medium truncate">{dep.name}</div>
                      <div className="text-xs text-gray-500 dark:text-gray-400 truncate">
                        {dep.slug}
                      </div>
                    </div>
                    <ModernBadge variant={runnable ? 'success' : 'warning'}>
                      {dep.status}
                    </ModernBadge>
                  </div>
                  {!runnable && (
                    <div className="mt-2 text-xs text-yellow-700 dark:text-yellow-400">
                      {unrunnableReason(dep)}
                    </div>
                  )}
                </button>
              );
            })}
          </div>
        )}
      </Section>

      <Section title="Run">
        <ModernCard>
          <div className="flex flex-col sm:flex-row gap-4 items-start sm:items-end">
            <div className="flex-1 min-w-0">
              <label
                htmlFor="latency-budget"
                className="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1"
              >
                Per-route latency budget (ms)
              </label>
              <Input
                id="latency-budget"
                type="number"
                min={1}
                max={60_000}
                value={latencyBudget}
                onChange={(e) => setLatencyBudget(Number(e.target.value) || 0)}
                className="w-40"
              />
              <div className="text-xs text-gray-500 dark:text-gray-400 mt-1">
                Routes that respond slower than this are flagged red even if the status is 2xx.
              </div>
            </div>
            <Button
              onClick={handleTrigger}
              disabled={!selectedRunnable || triggering}
              size="lg"
              title={selectedRunnable ? undefined : blockReason}
            >
              {triggering ? (
                <>
                  <RefreshCw className="w-4 h-4 mr-2 animate-spin" />
                  Queueing…
                </>
              ) : (
                <>
                  <Play className="w-4 h-4 mr-2" />
                  Run smoke
                </>
              )}
            </Button>
          </div>
          {triggerError && (
            <div className="mt-3 text-sm text-red-700 dark:text-red-300">{triggerError}</div>
          )}
        </ModernCard>
      </Section>

      {activeRun && (
        <Section title="Live results">
          <RunDetailPanel run={activeRun} onClose={() => setActiveRun(null)} />
        </Section>
      )}
    </div>
  );
}

// ─── Helpers ─────────────────────────────────────────────────────────

function isRunnable(d: DeploymentSummary): boolean {
  // Mirrors the registry handler's pre-flight gates so the picker can
  // disable buttons at the source rather than surfacing a 400 only at
  // trigger time.
  return (
    d.status === 'active' &&
    typeof d.deployment_url === 'string' &&
    d.deployment_url.length > 0 &&
    typeof d.openapi_spec_url === 'string' &&
    d.openapi_spec_url.length > 0
  );
}

function unrunnableReason(d: DeploymentSummary): string {
  if (d.status !== 'active') {
    return `Status is ${d.status}; smoke needs an active deployment.`;
  }
  if (!d.deployment_url) {
    return 'No public URL yet — wait for the deploy to finish.';
  }
  if (!d.openapi_spec_url) {
    return 'No OpenAPI spec uploaded.';
  }
  return '';
}

// ─── Live event panel ────────────────────────────────────────────────

const RunDetailPanel: React.FC<{ run: TestRun; onClose: () => void }> = ({ run, onClose }) => {
  const [events, setEvents] = useState<StreamEvent[]>([]);
  const [streaming, setStreaming] = useState(false);
  const sourceRef = useRef<EventSource | null>(null);

  useEffect(() => {
    const inflight = run.status === 'queued' || run.status === 'running';
    if (!inflight) return;

    const es = cloudTestRunsApi.streamRunEvents(run.id);
    sourceRef.current = es;
    setStreaming(true);

    const onMessage = (ev: MessageEvent) => {
      try {
        const payload = JSON.parse(ev.data) as Record<string, unknown>;
        setEvents((prev) => [
          // Cap the in-memory buffer at 500 events so a misshapen spec
          // (or a pathological route count) doesn't grow unboundedly.
          ...prev.slice(-499),
          {
            type: ev.type || 'message',
            payload,
            receivedAt: new Date().toISOString(),
          },
        ]);
        if (ev.type === 'done') {
          setStreaming(false);
          es.close();
        }
      } catch {
        /* ignore non-JSON keepalive frames */
      }
    };

    // The smoke executor emits these specific event types; `log` is the
    // generic catch-all (start banner + final summary), `done` signals
    // run completion.
    for (const t of [
      'log',
      'route_pass',
      'route_fail',
      'route_skipped',
      'ping',
      'done',
      'stream_error',
    ]) {
      es.addEventListener(t, onMessage);
    }
    es.addEventListener('message', onMessage);
    es.onerror = () => setStreaming(false);

    return () => {
      es.close();
      sourceRef.current = null;
    };
  }, [run.id, run.status]);

  const tally = useMemo(() => {
    let passed = 0;
    let failed = 0;
    let skipped = 0;
    for (const e of events) {
      if (e.type === 'route_pass') passed++;
      else if (e.type === 'route_fail') failed++;
      else if (e.type === 'route_skipped') skipped++;
    }
    return { passed, failed, skipped };
  }, [events]);

  return (
    <ModernCard>
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center gap-3">
          <div className="font-mono text-xs text-gray-500 dark:text-gray-400">
            run {run.id.slice(0, 8)}
          </div>
          {streaming ? (
            <ModernBadge variant="info">
              <span className="inline-flex items-center gap-1">
                <Clock className="w-3 h-3 animate-pulse" />
                streaming
              </span>
            </ModernBadge>
          ) : (
            <ModernBadge variant="default">closed</ModernBadge>
          )}
        </div>
        <Button variant="ghost" size="sm" onClick={onClose}>
          Dismiss
        </Button>
      </div>

      <div className="grid grid-cols-3 gap-2 mb-4">
        <TallyCard label="Passed" value={tally.passed} accent="green" icon={CheckCircle} />
        <TallyCard label="Failed" value={tally.failed} accent="red" icon={XCircle} />
        <TallyCard label="Skipped" value={tally.skipped} accent="gray" icon={SkipForward} />
      </div>

      <div className="border border-gray-200 dark:border-gray-700 rounded max-h-[60vh] overflow-y-auto divide-y divide-gray-100 dark:divide-gray-800">
        {events.length === 0 && (
          <div className="text-sm text-gray-500 dark:text-gray-400 px-3 py-6 text-center">
            Waiting for the runner to pick up this job…
          </div>
        )}
        {events.map((e, i) => (
          <EventRow key={i} event={e} />
        ))}
      </div>
    </ModernCard>
  );
};

const TallyCard: React.FC<{
  label: string;
  value: number;
  accent: 'green' | 'red' | 'gray';
  icon: React.ComponentType<{ className?: string }>;
}> = ({ label, value, accent, icon: Icon }) => {
  const accentClass = {
    green: value > 0 ? 'text-green-600 dark:text-green-400' : 'text-gray-400',
    red: value > 0 ? 'text-red-600 dark:text-red-400' : 'text-gray-400',
    gray: 'text-gray-500 dark:text-gray-400',
  }[accent];
  return (
    <div className="bg-white dark:bg-gray-900 rounded border border-gray-200 dark:border-gray-700 p-3">
      <div className="text-xs text-gray-500 dark:text-gray-400 mb-1 flex items-center gap-1">
        <Icon className="w-3 h-3" />
        {label}
      </div>
      <div className={`text-lg font-mono font-bold ${accentClass}`}>{value}</div>
    </div>
  );
};

const EventRow: React.FC<{ event: StreamEvent }> = ({ event }) => {
  const ts = event.receivedAt.slice(11, 19);
  if (event.type === 'route_pass' || event.type === 'route_fail' || event.type === 'route_skipped') {
    const r = event.payload as RouteEventPayload;
    const isPass = event.type === 'route_pass';
    const isSkip = event.type === 'route_skipped';
    const colorClass = isPass
      ? 'text-green-700 dark:text-green-400'
      : isSkip
      ? 'text-gray-500 dark:text-gray-400'
      : 'text-red-700 dark:text-red-400';
    return (
      <div className="px-3 py-2 text-sm flex items-center gap-3">
        <span className="font-mono text-xs text-gray-400 w-16 shrink-0">{ts}</span>
        <span
          className={`font-mono text-xs uppercase ${colorClass} w-14 shrink-0`}
          title={event.type}
        >
          {isPass ? 'pass' : isSkip ? 'skip' : 'fail'}
        </span>
        <span className="font-mono text-xs text-gray-600 dark:text-gray-300 w-16 shrink-0">
          {r.method ?? ''}
        </span>
        <span className="font-mono text-xs flex-1 truncate" title={r.path}>
          {r.path ?? ''}
        </span>
        {typeof r.status === 'number' && (
          <span className="font-mono text-xs text-gray-500 w-10 shrink-0 text-right">
            {r.status}
          </span>
        )}
        {typeof r.latency_ms === 'number' && (
          <span className="font-mono text-xs text-gray-500 w-16 shrink-0 text-right">
            {r.latency_ms}ms
          </span>
        )}
        {r.reason && !isPass && (
          <span className="text-xs text-red-600 dark:text-red-400 truncate max-w-[12rem]">
            {r.reason}
          </span>
        )}
      </div>
    );
  }
  // Generic log / done / stream_error fallback — show the message
  // verbatim so a misshaped event still surfaces in the UI.
  return (
    <div className="px-3 py-2 text-xs flex items-center gap-3 text-gray-600 dark:text-gray-400 font-mono">
      <span className="text-gray-400 w-16 shrink-0">{ts}</span>
      <Server className="w-3 h-3 shrink-0" />
      <span className="uppercase w-14 shrink-0 truncate" title={event.type}>
        {event.type}
      </span>
      <span className="flex-1 truncate" title={JSON.stringify(event.payload)}>
        {extractMessage(event.payload)}
      </span>
    </div>
  );
};

function extractMessage(payload: Record<string, unknown>): string {
  // Prefer the runner's `message` field on `log` events; fall back to a
  // compact JSON dump so unfamiliar event shapes still render.
  const msg = payload.message;
  if (typeof msg === 'string') return msg;
  try {
    return JSON.stringify(payload);
  } catch {
    return '[unserializable payload]';
  }
}
