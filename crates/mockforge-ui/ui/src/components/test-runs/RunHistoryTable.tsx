import React from 'react';
import { Activity, Square, Trash2, ChevronRight } from 'lucide-react';
import type { TestRun } from '../../services/api/cloudTestRuns';
import { STATUS_STYLES } from './runStatusStyles';

interface RunHistoryTableProps {
    runs: TestRun[];
    loading: boolean;
    isPending: (id: string) => boolean;
    onView: (run: TestRun) => void;
    onDelete: (run: TestRun) => void;
    onCancel: (run: TestRun) => void;
}
export function RunHistoryTable({ runs, loading, isPending, onView, onDelete, onCancel }: RunHistoryTableProps) {
    return <>
            {runs.length === 0 && !loading ? (
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
                                    onView={() => onView(r)}
                                    pending={isPending(r.id)}
                                    onDelete={() => onDelete(r)}
                                    onCancel={() => onCancel(r)}
                                />
                            ))}
                        </tbody>
                    </table>
                </div>
            )}

    </>;
}

const RunRow: React.FC<{
    run: TestRun;
    onView: () => void;
    onCancel: () => void;
    onDelete: () => void;
    pending: boolean;
}> = ({ run, onView, onCancel, onDelete, pending }) => {
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
                        aria-label={`Cancel run ${run.id.slice(0, 8)}`}
                        disabled={pending}
                    >
                        <Square className="w-4 h-4" />
                    </button>
                )}
                {!inflight && <button onClick={onDelete} disabled={pending} aria-label={`Delete run ${run.id.slice(0, 8)}`} title="Delete" className="p-2 text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20 rounded-lg">
                    <Trash2 className="w-4 h-4" />
                </button>}
                <button onClick={onView} aria-label={`View run ${run.id.slice(0, 8)}`} className="p-2 rounded-lg"><ChevronRight className="w-4 h-4 inline text-gray-400" /></button>
            </td>
        </tr>
    );
};
