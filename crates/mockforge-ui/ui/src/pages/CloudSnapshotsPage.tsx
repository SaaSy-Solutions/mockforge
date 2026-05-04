/**
 * Cloud Snapshots — Time Travel for the active workspace (#10).
 *
 * Renders the workspace's snapshot history with capture / diff / restore
 * / delete actions. Snapshot capture is synchronous on the backend so
 * the UI just refetches after the request returns; no SSE needed.
 *
 * Local-mode TimeTravelPage covers a different feature (mock-server
 * temporal simulation) and stays separately routable.
 */
import React, { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
    Camera,
    RefreshCw,
    Trash2,
    GitCompare,
    Undo2,
    AlertTriangle,
    Clock,
} from 'lucide-react';
import { isCloudMode } from '../utils/cloudMode';
import { useWorkspaceStore } from '../stores/useWorkspaceStore';
import {
    cloudSnapshotsApi,
    type Snapshot,
    type SnapshotDiff,
    type SnapshotRestoreResult,
} from '../services/api/cloudSnapshots';

export const CloudSnapshotsPage: React.FC = () => {
    if (!isCloudMode()) {
        return (
            <div className="p-6 max-w-7xl mx-auto">
                <div className="bg-blue-50 dark:bg-blue-900/20 text-blue-800 dark:text-blue-300 p-4 rounded-lg">
                    Workspace snapshots are a cloud-only feature. The local{' '}
                    <code className="font-mono text-xs">/time-travel</code> page covers in-mock
                    temporal simulation.
                </div>
            </div>
        );
    }
    return <CloudView />;
};

