/**
 * Cloud Test Runs — org-wide run history with live event tailing (#4).
 *
 * Shows the test_runs table for the org plus a detail panel that opens
 * an SSE EventSource against /api/v1/test-runs/{id}/stream so an
 * operator can watch a queued run progress without polling. Once a run
 * reaches terminal status the stream's final 'done' event triggers a
 * summary refresh.
 */
import React, { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { RefreshCw, Play } from 'lucide-react';
import { RunHistoryTable } from '../components/test-runs/RunHistoryTable';
import { isRunActionPending } from '../components/test-runs/runActions';
import { STATUS_STYLES } from '../components/test-runs/runStatusStyles';
import { RunSummary } from '../components/test-runs/RunSummary';
import RunLiveTail from '../components/RunLiveTail';
import { confirmAction } from '../components/ui/ConfirmationDialog';
import { isCloudMode } from '../utils/cloudMode';
import { useCloudOrgId } from '../hooks/useCloudOrgId';
import {
    cloudTestRunsApi,
    type TestRun,
    type TestRunStatus,
} from '../services/api/cloudTestRuns';

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

    const deleteMutation = useMutation({
        mutationFn: (id: string) => cloudTestRunsApi.deleteRun(id),
        onSuccess: (_result, id) => {
            setSelected(current => current?.id === id ? null : current);
            void queryClient.invalidateQueries({ queryKey: ['cloud', 'test-runs'] });
        },
    });

    async function deleteRun(run: TestRun) {
        if (await confirmAction(`Delete run ${run.id.slice(0, 8)} and its event history? This cannot be undone.`)) {
            deleteMutation.mutate(run.id);
        }
    }

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
                    <h1 className="text-2xl font-semibold tracking-tight text-foreground">Test Runs</h1>
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

            {(deleteMutation.isError || cancelMutation.isError) && <p role="alert" className="mb-4 text-sm text-destructive">{(deleteMutation.error ?? cancelMutation.error)?.message}</p>}

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

            <RunHistoryTable runs={runs} loading={runsQuery.isLoading}
                isPending={id => isRunActionPending(id, deleteMutation, cancelMutation)}
                onView={setSelected} onDelete={run => void deleteRun(run)}
                onCancel={async run => {
                    if (await confirmAction('Stop this queued or running test?')) cancelMutation.mutate(run.id);
                }} />

            {selected && <RunDetailPanel run={runs.find(run => run.id === selected.id) ?? selected} onClose={() => setSelected(null)} />}
        </div>
    );
};

const RunDetailPanel: React.FC<{ run: TestRun; onClose: () => void }> = ({ run, onClose }) => {
    const queryClient = useQueryClient();
    const inflight = run.status === 'queued' || run.status === 'running';

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
                            </div>
                        </div>
                        <button onClick={onClose} aria-label="Close run details" className="text-gray-400 hover:text-gray-600">
                            ✕
                        </button>
                    </div>
                </div>
                <div className="p-6 space-y-4">
                    {run.summary && <RunSummary run={run} />}
                    <RunLiveTail runId={run.id} inflight={inflight} onDone={() => {
                        void queryClient.invalidateQueries({ queryKey: ['cloud', 'test-runs'] });
                    }} />
                </div>
            </div>
        </div>
    );
};
