import React, { useState, useCallback, useRef, useEffect } from 'react';
import {
  Shield, Play, CheckCircle, XCircle, Clock, RefreshCw, Trash2,
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
import type {
  ConformanceRun,
  ConformanceRunRequest,
  ConformanceRunSummary,
  RunStatus,
  CategoryResult,
  FailureDetail,
} from '../types/conformance';

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
  if (rate >= 90) return 'text-green-600 dark:text-green-400';
  if (rate >= 70) return 'text-yellow-600 dark:text-yellow-400';
  return 'text-red-600 dark:text-red-400';
}

export function ConformancePage() {
  // Config state
  const [targetUrl, setTargetUrl] = useState('');
  const [basePath, setBasePath] = useState('');
  const [apiKey, setApiKey] = useState('');
  const [basicAuth, setBasicAuth] = useState('');
  const [skipTls, setSkipTls] = useState(false);
  const [allOperations, setAllOperations] = useState(false);
  const [selectedCategories, setSelectedCategories] = useState<string[]>([]);
  const [showAdvanced, setShowAdvanced] = useState(false);

  // Run state
  const [activeRunId, setActiveRunId] = useState<string | null>(null);
  const [activeRun, setActiveRun] = useState<ConformanceRun | null>(null);
  const [runs, setRuns] = useState<ConformanceRunSummary[]>([]);
  const [isStarting, setIsStarting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [expandedFailures, setExpandedFailures] = useState<Set<string>>(new Set());

  const eventSourceRef = useRef<EventSource | null>(null);
  const pollIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  // Load runs on mount
  useEffect(() => {
    listConformanceRuns().then(setRuns).catch(() => {});
  }, []);

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
    setError(null);
    setIsStarting(true);

    try {
      const config: ConformanceRunRequest = {
        target_url: targetUrl.trim(),
        ...(basePath && { base_path: basePath }),
        ...(apiKey && { api_key: apiKey }),
        ...(basicAuth && { basic_auth: basicAuth }),
        ...(skipTls && { skip_tls_verify: true }),
        ...(allOperations && { all_operations: true }),
        ...(selectedCategories.length > 0 && { categories: selectedCategories }),
      };

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
          getConformanceRun(id).then(run => {
            setActiveRun(run);
            if (run.status === 'completed' || run.status === 'failed') {
              es.close();
              if (pollIntervalRef.current) clearInterval(pollIntervalRef.current);
              listConformanceRuns().then(setRuns).catch(() => {});
            }
          }).catch(() => {});
        }
      );
      eventSourceRef.current = es;

      // Polling fallback (SSE may fail due to auth)
      if (pollIntervalRef.current) clearInterval(pollIntervalRef.current);
      pollIntervalRef.current = setInterval(() => {
        getConformanceRun(id).then(run => {
          setActiveRun(run);
          if (run.status === 'completed' || run.status === 'failed') {
            es.close();
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
  }, [targetUrl, basePath, apiKey, basicAuth, skipTls, allOperations, selectedCategories]);

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
        description="Run OpenAPI 3.0 conformance tests against your mock server"
        icon={Shield}
      />

      {error && (
        <div className="rounded-lg border border-red-200 dark:border-red-800 bg-red-50 dark:bg-red-900/20 p-4 flex items-center gap-2">
          <AlertTriangle className="h-4 w-4 text-red-600 dark:text-red-400" />
          <span className="text-sm text-red-700 dark:text-red-300">{error}</span>
          <button onClick={() => setError(null)} className="ml-auto text-red-600 hover:text-red-800">
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
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                  Target URL *
                </label>
                <Input
                  placeholder="http://localhost:3000"
                  value={targetUrl}
                  onChange={e => setTargetUrl(e.target.value)}
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
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
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                Categories {selectedCategories.length > 0 && `(${selectedCategories.length} selected)`}
              </label>
              <div className="flex flex-wrap gap-2">
                {CATEGORIES.map(cat => (
                  <button
                    key={cat}
                    onClick={() => toggleCategory(cat)}
                    className={`px-3 py-1 rounded-full text-xs font-medium transition-colors ${
                      selectedCategories.includes(cat)
                        ? 'bg-blue-600 text-white'
                        : 'bg-gray-100 dark:bg-gray-800 text-gray-700 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-700'
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
                className="flex items-center gap-1 text-sm text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-200"
              >
                {showAdvanced ? <ChevronDown className="h-4 w-4" /> : <ChevronRight className="h-4 w-4" />}
                Advanced Options
              </button>
              {showAdvanced && (
                <div className="mt-3 grid grid-cols-1 md:grid-cols-2 gap-4">
                  <div>
                    <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
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
                    <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                      Basic Auth
                    </label>
                    <Input
                      placeholder="user:password"
                      value={basicAuth}
                      onChange={e => setBasicAuth(e.target.value)}
                    />
                  </div>
                  <label className="flex items-center gap-2 text-sm text-gray-700 dark:text-gray-300">
                    <input type="checkbox" checked={skipTls} onChange={e => setSkipTls(e.target.checked)} className="rounded" />
                    Skip TLS verification
                  </label>
                  <label className="flex items-center gap-2 text-sm text-gray-700 dark:text-gray-300">
                    <input type="checkbox" checked={allOperations} onChange={e => setAllOperations(e.target.checked)} className="rounded" />
                    Test all operations
                  </label>
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
                  <span className="text-sm text-gray-600 dark:text-gray-400">
                    {activeRun.checks_done} / {activeRun.total_checks} checks
                  </span>
                </div>
              </div>
              {activeRun.total_checks > 0 && (
                <div className="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-2">
                  <div
                    className="bg-blue-600 h-2 rounded-full transition-all duration-300"
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
                <div className="text-2xl font-bold text-gray-900 dark:text-gray-100">{summary.total_checks}</div>
                <div className="text-xs text-gray-500 dark:text-gray-400">Total Checks</div>
              </div>
            </ModernCard>
            <ModernCard>
              <div className="p-4 text-center">
                <div className="text-2xl font-bold text-green-600 dark:text-green-400">{summary.passed}</div>
                <div className="text-xs text-gray-500 dark:text-gray-400">Passed</div>
              </div>
            </ModernCard>
            <ModernCard>
              <div className="p-4 text-center">
                <div className="text-2xl font-bold text-red-600 dark:text-red-400">{summary.failed}</div>
                <div className="text-xs text-gray-500 dark:text-gray-400">Failed</div>
              </div>
            </ModernCard>
            <ModernCard>
              <div className="p-4 text-center">
                <div className={`text-2xl font-bold ${rateColor(summary.overall_rate)}`}>
                  {summary.overall_rate.toFixed(1)}%
                </div>
                <div className="text-xs text-gray-500 dark:text-gray-400">Pass Rate</div>
              </div>
            </ModernCard>
          </div>

          {/* Category breakdown */}
          {categories && (
            <ModernCard>
              <div className="p-6">
                <div className="flex items-center justify-between mb-4">
                  <h3 className="text-sm font-semibold text-gray-900 dark:text-gray-100">Category Results</h3>
                  <Button variant="outline" size="sm" onClick={exportJson} className="flex items-center gap-1">
                    <Download className="h-3 w-3" />
                    Export JSON
                  </Button>
                </div>
                <div className="overflow-x-auto">
                  <table className="w-full text-sm">
                    <thead>
                      <tr className="border-b border-gray-200 dark:border-gray-700">
                        <th className="text-left py-2 px-3 text-gray-600 dark:text-gray-400 font-medium">Category</th>
                        <th className="text-right py-2 px-3 text-gray-600 dark:text-gray-400 font-medium">Passed</th>
                        <th className="text-right py-2 px-3 text-gray-600 dark:text-gray-400 font-medium">Total</th>
                        <th className="text-right py-2 px-3 text-gray-600 dark:text-gray-400 font-medium">Rate</th>
                        <th className="py-2 px-3 text-gray-600 dark:text-gray-400 font-medium w-32"></th>
                      </tr>
                    </thead>
                    <tbody>
                      {Object.entries(categories).map(([name, cat]) => (
                        <tr key={name} className="border-b border-gray-100 dark:border-gray-800">
                          <td className="py-2 px-3 text-gray-900 dark:text-gray-100">{name}</td>
                          <td className="py-2 px-3 text-right text-green-600 dark:text-green-400">{cat.passed}</td>
                          <td className="py-2 px-3 text-right text-gray-600 dark:text-gray-400">{cat.total}</td>
                          <td className={`py-2 px-3 text-right font-medium ${rateColor(cat.rate)}`}>
                            {cat.rate.toFixed(1)}%
                          </td>
                          <td className="py-2 px-3">
                            <div className="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-1.5">
                              <div
                                className={`h-1.5 rounded-full ${cat.rate >= 90 ? 'bg-green-500' : cat.rate >= 70 ? 'bg-yellow-500' : 'bg-red-500'}`}
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
                <h3 className="text-sm font-semibold text-gray-900 dark:text-gray-100 mb-4">
                  Failed Checks ({failures.length})
                </h3>
                <div className="space-y-2">
                  {failures.map((f, i) => {
                    const key = `${f.check_name}-${i}`;
                    const isExpanded = expandedFailures.has(key);
                    return (
                      <div key={key} className="border border-gray-200 dark:border-gray-700 rounded-lg">
                        <button
                          className="w-full flex items-center gap-2 p-3 text-left hover:bg-gray-50 dark:hover:bg-gray-800/50"
                          onClick={() => toggleFailure(key)}
                        >
                          {isExpanded
                            ? <ChevronDown className="h-4 w-4 text-gray-400" />
                            : <ChevronRight className="h-4 w-4 text-gray-400" />
                          }
                          <XCircle className="h-4 w-4 text-red-500" />
                          <span className="text-sm font-medium text-gray-900 dark:text-gray-100">{f.check_name}</span>
                          <ModernBadge variant="default" className="ml-auto">{f.category}</ModernBadge>
                        </button>
                        {isExpanded && (
                          <div className="px-10 pb-3 space-y-1 text-xs text-gray-600 dark:text-gray-400">
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
                <div className="flex items-center gap-2 text-red-600 dark:text-red-400 mb-2">
                  <AlertTriangle className="h-4 w-4" />
                  <h3 className="text-sm font-semibold">Run Failed</h3>
                </div>
                <p className="text-sm text-gray-700 dark:text-gray-300">{activeRun.error}</p>
              </div>
            </ModernCard>
          )}
        </Section>
      )}

      {/* Recent Runs */}
      {runs.length > 0 && (
        <Section title="Recent Runs">
          <ModernCard>
            <div className="p-6">
              <div className="overflow-x-auto">
                <table className="w-full text-sm">
                  <thead>
                    <tr className="border-b border-gray-200 dark:border-gray-700">
                      <th className="text-left py-2 px-3 text-gray-600 dark:text-gray-400 font-medium">ID</th>
                      <th className="text-left py-2 px-3 text-gray-600 dark:text-gray-400 font-medium">Target</th>
                      <th className="text-left py-2 px-3 text-gray-600 dark:text-gray-400 font-medium">Status</th>
                      <th className="text-right py-2 px-3 text-gray-600 dark:text-gray-400 font-medium">Progress</th>
                      <th className="py-2 px-3"></th>
                    </tr>
                  </thead>
                  <tbody>
                    {runs.map(run => (
                      <tr key={run.id} className="border-b border-gray-100 dark:border-gray-800 hover:bg-gray-50 dark:hover:bg-gray-800/30">
                        <td className="py-2 px-3 font-mono text-xs text-gray-600 dark:text-gray-400">
                          {run.id.slice(0, 8)}...
                        </td>
                        <td className="py-2 px-3 text-gray-900 dark:text-gray-100">{run.target_url}</td>
                        <td className="py-2 px-3">{statusBadge(run.status)}</td>
                        <td className="py-2 px-3 text-right text-gray-600 dark:text-gray-400">
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
