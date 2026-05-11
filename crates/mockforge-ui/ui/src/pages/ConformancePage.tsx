import React, { useState, useCallback, useRef, useEffect } from 'react';
import {
  Play, CheckCircle, XCircle, Clock, RefreshCw, Trash2,
  ChevronDown, ChevronRight, Download, AlertTriangle,
} from 'lucide-react';
import {
  PageHeader,
  ModernCard,
  ModernBadge,
  Section,
} from '../components/ui/DesignSystem';
import { Button } from '../components/ui/button';
import { Input } from '../components/ui/input';
import {
  startConformanceRun,
  getConformanceRun,
  listConformanceRuns,
  deleteConformanceRun,
  streamConformanceProgress,
} from '../services/conformanceApi';
import { cloudConformanceApi } from '../services/cloudConformanceApi';
import { isCloudMode } from '../utils/cloudMode';
import { useWorkspaceStore } from '../stores/useWorkspaceStore';
import type { TestRunStatus } from '../services/api/cloudTestRuns';
import type {
  ConformanceRun,
  ConformanceRunRequest,
  ConformanceRunSummary,
  RunStatus,
  CategoryResult,
  FailureDetail,
} from '../types/conformance';

/** Translate cloud TestRun.status into the local RunStatus vocabulary the page renders against. */
function cloudStatusToRunStatus(status: TestRunStatus): RunStatus {
  switch (status) {
    case 'queued':
      return 'pending';
    case 'running':
      return 'running';
    case 'passed':
      return 'completed';
    case 'failed':
    case 'cancelled':
    case 'errored':
      return 'failed';
  }
}

const CATEGORIES = [
  'Parameters', 'Request Bodies', 'Response Codes', 'Schema Types',
  'Composition', 'String Formats', 'Constraints', 'Security',
  'HTTP Methods', 'Content Types', 'Response Validation',
];

function statusBadge(status: RunStatus) {
  switch (status) {
    case 'pending':
      return <ModernBadge variant="default"><Clock className="h-3 w-3 mr-1" />Pending</ModernBadge>;
    case 'running':
      return <ModernBadge variant="info"><RefreshCw className="h-3 w-3 mr-1 animate-spin" />Running</ModernBadge>;
    case 'completed':
      return <ModernBadge variant="success"><CheckCircle className="h-3 w-3 mr-1" />Completed</ModernBadge>;
    case 'failed':
      return <ModernBadge variant="destructive"><XCircle className="h-3 w-3 mr-1" />Failed</ModernBadge>;
  }
}

function rateColor(rate: number): string {
  if (rate >= 90) return 'text-success-600 dark:text-success-400';
  if (rate >= 70) return 'text-warning-600 dark:text-warning-400';
  return 'text-danger-600 dark:text-danger-400';
}

