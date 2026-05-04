/**
 * Cloud Traces — cross-deployment trace search (#2).
 *
 * Wraps the POST /observability/traces/query endpoint so an org can
 * search traces across every hosted-mock deployment in one shot
 * instead of fanning out per-deployment.
 */
import React, { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { Search, RefreshCw, Network } from 'lucide-react';
import { isCloudMode } from '../utils/cloudMode';
import { useCloudOrgId } from '../hooks/useCloudOrgId';
import {
    cloudObservabilityApi,
    type TraceSpanRow,
    type TraceQueryRequest,
} from '../services/api/cloudObservability';

export const CloudTracesPage: React.FC = () => {
    if (!isCloudMode()) {
        return (
            <div className="p-6 max-w-7xl mx-auto">
                <div className="bg-blue-50 dark:bg-blue-900/20 text-blue-800 dark:text-blue-300 p-4 rounded-lg">
                    Cross-deployment trace search is a cloud-only feature. The local{' '}
                    <code className="font-mono text-xs">/traces</code> page covers per-server traces.
                </div>
            </div>
        );
    }
    return <CloudView />;
};

const CloudView: React.FC = () => {
    const orgId = useCloudOrgId();
    const [filters, setFilters] = useState<TraceQueryRequest>({ status: 'any', limit: 200 });
    const [activeFilters, setActiveFilters] = useState<TraceQueryRequest>(filters);

    const tracesQuery = useQuery({
        queryKey: ['cloud', 'traces', orgId, activeFilters],
        queryFn: () => cloudObservabilityApi.queryTraces(orgId!, activeFilters),
        enabled: !!orgId,
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

    const onSearch = (e: React.FormEvent) => {
        e.preventDefault();
        setActiveFilters({ ...filters });
    };

    const traces = tracesQuery.data ?? [];

    return (
        <div className="p-6 max-w-7xl mx-auto">
            <div className="flex justify-between items-start mb-6">
                <div>
                    <h1 className="text-2xl font-bold text-gray-900 dark:text-gray-100 mb-2">Cloud Traces</h1>
                    <p className="text-gray-600 dark:text-gray-400">
                        Cross-deployment OTLP trace search scoped to your org.
                    </p>
                </div>
                <button
                    onClick={() => tracesQuery.refetch()}
                    className="flex items-center px-3 py-2 border border-gray-300 dark:border-gray-600 hover:bg-gray-50 dark:hover:bg-gray-700 rounded-lg text-sm"
                    disabled={tracesQuery.isFetching}
                >
                    <RefreshCw className={`w-4 h-4 mr-2 ${tracesQuery.isFetching ? 'animate-spin' : ''}`} />
                    Refresh
                </button>
            </div>

            <form
                onSubmit={onSearch}
                className="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-4 mb-6"
            >
                <div className="grid grid-cols-1 md:grid-cols-4 gap-3">
                    <input
                        type="text"
                        value={filters.service_name ?? ''}
                        onChange={(e) =>
                            setFilters({ ...filters, service_name: e.target.value || undefined })
                        }
                        placeholder="service name (exact)"
                        className="px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-lg text-sm outline-none focus:ring-2 focus:ring-blue-500"
                    />
                    <input
                        type="text"
                        value={filters.name_contains ?? ''}
                        onChange={(e) =>
                            setFilters({ ...filters, name_contains: e.target.value || undefined })
                        }
                        placeholder="span name (substring)"
                        className="px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-lg text-sm outline-none focus:ring-2 focus:ring-blue-500"
                    />
                    <select
                        value={filters.status ?? 'any'}
                        onChange={(e) =>
                            setFilters({
                                ...filters,
                                status: e.target.value as TraceQueryRequest['status'],
                            })
                        }
                        className="px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-lg text-sm outline-none focus:ring-2 focus:ring-blue-500"
                    >
                        <option value="any">all statuses</option>
                        <option value="ok">ok</option>
                        <option value="error">error</option>
                    </select>
                    <button
                        type="submit"
                        className="flex items-center justify-center px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg font-medium"
                    >
                        <Search className="w-4 h-4 mr-2" />
                        Search
                    </button>
                </div>
                <div className="text-xs text-gray-500 mt-2">
                    Defaults to the last 1 hour, capped at 500 spans per query.
                </div>
            </form>

            {tracesQuery.isError && (
                <div className="mb-4 bg-red-50 dark:bg-red-900/20 text-red-700 dark:text-red-400 p-4 rounded-lg text-sm">
                    {(tracesQuery.error as Error).message}
                </div>
            )}

            {traces.length === 0 && !tracesQuery.isLoading ? (
                <div className="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-12 text-center">
                    <Network className="w-16 h-16 mx-auto text-gray-400 mb-4" />
                    <h3 className="text-lg font-medium text-gray-900 dark:text-gray-100 mb-2">
                        No spans match
                    </h3>
                    <p className="text-gray-500 dark:text-gray-400">
                        Adjust filters or wait for new traffic — OTLP exporters ingest into
                        runtime_traces in near-real-time.
                    </p>
                </div>
            ) : (
                <div className="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 overflow-hidden">
                    <table className="w-full text-left text-sm">
                        <thead className="bg-gray-50 dark:bg-gray-900/50 border-b border-gray-200 dark:border-gray-700">
                            <tr>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Time</th>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Service</th>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Name</th>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Status</th>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Duration</th>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Trace ID</th>
                            </tr>
                        </thead>
                        <tbody className="divide-y divide-gray-200 dark:divide-gray-700">
                            {traces.map((s) => (
                                <SpanRow key={`${s.trace_id}-${s.span_id}`} span={s} />
                            ))}
                        </tbody>
                    </table>
                </div>
            )}
        </div>
    );
};

const SpanRow: React.FC<{ span: TraceSpanRow }> = ({ span }) => {
    const durMs = (span.end_unix_nano - span.start_unix_nano) / 1_000_000;
    const isError = span.status_code === 2;
    return (
        <tr className="hover:bg-gray-50 dark:hover:bg-gray-800/50">
            <td className="px-6 py-4 text-xs text-gray-600 dark:text-gray-300">
                {new Date(span.occurred_at).toLocaleTimeString()}
            </td>
            <td className="px-6 py-4 text-gray-700 dark:text-gray-300">{span.service_name ?? '—'}</td>
            <td className="px-6 py-4 text-gray-900 dark:text-gray-100 font-mono text-xs truncate max-w-[260px]">
                {span.name}
            </td>
            <td className="px-6 py-4">
                <span
                    className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium border ${
                        isError
                            ? 'bg-red-50 text-red-700 border-red-200 dark:bg-red-900/20 dark:text-red-400 dark:border-red-900/30'
                            : span.status_code === 1
                            ? 'bg-green-50 text-green-700 border-green-200 dark:bg-green-900/20 dark:text-green-400 dark:border-green-900/30'
                            : 'bg-gray-100 text-gray-700 border-gray-200 dark:bg-gray-800 dark:text-gray-400 dark:border-gray-700'
                    }`}
                >
                    {isError ? 'error' : span.status_code === 1 ? 'ok' : 'unset'}
                </span>
            </td>
            <td className="px-6 py-4 text-gray-600 dark:text-gray-300 font-mono text-xs">
                {durMs.toFixed(1)}ms
            </td>
            <td className="px-6 py-4 text-gray-400 font-mono text-xs">{span.trace_id.slice(0, 16)}…</td>
        </tr>
    );
};
