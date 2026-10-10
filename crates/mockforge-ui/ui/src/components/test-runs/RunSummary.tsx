import React from 'react';
import type { TestRun } from '../../services/api/cloudTestRuns';

/// Renders the run summary in a structured view when the executor_phase
/// + kind combination has a known shape (real_bench, real_owasp_scan,
/// real_diff, real_hosted_mock, etc.) — falls back to the JSON dump
/// for anything we don't recognize so new shapes still surface
/// something useful.
export const RunSummary: React.FC<{ run: Pick<TestRun, 'kind' | 'summary'> }> = ({ run }) => {
    const summary = run.summary as Record<string, unknown> | null;
    if (!summary) return null;

    const phase = (summary['executor_phase'] as string | undefined) ?? '';
    const kind = run.kind;

    const presenter = SUMMARY_PRESENTERS.get(kind);
    if (presenter && (!presenter.phases || presenter.phases.includes(phase))) {
        const Component = presenter.component;
        return <Component summary={summary} />;
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


const SUMMARY_PRESENTERS = new Map<string, { component: React.FC<{ summary: Record<string, unknown> }>; phases?: readonly string[] }>([
    ['bench', { component: BenchSummary, phases: ['real_bench'] }],
    ['owasp', { component: OwaspSummary, phases: ['real_owasp_scan'] }],
    ['contract_diff', { component: ContractSummary, phases: ['real_diff', 'real_fetch'] }],
    ['chaos_campaign', { component: ChaosSummary }],
    ['replay', { component: ReplaySummary, phases: ['real'] }],
]);