const CloudView: React.FC = () => {
    const activeWorkspace = useWorkspaceStore((s) => s.activeWorkspace);
    const queryClient = useQueryClient();
    const [showCapture, setShowCapture] = useState(false);
    const [draft, setDraft] = useState({ name: '', description: '' });
    const [diffPreview, setDiffPreview] = useState<SnapshotDiff | null>(null);
    const [restoreResult, setRestoreResult] = useState<SnapshotRestoreResult | null>(null);

    const workspaceId = activeWorkspace?.id;

    const snapshotsQuery = useQuery({
        queryKey: ['cloud', 'snapshots', workspaceId],
        queryFn: () => cloudSnapshotsApi.listForWorkspace(workspaceId!),
        enabled: !!workspaceId,
    });

    const captureMutation = useMutation({
        mutationFn: () =>
            cloudSnapshotsApi.capture(workspaceId!, {
                name: draft.name || undefined,
                description: draft.description || undefined,
            }),
        onSuccess: () => {
            setShowCapture(false);
            setDraft({ name: '', description: '' });
            queryClient.invalidateQueries({ queryKey: ['cloud', 'snapshots', workspaceId] });
        },
    });

    const deleteMutation = useMutation({
        mutationFn: (id: string) => cloudSnapshotsApi.delete(id),
        onSuccess: () =>
            queryClient.invalidateQueries({ queryKey: ['cloud', 'snapshots', workspaceId] }),
    });

    const diffMutation = useMutation({
        mutationFn: (id: string) => cloudSnapshotsApi.diff(id, 'current'),
        onSuccess: (data) => setDiffPreview(data),
    });

    const restoreMutation = useMutation({
        mutationFn: (id: string) => cloudSnapshotsApi.restore(id),
        onSuccess: (data) => {
            setRestoreResult(data);
            queryClient.invalidateQueries({ queryKey: ['cloud', 'snapshots', workspaceId] });
        },
    });

    if (!workspaceId) {
        return (
            <div className="p-6 max-w-7xl mx-auto">
                <div className="bg-yellow-50 dark:bg-yellow-900/20 text-yellow-800 dark:text-yellow-300 p-4 rounded-lg">
                    Select a workspace to manage snapshots.
                </div>
            </div>
        );
    }

    const snapshots = snapshotsQuery.data ?? [];

    return (
        <div className="p-6 max-w-7xl mx-auto">
            <div className="flex justify-between items-start mb-8">
                <div>
                    <h1 className="text-2xl font-bold text-gray-900 dark:text-gray-100 mb-2">
                        Workspace Snapshots
                    </h1>
                    <p className="text-gray-600 dark:text-gray-400">
                        Capture the workspace's services, fixtures, flows, environments, and chaos
                        campaigns. Diff against current state before restoring.
                    </p>
                </div>
                <div className="flex gap-2">
                    <button
                        onClick={() => snapshotsQuery.refetch()}
                        className="flex items-center px-3 py-2 border border-gray-300 dark:border-gray-600 hover:bg-gray-50 dark:hover:bg-gray-700 rounded-lg text-sm"
                        disabled={snapshotsQuery.isFetching}
                    >
                        <RefreshCw
                            className={`w-4 h-4 mr-2 ${snapshotsQuery.isFetching ? 'animate-spin' : ''}`}
                        />
                        Refresh
                    </button>
                    <button
                        onClick={() => setShowCapture(true)}
                        className="flex items-center px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg font-medium"
                    >
                        <Camera className="w-4 h-4 mr-2" />
                        Capture Snapshot
                    </button>
                </div>
            </div>

            {snapshotsQuery.isError && (
                <div className="mb-4 bg-red-50 dark:bg-red-900/20 text-red-700 dark:text-red-400 p-4 rounded-lg text-sm">
                    {(snapshotsQuery.error as Error).message}
                </div>
            )}

            {restoreResult && <RestoreBanner result={restoreResult} onDismiss={() => setRestoreResult(null)} />}

            {snapshots.length === 0 && !snapshotsQuery.isLoading ? (
                <div className="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-12 text-center">
                    <Camera className="w-16 h-16 mx-auto text-gray-400 mb-4" />
                    <h3 className="text-lg font-medium text-gray-900 dark:text-gray-100 mb-2">
                        No snapshots yet
                    </h3>
                    <p className="text-gray-500 dark:text-gray-400 mb-6">
                        Capture before chaos campaigns or risky migrations so you can roll back if it
                        goes wrong.
                    </p>
                    <button
                        onClick={() => setShowCapture(true)}
                        className="px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700"
                    >
                        Capture First Snapshot
                    </button>
                </div>
            ) : (
                <div className="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 overflow-hidden">
                    <table className="w-full text-left text-sm">
                        <thead className="bg-gray-50 dark:bg-gray-900/50 border-b border-gray-200 dark:border-gray-700">
                            <tr>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Name</th>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Status</th>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Captured</th>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Expires</th>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Size</th>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400 text-right">Actions</th>
                            </tr>
                        </thead>
                        <tbody className="divide-y divide-gray-200 dark:divide-gray-700">
                            {snapshots.map((s) => (
                                <SnapshotRow
                                    key={s.id}
                                    snapshot={s}
                                    onDiff={() => diffMutation.mutate(s.id)}
                                    onRestore={() => {
                                        if (
                                            confirm(
                                                `Restore from "${s.name ?? s.id.slice(0, 8)}"?\n\nThis re-creates environments + chaos campaigns from the snapshot. Existing rows with the same name are skipped.`,
                                            )
                                        )
                                            restoreMutation.mutate(s.id);
                                    }}
                                    onDelete={() => {
                                        if (confirm('Delete this snapshot?')) deleteMutation.mutate(s.id);
                                    }}
                                />
                            ))}
                        </tbody>
                    </table>
                </div>
            )}

            {showCapture && (
                <CaptureModal
                    state={draft}
                    setState={setDraft}
                    onClose={() => setShowCapture(false)}
                    onSubmit={() => captureMutation.mutate()}
                    submitting={captureMutation.isPending}
                    error={captureMutation.error ? (captureMutation.error as Error).message : null}
                />
            )}

            {diffPreview && <DiffModal diff={diffPreview} onClose={() => setDiffPreview(null)} />}
        </div>
    );
};