export function ConformancePage() {
  const cloudMode = isCloudMode();
  const activeWorkspace = useWorkspaceStore((s) => s.activeWorkspace);

  // Config state
  const [targetUrl, setTargetUrl] = useState('');
  const [basePath, setBasePath] = useState('');
  const [apiKey, setApiKey] = useState('');
  const [basicAuth, setBasicAuth] = useState('');
  const [skipTls, setSkipTls] = useState(false);
  const [allOperations, setAllOperations] = useState(false);
  const [selectedCategories, setSelectedCategories] = useState<string[]>([]);
  const [showAdvanced, setShowAdvanced] = useState(false);
  // Inline custom-checks YAML (#391 Phase 2). Wire-only on local
  // mode (no form field — the local CLI passes it via --custom-checks
  // file path). In cloud mode the textarea below surfaces it.
  const [customChecksYaml, setCustomChecksYaml] = useState('');

  // Run state
  const [activeRunId, setActiveRunId] = useState<string | null>(null);
  const [activeRun, setActiveRun] = useState<ConformanceRun | null>(null);
  const [runs, setRuns] = useState<ConformanceRunSummary[]>([]);
  const [isStarting, setIsStarting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [expandedFailures, setExpandedFailures] = useState<Set<string>>(new Set());

  const eventSourceRef = useRef<EventSource | null>(null);
  const pollIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  // Load runs on mount. Cloud mode doesn't have a per-page run list —
  // CloudTestRunsPage owns that surface — so we skip it.
  useEffect(() => {
    if (cloudMode) return;
    listConformanceRuns().then(setRuns).catch(() => {});
  }, [cloudMode]);

  // Clean up SSE and polling on unmount
  useEffect(() => {
    return () => {
      eventSourceRef.current?.close();
      if (pollIntervalRef.current) clearInterval(pollIntervalRef.current);
    };
  }, []);

  const handleStart = useCallback(async () => {
    if (!targetUrl.trim()) {
      setError('Target URL is required');
      return;
    }
    if (cloudMode && !activeWorkspace?.id) {
      setError('Select an active workspace before running cloud conformance.');
      return;
    }
    setError(null);
    setIsStarting(true);

    try {
      // Only forward custom_checks_yaml in cloud mode — the local
      // CLI/server accepts it but takes the YAML from a --custom-checks
      // file path, not an inline string, so passing it through here
      // would land as an unused field on the local API.
      const trimmedCustomYaml = customChecksYaml.trim();
      const config: ConformanceRunRequest = {
        target_url: targetUrl.trim(),
        ...(basePath && { base_path: basePath }),
        ...(apiKey && { api_key: apiKey }),
        ...(basicAuth && { basic_auth: basicAuth }),
        ...(skipTls && { skip_tls_verify: true }),
        ...(allOperations && { all_operations: true }),
        ...(selectedCategories.length > 0 && { categories: selectedCategories }),
        ...(cloudMode && trimmedCustomYaml
          ? { custom_checks_yaml: trimmedCustomYaml }
          : {}),
      };

      if (cloudMode) {
        // Cloud path: create a transient suite + trigger a run, tail
        // SSE event_types onto the same `activeRun` shape the local
        // path produces. Final report comes either from the
        // `finished` event payload or, if missed, from
        // GET /api/v1/test-runs/{id}.summary.report.
        const { run } = await cloudConformanceApi.startRun(activeWorkspace!.id, config);
        const id = run.id;
        setActiveRunId(id);
        // Seed an active run so the page has something to render
        // while we wait for the first event.
        setActiveRun({
          id,
          status: cloudStatusToRunStatus(run.status),
          config,
          checks_done: 0,
          total_checks: 0,
        });

        eventSourceRef.current?.close();
        const es = cloudConformanceApi.streamProgress(
          id,
          (event) => {
            setActiveRun((prev) => {
              if (!prev) return prev;
              switch (event.type) {
                case 'started':
                  return { ...prev, status: 'running', total_checks: event.total_checks };
                case 'check_completed':
                  return { ...prev, status: 'running', checks_done: event.checks_done };
                case 'finished': {
                  const finished = event as { type: 'finished'; report?: unknown };
                  const reportTyped =
                    (finished.report as ConformanceRun['report']) ?? prev.report;
                  return {
                    ...prev,
                    status: 'completed',
                    report: reportTyped,
                  };
                }
                case 'error':
                  return { ...prev, status: 'failed', error: event.message };
              }
            });
          },
          () => {
            // SSE error / disconnect — fall back to polling the run row.
            cloudConformanceApi
              .getRun(id)
              .then((tr) => {
                setActiveRun((prev) => {
                  if (!prev) return prev;
                  const next: ConformanceRun = {
                    ...prev,
                    status: cloudStatusToRunStatus(tr.status),
                  };
                  const summary = tr.summary as Record<string, unknown> | null;
                  const report = summary?.report as ConformanceRun['report'] | undefined;
                  if (report) next.report = report;
                  return next;
                });
              })
              .catch(() => {});
          },
        );
        eventSourceRef.current = es;

        // Backstop polling — same cadence as the local path. Closes
        // the SSE + interval once the run reaches terminal status.
        if (pollIntervalRef.current) clearInterval(pollIntervalRef.current);
        pollIntervalRef.current = setInterval(() => {
          cloudConformanceApi
            .getRun(id)
            .then((tr) => {
              const status = cloudStatusToRunStatus(tr.status);
              setActiveRun((prev) => {
                if (!prev) return prev;
                const next: ConformanceRun = { ...prev, status };
                const summary = tr.summary as Record<string, unknown> | null;
                const report = summary?.report as ConformanceRun['report'] | undefined;
                if (report) next.report = report;
                return next;
              });
              if (status === 'completed' || status === 'failed') {
                es.close();
                if (pollIntervalRef.current) clearInterval(pollIntervalRef.current);
              }
            })
            .catch(() => {});
        }, 2000);
        return;
      }

      const { id } = await startConformanceRun(config);
      setActiveRunId(id);

      // Start SSE stream (best-effort — EventSource doesn't support auth headers)
      eventSourceRef.current?.close();
      const es = streamConformanceProgress(
        id,
        () => {
          getConformanceRun(id).then(setActiveRun).catch(() => {});
        },
        () => {
          if (es) {
            getConformanceRun(id).then(run => {
              setActiveRun(run);
              if (run.status === 'completed' || run.status === 'failed') {
                es.close();
                if (pollIntervalRef.current) clearInterval(pollIntervalRef.current);
                listConformanceRuns().then(setRuns).catch(() => {});
              }
            }).catch(() => {});
          }
        }
      );
      eventSourceRef.current = es;

      // Polling fallback (SSE may fail due to auth, or is null in cloud mode)
      if (pollIntervalRef.current) clearInterval(pollIntervalRef.current);
      pollIntervalRef.current = setInterval(() => {
        getConformanceRun(id).then(run => {
          setActiveRun(run);
          if (run.status === 'completed' || run.status === 'failed') {
            es?.close();
            if (pollIntervalRef.current) clearInterval(pollIntervalRef.current);
            listConformanceRuns().then(setRuns).catch(() => {});
          }
        }).catch(() => {});
      }, 2000);

      // Initial poll
      const run = await getConformanceRun(id);
      setActiveRun(run);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to start run');
    } finally {
      setIsStarting(false);
    }
  }, [
    targetUrl,
    basePath,
    apiKey,
    basicAuth,
    skipTls,
    allOperations,
    selectedCategories,
    cloudMode,
    activeWorkspace?.id,
    customChecksYaml,
  ]);

  const handleViewRun = useCallback(async (id: string) => {
    try {
      const run = await getConformanceRun(id);
      setActiveRunId(id);
      setActiveRun(run);
    } catch {
      setError('Failed to load run');
    }
  }, []);

  const handleDeleteRun = useCallback(async (id: string) => {
    try {
      await deleteConformanceRun(id);
      setRuns(prev => prev.filter(r => r.id !== id));
      if (activeRunId === id) {
        setActiveRunId(null);
        setActiveRun(null);
      }
    } catch {
      setError('Failed to delete run');
    }
  }, [activeRunId]);

  const toggleCategory = (cat: string) => {
    setSelectedCategories(prev =>
      prev.includes(cat) ? prev.filter(c => c !== cat) : [...prev, cat]
    );
  };

  const toggleFailure = (name: string) => {
    setExpandedFailures(prev => {
      const next = new Set(prev);
      if (next.has(name)) next.delete(name); else next.add(name);
      return next;
    });
  };

  const exportJson = () => {
    if (!activeRun?.report) return;
    const blob = new Blob([JSON.stringify(activeRun.report, null, 2)], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `conformance-report-${activeRunId}.json`;
    a.click();
    URL.revokeObjectURL(url);
  };

  const report = activeRun?.report;
  const categories = report?.categories as Record<string, CategoryResult> | undefined;
  const failures = report?.failures as FailureDetail[] | undefined;
  const summary = report?.summary as { total_checks: number; passed: number; failed: number; overall_rate: number } | undefined;

  return (
    <div className="space-y-6">
      <PageHeader
        title="Conformance Testing"
        subtitle={
          cloudMode
            ? 'OpenAPI 3.0 conformance probes dispatched through the cloud test-runner'
            : 'Run OpenAPI 3.0 conformance tests against your mock server'
        }
      />

      {cloudMode && !activeWorkspace && (
        <div className="rounded-lg border border-warning-200 dark:border-warning-800 bg-warning-50 dark:bg-warning-900/20 p-4 flex items-center gap-2">
          <AlertTriangle className="h-4 w-4 text-warning-600 dark:text-warning-400" />
          <span className="text-sm">
            Select an active workspace from the workspace switcher before triggering a cloud
            conformance run.
          </span>
        </div>
      )}

      {error && (
        <div className="rounded-lg border border-danger-200 dark:border-danger-800 bg-danger-50 dark:bg-danger-900/20 p-4 flex items-center gap-2">
          <AlertTriangle className="h-4 w-4 text-danger-600 dark:text-danger-400" />
          <span className="text-sm text-danger-700 dark:text-danger-300">{error}</span>
          <button onClick={() => setError(null)} className="ml-auto text-danger-600 hover:text-danger-700">
            <XCircle className="h-4 w-4" />
          </button>
        </div>
      )}

      {/* Configuration */}
      <Section title="Configuration">
        <ModernCard>
          <div className="p-6 space-y-4">
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div>
                <label className="block text-sm font-medium text-foreground mb-1">
                  Target URL *
                </label>
                <Input
                  placeholder="http://localhost:3000"
                  value={targetUrl}
                  onChange={e => setTargetUrl(e.target.value)}
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-foreground mb-1">
                  Base Path
                </label>
                <Input
                  placeholder="/api/v1"
                  value={basePath}
                  onChange={e => setBasePath(e.target.value)}
                />
              </div>
            </div>

            {/* Category filter */}
            <div>
              <label className="block text-sm font-medium text-foreground mb-2">
                Categories {selectedCategories.length > 0 && `(${selectedCategories.length} selected)`}
              </label>
              <div className="flex flex-wrap gap-2">
                {CATEGORIES.map(cat => (
                  <button
                    key={cat}
                    onClick={() => toggleCategory(cat)}
                    className={`px-3 py-1 rounded-full text-xs font-medium transition-colors ${
                      selectedCategories.includes(cat)
                        ? 'bg-primary text-primary-foreground'
                        : 'bg-muted text-foreground hover:bg-gray-200 dark:hover:bg-gray-700'
                    }`}
                  >
                    {cat}
                  </button>
                ))}
              </div>
            </div>

            {/* Advanced options */}
            <div>
              <button
                onClick={() => setShowAdvanced(!showAdvanced)}
                className="flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground dark:hover:text-gray-200"
              >
                {showAdvanced ? <ChevronDown className="h-4 w-4" /> : <ChevronRight className="h-4 w-4" />}
                Advanced Options
              </button>
              {showAdvanced && (
                <div className="mt-3 grid grid-cols-1 md:grid-cols-2 gap-4">
                  <div>
                    <label className="block text-sm font-medium text-foreground mb-1">
                      API Key
                    </label>
                    <Input
                      type="password"
                      placeholder="Bearer token or API key"
                      value={apiKey}
                      onChange={e => setApiKey(e.target.value)}
                    />
                  </div>
                  <div>
                    <label className="block text-sm font-medium text-foreground mb-1">
                      Basic Auth
                    </label>
                    <Input
                      placeholder="user:password"
                      value={basicAuth}
                      onChange={e => setBasicAuth(e.target.value)}
                    />
                  </div>
                  <label className="flex items-center gap-2 text-sm text-foreground">
                    <input type="checkbox" checked={skipTls} onChange={e => setSkipTls(e.target.checked)} className="rounded" />
                    Skip TLS verification
                  </label>
                  <label className="flex items-center gap-2 text-sm text-foreground">
                    <input type="checkbox" checked={allOperations} onChange={e => setAllOperations(e.target.checked)} className="rounded" />
                    Test all operations
                  </label>
                </div>
              )}

              {/*
                Custom checks YAML (#391 Phase 2). Cloud-only surface
                because the local CLI takes this as a file path; the
                cloud runner accepts the inline YAML and parses it
                server-side via the same declarative schema.
              */}
              {showAdvanced && cloudMode && (
                <div className="mt-3">
                  <label className="block text-sm font-medium text-foreground mb-1">
                    Custom checks (YAML)
                  </label>
                  <p className="text-xs text-muted-foreground mb-2">
                    Declarative checks layered on top of the built-in OpenAPI probes.
                    See the docs for the <code>custom_checks</code> schema (name, path,
                    method, expected_status, expected_headers, expected_body_fields).
                  </p>
                  <textarea
                    className="w-full min-h-[140px] p-2 font-mono text-xs border rounded bg-background text-foreground"
                    placeholder={'custom_checks:\n  - name: "custom:health-200"\n    path: /health\n    method: GET\n    expected_status: 200'}
                    value={customChecksYaml}
                    onChange={e => setCustomChecksYaml(e.target.value)}
                  />
                </div>
              )}
            </div>

            <div className="flex items-center gap-3 pt-2">
              <Button
                onClick={handleStart}
                disabled={isStarting || !targetUrl.trim()}
                className="flex items-center gap-2"
              >
                <Play className="h-4 w-4" />
                {isStarting ? 'Starting...' : 'Run Conformance Tests'}
              </Button>
            </div>
          </div>
        </ModernCard>
      </Section>

      {/* Progress */}
      {activeRun && (activeRun.status === 'pending' || activeRun.status === 'running') && (
        <Section title="Progress">
          <ModernCard>
            <div className="p-6 space-y-3">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  {statusBadge(activeRun.status)}
                  <span className="text-sm text-muted-foreground">
                    {activeRun.checks_done} / {activeRun.total_checks} checks
                  </span>
                </div>
              </div>
              {activeRun.total_checks > 0 && (
                <div className="w-full bg-muted rounded-full h-2">
                  <div
                    className="bg-primary h-2 rounded-full transition-all duration-300"
                    style={{ width: `${(activeRun.checks_done / activeRun.total_checks) * 100}%` }}
                  />
                </div>
              )}
            </div>
          </ModernCard>
        </Section>
      )}

      {/* Results */}
      {activeRun && report && summary && (
        <Section title="Results">
          {/* Summary cards */}
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-4">
            <ModernCard>
              <div className="p-4 text-center">
                <div className="text-2xl font-bold text-foreground">{summary.total_checks}</div>
                <div className="text-xs text-muted-foreground">Total Checks</div>
              </div>
            </ModernCard>
            <ModernCard>
              <div className="p-4 text-center">
                <div className="text-2xl font-bold text-success-600 dark:text-success-400">{summary.passed}</div>
                <div className="text-xs text-muted-foreground">Passed</div>
              </div>
            </ModernCard>
            <ModernCard>
              <div className="p-4 text-center">
                <div className="text-2xl font-bold text-danger-600 dark:text-danger-400">{summary.failed}</div>
                <div className="text-xs text-muted-foreground">Failed</div>
              </div>
            </ModernCard>
            <ModernCard>
              <div className="p-4 text-center">
                <div className={`text-2xl font-bold ${rateColor(summary.overall_rate)}`}>
                  {summary.overall_rate.toFixed(1)}%
                </div>
                <div className="text-xs text-muted-foreground">Pass Rate</div>
              </div>
            </ModernCard>
          </div>

          {/* Category breakdown */}
          {categories && (
            <ModernCard>
              <div className="p-6">
                <div className="flex items-center justify-between mb-4">
                  <h3 className="text-sm font-semibold text-foreground">Category Results</h3>
                  <Button variant="outline" size="sm" onClick={exportJson} className="flex items-center gap-1">
                    <Download className="h-3 w-3" />
                    Export JSON
                  </Button>
                </div>
                <div className="overflow-x-auto">
                  <table className="w-full text-sm">
                    <thead>
                      <tr className="border-b border-border">
                        <th className="text-left py-2 px-3 text-muted-foreground font-medium">Category</th>
                        <th className="text-right py-2 px-3 text-muted-foreground font-medium">Passed</th>
                        <th className="text-right py-2 px-3 text-muted-foreground font-medium">Total</th>
                        <th className="text-right py-2 px-3 text-muted-foreground font-medium">Rate</th>
                        <th className="py-2 px-3 text-muted-foreground font-medium w-32"></th>
                      </tr>
                    </thead>
                    <tbody>
                      {Object.entries(categories).map(([name, cat]) => (
                        <tr key={name} className="border-b border-border">
                          <td className="py-2 px-3 text-foreground">{name}</td>
                          <td className="py-2 px-3 text-right text-success-600 dark:text-success-400">{cat.passed}</td>
                          <td className="py-2 px-3 text-right text-muted-foreground">{cat.total}</td>
                          <td className={`py-2 px-3 text-right font-medium ${rateColor(cat.rate)}`}>
                            {cat.rate.toFixed(1)}%
                          </td>
                          <td className="py-2 px-3">
                            <div className="w-full bg-muted rounded-full h-1.5">
                              <div
                                className={`h-1.5 rounded-full ${cat.rate >= 90 ? 'bg-success-500' : cat.rate >= 70 ? 'bg-warning-500' : 'bg-danger-500'}`}
                                style={{ width: `${cat.rate}%` }}
                              />
                            </div>
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </div>
            </ModernCard>
          )}

          {/* Failure details */}
          {failures && failures.length > 0 && (
            <ModernCard className="mt-4">
              <div className="p-6">
                <h3 className="text-sm font-semibold text-foreground mb-4">
                  Failed Checks ({failures.length})
                </h3>
                <div className="space-y-2">
                  {failures.map((f, i) => {
                    const key = `${f.check_name}-${i}`;
                    const isExpanded = expandedFailures.has(key);
                    return (
                      <div key={key} className="border border-border rounded-lg">
                        <button
                          className="w-full flex items-center gap-2 p-3 text-left hover:bg-accent hover:text-accent-foreground/50"
                          onClick={() => toggleFailure(key)}
                        >
                          {isExpanded
                            ? <ChevronDown className="h-4 w-4 text-muted-foreground" />
                            : <ChevronRight className="h-4 w-4 text-muted-foreground" />
                          }
                          <XCircle className="h-4 w-4 text-danger-500" />
                          <span className="text-sm font-medium text-foreground">{f.check_name}</span>
                          <ModernBadge variant="default" className="ml-auto">{f.category}</ModernBadge>
                        </button>
                        {isExpanded && (
                          <div className="px-10 pb-3 space-y-1 text-xs text-muted-foreground">
                            <div><span className="font-medium">Expected:</span> {f.expected}</div>
                            <div><span className="font-medium">Actual:</span> {f.actual}</div>
                            {f.details && <div><span className="font-medium">Details:</span> {f.details}</div>}
                          </div>
                        )}
                      </div>
                    );
                  })}
                </div>
              </div>
            </ModernCard>
          )}

          {/* Error display */}
          {activeRun.status === 'failed' && activeRun.error && (
            <ModernCard className="mt-4">
              <div className="p-6">
                <div className="flex items-center gap-2 text-danger-600 dark:text-danger-400 mb-2">
                  <AlertTriangle className="h-4 w-4" />
                  <h3 className="text-sm font-semibold">Run Failed</h3>
                </div>
                <p className="text-sm text-foreground">{activeRun.error}</p>
              </div>
            </ModernCard>
          )}
        </Section>
      )}

      {/* Recent Runs (local-only — cloud users see history under Cloud Test Runs) */}
      {!cloudMode && runs.length > 0 && (
        <Section title="Recent Runs">
          <ModernCard>
            <div className="p-6">
              <div className="overflow-x-auto">
                <table className="w-full text-sm">
                  <thead>
                    <tr className="border-b border-border">
                      <th className="text-left py-2 px-3 text-muted-foreground font-medium">ID</th>
                      <th className="text-left py-2 px-3 text-muted-foreground font-medium">Target</th>
                      <th className="text-left py-2 px-3 text-muted-foreground font-medium">Status</th>
                      <th className="text-right py-2 px-3 text-muted-foreground font-medium">Progress</th>
                      <th className="py-2 px-3"></th>
                    </tr>
                  </thead>
                  <tbody>
                    {runs.map(run => (
                      <tr key={run.id} className="border-b border-border hover:bg-accent hover:text-accent-foreground/30">
                        <td className="py-2 px-3 font-mono text-xs text-muted-foreground">
                          {run.id.slice(0, 8)}...
                        </td>
                        <td className="py-2 px-3 text-foreground">{run.target_url}</td>
                        <td className="py-2 px-3">{statusBadge(run.status)}</td>
                        <td className="py-2 px-3 text-right text-muted-foreground">
                          {run.checks_done}/{run.total_checks}
                        </td>
                        <td className="py-2 px-3 text-right">
                          <div className="flex items-center gap-1 justify-end">
                            <Button variant="ghost" size="sm" onClick={() => handleViewRun(run.id)}>
                              View
                            </Button>
                            {run.status !== 'running' && (
                              <Button variant="ghost" size="sm" onClick={() => handleDeleteRun(run.id)}>
                                <Trash2 className="h-3 w-3" />
                              </Button>
                            )}
                          </div>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          </ModernCard>
        </Section>
      )}
    </div>
  );
}
