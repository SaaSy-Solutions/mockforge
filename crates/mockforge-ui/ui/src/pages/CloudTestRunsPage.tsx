/**
 * Cloud Test Runs — org-wide run history with live event tailing (#4).
 *
 * Shows the test_runs table for the org plus a detail panel that opens
 * an SSE EventSource against /api/v1/test-runs/{id}/stream so an
 * operator can watch a queued run progress without polling. Once a run
 * reaches terminal status the stream's final 'done' event triggers a
 * summary refresh.
 */
import React, { useEffect, useRef, useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { RefreshCw, Square, Play, ChevronRight, Activity } from 'lucide-react';
import { isCloudMode } from '../utils/cloudMode';
import { useCloudOrgId } from '../hooks/useCloudOrgId';
import {
    cloudTestRunsApi,
    type TestRun,
    type TestRunStatus,
} from '../services/api/cloudTestRuns';

const STATUS_STYLES: Record<TestRunStatus, string> = {
    queued:
        'bg-gray-100 text-gray-700 border-gray-200 dark:bg-gray-800 dark:text-gray-400 dark:border-gray-700',
    running:
        'bg-blue-50 text-blue-700 border-blue-200 dark:bg-blue-900/20 dark:text-blue-400 dark:border-blue-900/30',
    passed:
        'bg-green-50 text-green-700 border-green-200 dark:bg-green-900/20 dark:text-green-400 dark:border-green-900/30',
    failed:
        'bg-red-50 text-red-700 border-red-200 dark:bg-red-900/20 dark:text-red-400 dark:border-red-900/30',
    cancelled:
        'bg-gray-100 text-gray-700 border-gray-200 dark:bg-gray-800 dark:text-gray-400 dark:border-gray-700',
    errored:
        'bg-red-50 text-red-700 border-red-200 dark:bg-red-900/20 dark:text-red-400 dark:border-red-900/30',
};

export const CloudTestRunsPage: React.FC = () => {
    if (!isCloudMode()) {
        return (
            <div className="p-6 max-w-7xl mx-auto">
                <div className="bg-blue-50 dark:bg-blue-900/20 text-blue-800 dark:text-blue-300 p-4 rounded-lg">
                    Cloud test runs only fire in cloud mode (the runner pool is part of the cloud
                    infra). Self-hosted users invoke tests via{' '}
                    <code className="font-mono text-xs">cargo test</code> directly.
                </div>
            </div>
        );
    }
    return <CloudView />;
};

const CloudView: React.FC = () => {
    const orgId = useCloudOrgId();
    const queryClient = useQueryClient();
    const [statusFilter, setStatusFilter] = useState<TestRunStatus | 'all'>('all');
    const [selected, setSelected] = useState<TestRun | null>(null);

    const runsQuery = useQuery({
        queryKey: ['cloud', 'test-runs', orgId, statusFilter],
        queryFn: () =>
            cloudTestRunsApi.listOrgRuns(orgId!, {
                status: statusFilter === 'all' ? undefined : statusFilter,
                limit: 100,
            }),
        enabled: !!orgId,
        refetchInterval: 5000,
    });

    const cancelMutation = useMutation({
        mutationFn: (id: string) => cloudTestRunsApi.cancelRun(id),
        onSuccess: () =>
            queryClient.invalidateQueries({ queryKey: ['cloud', 'test-runs'] }),
    });

    if (!orgId) {
        return (
            <div className="p-6 max-w-7xl mx-auto">
                <div className="bg-yellow-50 dark:bg-yellow-900/20 text-yellow-800 dark:text-yellow-300 p-4 rounded-lg">
                    Loading organization context…
                </div>
            </div>
        );
    }

    const runs = runsQuery.data ?? [];

    return (
        <div className="p-6 max-w-7xl mx-auto">
            <div className="flex justify-between items-start mb-6">
                <div>
                    <h1 className="text-2xl font-bold text-gray-900 dark:text-gray-100 mb-2">Test Runs</h1>
                    <p className="text-gray-600 dark:text-gray-400">
                        Cross-suite history. Open a run to tail its events live.
                    </p>
                </div>
                <button
                    onClick={() => runsQuery.refetch()}
                    className="flex items-center px-3 py-2 border border-gray-300 dark:border-gray-600 hover:bg-gray-50 dark:hover:bg-gray-700 rounded-lg text-sm"
                    disabled={runsQuery.isFetching}
                >
                    <RefreshCw className={`w-4 h-4 mr-2 ${runsQuery.isFetching ? 'animate-spin' : ''}`} />
                    Refresh
                </button>
            </div>

            <div className="mb-4 flex gap-2 flex-wrap">
                {(['all', 'queued', 'running', 'passed', 'failed', 'cancelled', 'errored'] as const).map(
                    (s) => (
                        <button
                            key={s}
                            onClick={() => setStatusFilter(s)}
                            className={`px-3 py-1.5 text-sm rounded-lg border ${
                                statusFilter === s
                                    ? 'bg-blue-600 text-white border-blue-600'
                                    : 'bg-white dark:bg-gray-800 border-gray-300 dark:border-gray-600 hover:bg-gray-50 dark:hover:bg-gray-700'
                            }`}
                        >
                            {s}
                        </button>
                    ),
                )}
            </div>

            {runsQuery.isError && (
                <div className="mb-4 bg-red-50 dark:bg-red-900/20 text-red-700 dark:text-red-400 p-4 rounded-lg text-sm">
                    {(runsQuery.error as Error).message}
                </div>
            )}

            {runs.length === 0 && !runsQuery.isLoading ? (
                <div className="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-12 text-center">
                    <Activity className="w-16 h-16 mx-auto text-gray-400 mb-4" />
                    <h3 className="text-lg font-medium text-gray-900 dark:text-gray-100 mb-2">No runs</h3>
                    <p className="text-gray-500 dark:text-gray-400">
                        Trigger a suite run via{' '}
                        <code className="font-mono text-xs">mockforge cloud test run &lt;suite-id&gt;</code> or
                        the suite editor.
                    </p>
                </div>
            ) : (
                <div className="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 overflow-hidden">
                    <table className="w-full text-left text-sm">
                        <thead className="bg-gray-50 dark:bg-gray-900/50 border-b border-gray-200 dark:border-gray-700">
                            <tr>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Run</th>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Kind</th>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Status</th>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Trigger</th>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Duration</th>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Queued</th>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400 text-right">Actions</th>
                            </tr>
                        </thead>
                        <tbody className="divide-y divide-gray-200 dark:divide-gray-700">
                            {runs.map((r) => (
                                <RunRow
                                    key={r.id}
                                    run={r}
                                    onView={() => setSelected(r)}
                                    onCancel={() => {
                                        if (confirm('Cancel this run?')) cancelMutation.mutate(r.id);
                                    }}
                                />
                            ))}
                        </tbody>
                    </table>
                </div>
            )}

            {selected && <RunDetailPanel run={selected} onClose={() => setSelected(null)} />}
        </div>
    );
};

const RunRow: React.FC<{
    run: TestRun;
    onView: () => void;
    onCancel: () => void;
}> = ({ run, onView, onCancel }) => {
    const inflight = run.status === 'queued' || run.status === 'running';
    return (
        <tr className="hover:bg-gray-50 dark:hover:bg-gray-800/50 cursor-pointer" onClick={onView}>
            <td className="px-6 py-4 font-mono text-xs text-gray-600 dark:text-gray-300">
                {run.id.slice(0, 8)}
            </td>
            <td className="px-6 py-4 text-gray-700 dark:text-gray-300">{run.kind}</td>
            <td className="px-6 py-4">
                <span
                    className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium border ${STATUS_STYLES[run.status]}`}
                >
                    {run.status}
                </span>
            </td>
            <td className="px-6 py-4 text-xs text-gray-600 dark:text-gray-300">{run.triggered_by}</td>
            <td className="px-6 py-4 text-gray-600 dark:text-gray-300">
                {run.runner_seconds != null ? `${run.runner_seconds}s` : '—'}
            </td>
            <td className="px-6 py-4 text-xs text-gray-600 dark:text-gray-300">
                {new Date(run.queued_at).toLocaleString()}
            </td>
            <td className="px-6 py-4 text-right space-x-1" onClick={(e) => e.stopPropagation()}>
                {inflight && (
                    <button
                        onClick={onCancel}
                        className="p-2 text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20 rounded-lg"
                        title="Cancel"
                    >
                        <Square className="w-4 h-4" />
                    </button>
                )}
                <ChevronRight className="w-4 h-4 inline text-gray-400" />
            </td>
        </tr>
    );
};

interface StreamEvent {
    type: string;
    data: unknown;
    received_at: string;
}

/// Renders the run summary in a structured view when the executor_phase
/// + kind combination has a known shape (real_bench, real_owasp_scan,
/// real_diff, real_hosted_mock, etc.) — falls back to the JSON dump
/// for anything we don't recognize so new shapes still surface
/// something useful.
const SummaryPanel: React.FC<{ run: TestRun }> = ({ run }) => {
    const summary = run.summary as Record<string, unknown> | null;
    if (!summary) return null;

    const phase = (summary['executor_phase'] as string | undefined) ?? '';
    const kind = run.kind;

    if (kind === 'bench' && phase === 'real_bench') {
        return <BenchSummary summary={summary} />;
    }
    if (kind === 'owasp' && phase === 'real_owasp_scan') {
        return <OwaspSummary summary={summary} />;
    }
    if (kind === 'contract_diff' && (phase === 'real_diff' || phase === 'real_fetch')) {
        return <ContractSummary summary={summary} />;
    }
    if (kind === 'chaos_campaign') {
        return <ChaosSummary summary={summary} />;
    }
    if (kind === 'replay' && phase === 'real') {
        return <ReplaySummary summary={summary} />;
    }
    return (
        <details className="bg-gray-50 dark:bg-gray-900/50 rounded p-3">
            <summary className="cursor-pointer text-sm font-medium">Run summary</summary>
            <pre className="text-xs mt-2 overflow-x-auto">{JSON.stringify(summary, null, 2)}</pre>
        </details>
    );
};

const BenchSummary: React.FC<{ summary: Record<string, unknown> }> = ({ summary }) => {
    const total = (summary['total_requests'] as number | undefined) ?? 0;
    const ok = (summary['ok'] as number | undefined) ?? 0;
    const errs = (summary['errors'] as number | undefined) ?? 0;
    const errRate = (summary['error_rate_pct'] as number | undefined) ?? 0;
    const p50 = (summary['p50_ms'] as number | undefined) ?? 0;
    const p95 = (summary['p95_ms'] as number | undefined) ?? 0;
    const p99 = (summary['p99_ms'] as number | undefined) ?? 0;
    const concurrency = (summary['concurrency'] as number | undefined) ?? 0;
    const duration = (summary['duration_secs'] as number | undefined) ?? 0;
    const target = (summary['target_url'] as string | undefined) ?? '';

    const card = (label: string, value: string, color: string) => (
        <div className="bg-white dark:bg-gray-900 rounded border border-gray-200 dark:border-gray-700 p-3">
            <div className="text-xs text-gray-500 dark:text-gray-400 mb-1">{label}</div>
            <div className={`text-lg font-mono font-bold ${color}`}>{value}</div>
        </div>
    );

    return (
        <div className="space-y-3">
            <div className="flex items-center gap-2 text-sm font-medium text-gray-700 dark:text-gray-300">
                Bench results
                <span className="text-xs text-gray-500 font-mono">{target}</span>
            </div>
            <div className="text-xs text-gray-500">
                {concurrency} concurrent workers for {duration}s · {total.toLocaleString()} total
                requests
            </div>
            <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
                {card('p50', `${p50.toFixed(1)}ms`, 'text-green-600 dark:text-green-400')}
                {card(
                    'p95',
                    `${p95.toFixed(1)}ms`,
                    p95 > 1000
                        ? 'text-red-600 dark:text-red-400'
                        : 'text-yellow-600 dark:text-yellow-400',
                )}
                {card('p99', `${p99.toFixed(1)}ms`, 'text-orange-600 dark:text-orange-400')}
                {card(
                    'error rate',
                    `${errRate.toFixed(2)}%`,
                    errRate >= 1
                        ? 'text-red-600 dark:text-red-400'
                        : 'text-green-600 dark:text-green-400',
                )}
            </div>
            <div className="text-xs text-gray-500">
                ✓ {ok.toLocaleString()} ok &middot; ✗ {errs.toLocaleString()} errors
            </div>
        </div>
    );
};

const OwaspSummary: React.FC<{ summary: Record<string, unknown> }> = ({ summary }) => {
    const passed = (summary['passed'] as number | undefined) ?? 0;
    const failed = (summary['failed'] as number | undefined) ?? 0;
    const total = (summary['checks_total'] as number | undefined) ?? passed + failed;
    const score = total > 0 ? Math.round((passed / total) * 100) : 0;
    const target = (summary['target_url'] as string | undefined) ?? '';
    const targetStatus = summary['target_status_code'] as number | undefined;

    return (
        <div className="space-y-3">
            <div className="flex items-center gap-2 text-sm font-medium text-gray-700 dark:text-gray-300">
                OWASP header scan
                <span className="text-xs text-gray-500 font-mono">{target}</span>
                {targetStatus && (
                    <span className="text-xs text-gray-500">→ HTTP {targetStatus}</span>
                )}
            </div>
            <div className="bg-white dark:bg-gray-900 rounded border border-gray-200 dark:border-gray-700 p-4">
                <div className="flex items-end justify-between">
                    <div>
                        <div className="text-xs text-gray-500 dark:text-gray-400 mb-1">Score</div>
                        <div
                            className={`text-3xl font-mono font-bold ${
                                score >= 80
                                    ? 'text-green-600 dark:text-green-400'
                                    : score >= 50
                                    ? 'text-yellow-600 dark:text-yellow-400'
                                    : 'text-red-600 dark:text-red-400'
                            }`}
                        >
                            {score}%
                        </div>
                    </div>
                    <div className="text-right text-sm">
                        <div className="text-green-600 dark:text-green-400">✓ {passed} present</div>
                        <div className="text-red-600 dark:text-red-400">✗ {failed} missing</div>
                    </div>
                </div>
            </div>
            <div className="text-xs text-gray-500">
                See the event stream below for per-header pass/fail with remediation advice.
            </div>
        </div>
    );
};

const ContractSummary: React.FC<{ summary: Record<string, unknown> }> = ({ summary }) => {
    const declared = (summary['declared_count'] as number | undefined) ?? 0;
    const unused = (summary['unused_count'] as number | undefined) ?? 0;
    const undeclared = (summary['undeclared_count'] as number | undefined) ?? 0;
    const endpointCount = (summary['endpoint_count'] as number | undefined) ?? 0;
    const trafficCount = (summary['traffic_endpoint_count'] as number | undefined) ?? 0;
    const service = (summary['service_name'] as string | undefined) ?? '';

    return (
        <div className="space-y-3">
            <div className="flex items-center gap-2 text-sm font-medium text-gray-700 dark:text-gray-300">
                Contract diff
                <span className="text-xs text-gray-500 font-mono">{service}</span>
            </div>
            <div className="grid grid-cols-3 gap-3">
                <div className="bg-white dark:bg-gray-900 rounded border border-gray-200 dark:border-gray-700 p-3">
                    <div className="text-xs text-gray-500 dark:text-gray-400 mb-1">Spec endpoints</div>
                    <div className="text-lg font-mono font-bold">{endpointCount}</div>
                    <div className="text-xs text-gray-500 mt-1">
                        {declared} hit · {unused} unused
                    </div>
                </div>
                <div className="bg-white dark:bg-gray-900 rounded border border-gray-200 dark:border-gray-700 p-3">
                    <div className="text-xs text-gray-500 dark:text-gray-400 mb-1">Traffic endpoints</div>
                    <div className="text-lg font-mono font-bold">{trafficCount}</div>
                    <div className="text-xs text-gray-500 mt-1">last 24h</div>
                </div>
                <div
                    className={`bg-white dark:bg-gray-900 rounded border p-3 ${
                        undeclared > 0
                            ? 'border-red-200 dark:border-red-900/30'
                            : 'border-gray-200 dark:border-gray-700'
                    }`}
                >
                    <div className="text-xs text-gray-500 dark:text-gray-400 mb-1">Drift</div>
                    <div
                        className={`text-lg font-mono font-bold ${
                            undeclared > 0
                                ? 'text-red-600 dark:text-red-400'
                                : 'text-green-600 dark:text-green-400'
                        }`}
                    >
                        {undeclared}
                    </div>
                    <div className="text-xs text-gray-500 mt-1">undeclared in spec</div>
                </div>
            </div>
        </div>
    );
};

const ChaosSummary: React.FC<{ summary: Record<string, unknown> }> = ({ summary }) => {
    const real = (summary['real_chaos_enabled'] as boolean | undefined) ?? false;
    const aborted = (summary['aborted'] as boolean | undefined) ?? false;
    const reason = summary['abort_reason'] as string | null | undefined;
    const faults = (summary['fault_count'] as number | undefined) ?? 0;
    const target = (summary['target_ref'] as string | undefined) ?? '';

    return (
        <div className="space-y-3">
            <div className="flex items-center gap-2 text-sm font-medium text-gray-700 dark:text-gray-300">
                Chaos campaign
                <span className="text-xs text-gray-500 font-mono">{target}</span>
                {real && (
                    <span className="text-xs px-2 py-0.5 rounded-full bg-orange-100 text-orange-800 dark:bg-orange-900/30 dark:text-orange-400">
                        live target
                    </span>
                )}
            </div>
            <div className="grid grid-cols-3 gap-3">
                <div className="bg-white dark:bg-gray-900 rounded border border-gray-200 dark:border-gray-700 p-3">
                    <div className="text-xs text-gray-500 dark:text-gray-400 mb-1">Faults</div>
                    <div className="text-lg font-mono font-bold">{faults}</div>
                </div>
                <div
                    className={`bg-white dark:bg-gray-900 rounded border p-3 ${
                        aborted
                            ? 'border-red-200 dark:border-red-900/30'
                            : 'border-green-200 dark:border-green-900/30'
                    }`}
                >
                    <div className="text-xs text-gray-500 dark:text-gray-400 mb-1">Aborted</div>
                    <div
                        className={`text-lg font-mono font-bold ${
                            aborted
                                ? 'text-red-600 dark:text-red-400'
                                : 'text-green-600 dark:text-green-400'
                        }`}
                    >
                        {aborted ? 'yes' : 'no'}
                    </div>
                </div>
                <div className="bg-white dark:bg-gray-900 rounded border border-gray-200 dark:border-gray-700 p-3">
                    <div className="text-xs text-gray-500 dark:text-gray-400 mb-1">Mode</div>
                    <div className="text-sm font-mono">
                        {real ? 'real_hosted_mock' : (summary['executor_phase'] as string) ?? '—'}
                    </div>
                </div>
            </div>
            {aborted && reason && (
                <div className="bg-red-50 dark:bg-red-900/20 text-red-700 dark:text-red-400 p-3 rounded text-xs">
                    <span className="font-medium">Abort reason: </span>
                    {reason}
                </div>
            )}
        </div>
    );
};

const ReplaySummary: React.FC<{ summary: Record<string, unknown> }> = ({ summary }) => {
    const matched = (summary['matched'] as number | undefined) ?? 0;
    const mismatched = (summary['mismatched'] as number | undefined) ?? 0;
    const errored = (summary['errored'] as number | undefined) ?? 0;
    const total = (summary['captures_replayed'] as number | undefined) ?? 0;
    const target = (summary['target_url'] as string | undefined) ?? '';

    return (
        <div className="space-y-3">
            <div className="flex items-center gap-2 text-sm font-medium text-gray-700 dark:text-gray-300">
                Capture replay
                <span className="text-xs text-gray-500 font-mono">{target}</span>
            </div>
            <div className="grid grid-cols-4 gap-3">
                <div className="bg-white dark:bg-gray-900 rounded border border-gray-200 dark:border-gray-700 p-3">
                    <div className="text-xs text-gray-500 dark:text-gray-400 mb-1">Total</div>
                    <div className="text-lg font-mono font-bold">{total}</div>
                </div>
                <div className="bg-white dark:bg-gray-900 rounded border border-green-200 dark:border-green-900/30 p-3">
                    <div className="text-xs text-gray-500 dark:text-gray-400 mb-1">Matched</div>
                    <div className="text-lg font-mono font-bold text-green-600 dark:text-green-400">
                        {matched}
                    </div>
                </div>
                <div
                    className={`bg-white dark:bg-gray-900 rounded border p-3 ${
                        mismatched > 0
                            ? 'border-yellow-200 dark:border-yellow-900/30'
                            : 'border-gray-200 dark:border-gray-700'
                    }`}
                >
                    <div className="text-xs text-gray-500 dark:text-gray-400 mb-1">Mismatched</div>
                    <div
                        className={`text-lg font-mono font-bold ${
                            mismatched > 0
                                ? 'text-yellow-600 dark:text-yellow-400'
                                : 'text-gray-400'
                        }`}
                    >
                        {mismatched}
                    </div>
                </div>
                <div
                    className={`bg-white dark:bg-gray-900 rounded border p-3 ${
                        errored > 0
                            ? 'border-red-200 dark:border-red-900/30'
                            : 'border-gray-200 dark:border-gray-700'
                    }`}
                >
                    <div className="text-xs text-gray-500 dark:text-gray-400 mb-1">Errored</div>
                    <div
                        className={`text-lg font-mono font-bold ${
                            errored > 0 ? 'text-red-600 dark:text-red-400' : 'text-gray-400'
                        }`}
                    >
                        {errored}
                    </div>
                </div>
            </div>
        </div>
    );
};

const RunDetailPanel: React.FC<{ run: TestRun; onClose: () => void }> = ({ run, onClose }) => {
    const [events, setEvents] = useState<StreamEvent[]>([]);
    const [streaming, setStreaming] = useState(false);
    const [finalSummary, setFinalSummary] = useState<unknown | null>(null);
    const sourceRef = useRef<EventSource | null>(null);

    useEffect(() => {
        const inflight = run.status === 'queued' || run.status === 'running';
        if (!inflight) return;

        const es = cloudTestRunsApi.streamRunEvents(run.id);
        sourceRef.current = es;
        setStreaming(true);

        const onMessage = (ev: MessageEvent) => {
            try {
                const data = JSON.parse(ev.data);
                setEvents((prev) => [
                    ...prev.slice(-499),
                    { type: ev.type || 'message', data, received_at: new Date().toISOString() },
                ]);
                if (ev.type === 'done') {
                    setFinalSummary(data);
                    setStreaming(false);
                    es.close();
                }
            } catch {
                /* ignore non-JSON ping payloads */
            }
        };

        // Listen for all known event types we emit + the catch-all 'message'.
        for (const t of [
            'log',
            'step_start',
            'step_pass',
            'step_fail',
            'metric',
            'fault_injected',
            'fault_recovered',
            'node_visited',
            'diff_finding',
            'training_epoch',
            'request_replayed',
            'component_dumped',
            'component_restored',
            'ping',
            'done',
            'stream_error',
        ]) {
            es.addEventListener(t, onMessage);
        }
        es.addEventListener('message', onMessage);
        es.onerror = () => {
            setStreaming(false);
        };
        return () => {
            es.close();
            sourceRef.current = null;
        };
    }, [run.id, run.status]);

    return (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm">
            <div className="bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-4xl w-full max-h-[85vh] overflow-y-auto border border-gray-200 dark:border-gray-700">
                <div className="p-6 border-b border-gray-200 dark:border-gray-700 sticky top-0 bg-white dark:bg-gray-800">
                    <div className="flex items-start justify-between">
                        <div>
                            <h2 className="text-xl font-semibold flex items-center gap-2">
                                <Play className="w-5 h-5" />
                                Run {run.id.slice(0, 8)}
                            </h2>
                            <div className="mt-2 flex gap-2 text-xs items-center">
                                <span
                                    className={`inline-flex items-center px-2.5 py-0.5 rounded-full font-medium border ${STATUS_STYLES[run.status]}`}
                                >
                                    {run.status}
                                </span>
                                <span className="text-gray-500">{run.kind}</span>
                                <span className="text-gray-500">via {run.triggered_by}</span>
                                {streaming && (
                                    <span className="text-blue-600 dark:text-blue-400 inline-flex items-center gap-1">
                                        <span className="relative flex h-2 w-2">
                                            <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-blue-400 opacity-75" />
                                            <span className="relative inline-flex rounded-full h-2 w-2 bg-blue-500" />
                                        </span>
                                        live
                                    </span>
                                )}
                            </div>
                        </div>
                        <button onClick={onClose} className="text-gray-400 hover:text-gray-600">
                            ✕
                        </button>
                    </div>
                </div>
                <div className="p-6 space-y-4">
                    {run.summary && <SummaryPanel run={run} />}
                    <div>
                        <h3 className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                            Event stream {streaming ? '(live)' : '(closed)'}
                        </h3>
                        <div className="bg-black/90 text-green-300 dark:text-green-300 rounded p-3 font-mono text-xs max-h-96 overflow-y-auto">
                            {events.length === 0 ? (
                                <div className="text-gray-500 italic">
                                    {streaming
                                        ? 'Waiting for events…'
                                        : 'No events recorded for this run.'}
                                </div>
                            ) : (
                                events
                                    .filter((e) => e.type !== 'ping')
                                    .map((e, idx) => (
                                        <div key={idx} className="mb-1">
                                            <span className="text-gray-500">
                                                {new Date(e.received_at).toLocaleTimeString()}
                                            </span>{' '}
                                            <span className="text-blue-300">{e.type}</span>{' '}
                                            <span>{JSON.stringify(e.data)}</span>
                                        </div>
                                    ))
                            )}
                        </div>
                    </div>
                    {finalSummary != null && (
                        <div className="bg-green-50 dark:bg-green-900/20 text-green-800 dark:text-green-400 p-3 rounded text-xs">
                            <div className="font-medium mb-1">Run complete</div>
                            <pre>{JSON.stringify(finalSummary, null, 2)}</pre>
                        </div>
                    )}
                </div>
            </div>
        </div>
    );
};