const SnapshotRow: React.FC<{
    snapshot: Snapshot;
    onDiff: () => void;
    onRestore: () => void;
    onDelete: () => void;
}> = ({ snapshot, onDiff, onRestore, onDelete }) => {
    const ready = snapshot.status === 'ready';
    return (
        <tr className="hover:bg-gray-50 dark:hover:bg-gray-800/50">
            <td className="px-6 py-4">
                <div className="font-medium text-gray-900 dark:text-gray-100">
                    {snapshot.name ?? <span className="text-gray-400 italic">(unnamed)</span>}
                </div>
                <div className="text-xs text-gray-500 font-mono mt-0.5">{snapshot.id.slice(0, 8)}</div>
                {snapshot.description && (
                    <div className="text-xs text-gray-500 mt-1">{snapshot.description}</div>
                )}
            </td>
            <td className="px-6 py-4">
                <StatusBadge status={snapshot.status} />
            </td>
            <td className="px-6 py-4 text-gray-600 dark:text-gray-300">
                {snapshot.captured_at ? new Date(snapshot.captured_at).toLocaleString() : '—'}
            </td>
            <td className="px-6 py-4 text-gray-600 dark:text-gray-300">
                {snapshot.expires_at ? (
                    <span className="inline-flex items-center">
                        <Clock className="w-3.5 h-3.5 mr-1 text-gray-400" />
                        {new Date(snapshot.expires_at).toLocaleDateString()}
                    </span>
                ) : (
                    <span className="text-gray-400 italic">never</span>
                )}
            </td>
            <td className="px-6 py-4 text-gray-600 dark:text-gray-300">
                {snapshot.size_bytes !== null ? `${(snapshot.size_bytes / 1024).toFixed(1)} KB` : '—'}
            </td>
            <td className="px-6 py-4 text-right space-x-1">
                <button
                    onClick={onDiff}
                    disabled={!ready}
                    className="p-2 text-blue-600 hover:bg-blue-50 dark:hover:bg-blue-900/20 rounded-lg disabled:opacity-30"
                    title="Diff vs current state"
                >
                    <GitCompare className="w-4 h-4" />
                </button>
                <button
                    onClick={onRestore}
                    disabled={!ready}
                    className="p-2 text-green-600 hover:bg-green-50 dark:hover:bg-green-900/20 rounded-lg disabled:opacity-30"
                    title="Restore environments + chaos campaigns"
                >
                    <Undo2 className="w-4 h-4" />
                </button>
                <button
                    onClick={onDelete}
                    className="p-2 text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20 rounded-lg"
                    title="Delete snapshot"
                >
                    <Trash2 className="w-4 h-4" />
                </button>
            </td>
        </tr>
    );
};

const StatusBadge: React.FC<{ status: Snapshot['status'] }> = ({ status }) => {
    const styles: Record<Snapshot['status'], string> = {
        ready: 'bg-green-50 text-green-700 border-green-200 dark:bg-green-900/20 dark:text-green-400 dark:border-green-900/30',
        capturing: 'bg-blue-50 text-blue-700 border-blue-200 dark:bg-blue-900/20 dark:text-blue-400 dark:border-blue-900/30',
        failed: 'bg-red-50 text-red-700 border-red-200 dark:bg-red-900/20 dark:text-red-400 dark:border-red-900/30',
        expired: 'bg-gray-100 text-gray-700 border-gray-200 dark:bg-gray-800 dark:text-gray-400 dark:border-gray-700',
    };
    return (
        <span
            className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium border ${styles[status]}`}
        >
            {status}
        </span>
    );
};

const CaptureModal: React.FC<{
    state: { name: string; description: string };
    setState: (s: { name: string; description: string }) => void;
    onClose: () => void;
    onSubmit: () => void;
    submitting: boolean;
    error: string | null;
}> = ({ state, setState, onClose, onSubmit, submitting, error }) => (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm">
        <div className="bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-md w-full border border-gray-200 dark:border-gray-700">
            <div className="p-6 border-b border-gray-200 dark:border-gray-700">
                <h2 className="text-xl font-semibold">Capture Snapshot</h2>
                <p className="text-xs text-gray-500 mt-1">
                    Synchronous — the manifest builds inline, so the snapshot is ready as soon as
                    this request returns.
                </p>
            </div>
            <div className="p-6 space-y-4">
                {error && (
                    <div className="bg-red-50 dark:bg-red-900/20 text-red-700 dark:text-red-400 p-3 rounded text-sm">
                        {error}
                    </div>
                )}
                <div className="space-y-2">
                    <label className="block text-sm font-medium">Name (optional)</label>
                    <input
                        type="text"
                        value={state.name}
                        onChange={(e) => setState({ ...state, name: e.target.value })}
                        placeholder="e.g., Pre-chaos baseline"
                        className="w-full px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-lg outline-none focus:ring-2 focus:ring-blue-500"
                    />
                </div>
                <div className="space-y-2">
                    <label className="block text-sm font-medium">Description (optional)</label>
                    <textarea
                        value={state.description}
                        onChange={(e) => setState({ ...state, description: e.target.value })}
                        rows={3}
                        placeholder="What's the snapshot for?"
                        className="w-full px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-lg outline-none focus:ring-2 focus:ring-blue-500"
                    />
                </div>
            </div>
            <div className="p-6 border-t border-gray-200 dark:border-gray-700 flex justify-end gap-3">
                <button onClick={onClose} className="px-4 py-2">
                    Cancel
                </button>
                <button
                    onClick={onSubmit}
                    disabled={submitting}
                    className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg disabled:opacity-50"
                >
                    {submitting ? 'Capturing…' : 'Capture'}
                </button>
            </div>
        </div>
    </div>
);

const DiffModal: React.FC<{ diff: SnapshotDiff; onClose: () => void }> = ({ diff, onClose }) => {
    const sections = [
        { key: 'services', diff: diff.services },
        { key: 'fixtures', diff: diff.fixtures },
        { key: 'flows', diff: diff.flows },
        { key: 'environments', diff: diff.environments },
        { key: 'chaos_campaigns', diff: diff.chaos_campaigns },
    ];
    return (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm">
            <div className="bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-3xl w-full max-h-[80vh] overflow-y-auto border border-gray-200 dark:border-gray-700">
                <div className="p-6 border-b border-gray-200 dark:border-gray-700 sticky top-0 bg-white dark:bg-gray-800">
                    <h2 className="text-xl font-semibold">Snapshot vs Current State</h2>
                </div>
                <div className="p-6 space-y-4">
                    {sections.map(({ key, diff }) => {
                        const total = diff.added.length + diff.removed.length + diff.changed.length;
                        if (total === 0) {
                            return (
                                <div
                                    key={key}
                                    className="text-sm text-gray-500 dark:text-gray-400 border-b pb-2"
                                >
                                    {key}: no changes
                                </div>
                            );
                        }
                        return (
                            <div key={key} className="border-b pb-3">
                                <div className="font-medium mb-2">{key}</div>
                                <div className="text-xs space-y-1">
                                    {diff.added.length > 0 && (
                                        <div className="text-green-700 dark:text-green-400">
                                            + {diff.added.length} added
                                        </div>
                                    )}
                                    {diff.removed.length > 0 && (
                                        <div className="text-red-700 dark:text-red-400">
                                            − {diff.removed.length} removed
                                        </div>
                                    )}
                                    {diff.changed.length > 0 && (
                                        <div className="text-yellow-700 dark:text-yellow-400">
                                            ~ {diff.changed.length} changed
                                        </div>
                                    )}
                                </div>
                            </div>
                        );
                    })}
                </div>
                <div className="p-6 border-t border-gray-200 dark:border-gray-700 flex justify-end sticky bottom-0 bg-white dark:bg-gray-800">
                    <button
                        onClick={onClose}
                        className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg"
                    >
                        Close
                    </button>
                </div>
            </div>
        </div>
    );
};

const RestoreBanner: React.FC<{ result: SnapshotRestoreResult; onDismiss: () => void }> = ({
    result,
    onDismiss,
}) => (
    <div className="mb-4 bg-green-50 dark:bg-green-900/20 text-green-800 dark:text-green-400 p-4 rounded-lg text-sm">
        <div className="flex items-start justify-between gap-3">
            <div className="space-y-1">
                <div className="font-medium">Restore complete</div>
                <div>
                    Environments: {result.environments.created} created,{' '}
                    {result.environments.skipped_existing} skipped (already exist).
                </div>
                <div>
                    Chaos campaigns: {result.chaos_campaigns.created} created,{' '}
                    {result.chaos_campaigns.skipped_existing} skipped.
                </div>
                {result.errors.length > 0 && (
                    <div className="flex items-start gap-1 text-yellow-700 dark:text-yellow-400 mt-2">
                        <AlertTriangle className="w-4 h-4 mt-0.5 shrink-0" />
                        <div>
                            {result.errors.length} error{result.errors.length > 1 ? 's' : ''} —{' '}
                            {result.errors[0].error}
                        </div>
                    </div>
                )}
                <div className="text-xs text-gray-500 mt-2">{result.note}</div>
            </div>
            <button onClick={onDismiss} className="text-xs underline">
                dismiss
            </button>
        </div>
    </div>
);
