/**
 * Cloud Recorder + Behavioral Cloning (#6).
 *
 * Workspace-scoped page wrapping cloudRecorderApi. Two halves:
 * 1. Capture sessions — group recorded exchanges; replayable + trainable.
 * 2. Clone models — trained behavioral clones derived from sessions.
 */
import React, { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
    Plus,
    RefreshCw,
    Trash2,
    Brain,
    PlayCircle,
    Tape,
    Database,
} from 'lucide-react';
import { isCloudMode } from '../utils/cloudMode';
import { useWorkspaceStore } from '../stores/useWorkspaceStore';
import {
    cloudRecorderApi,
    type CaptureSession,
    type CloneModel,
} from '../services/api/cloudRecorder';

export const CloudRecorderPage: React.FC = () => {
    if (!isCloudMode()) {
        return (
            <div className="p-6 max-w-7xl mx-auto">
                <div className="bg-blue-50 dark:bg-blue-900/20 text-blue-800 dark:text-blue-300 p-4 rounded-lg">
                    Capture sessions + behavioral clones live in cloud mode. Local recording uses the
                    embedded recorder + per-server SQLite.
                </div>
            </div>
        );
    }
    return <CloudView />;
};

const CloudView: React.FC = () => {
    const activeWorkspace = useWorkspaceStore((s) => s.activeWorkspace);
    const queryClient = useQueryClient();
    const [showCreate, setShowCreate] = useState(false);
    const [draft, setDraft] = useState({ name: '', description: '' });
    const [trainTarget, setTrainTarget] = useState<CaptureSession | null>(null);
    const [trainName, setTrainName] = useState('');
    const [replayTarget, setReplayTarget] = useState<CaptureSession | null>(null);
    const [replayUrl, setReplayUrl] = useState('');
    const [actionMessage, setActionMessage] = useState<string | null>(null);

    const workspaceId = activeWorkspace?.id;

    const sessionsQuery = useQuery({
        queryKey: ['cloud', 'recorder', 'sessions', workspaceId],
        queryFn: () => cloudRecorderApi.listSessions(workspaceId!),
        enabled: !!workspaceId,
    });

    const clonesQuery = useQuery({
        queryKey: ['cloud', 'recorder', 'clones', workspaceId],
        queryFn: () => cloudRecorderApi.listCloneModels(workspaceId!),
        enabled: !!workspaceId,
        refetchInterval: 10_000,
    });

    const createMutation = useMutation({
        mutationFn: () =>
            cloudRecorderApi.createSession(workspaceId!, {
                name: draft.name,
                description: draft.description || undefined,
            }),
        onSuccess: () => {
            setShowCreate(false);
            setDraft({ name: '', description: '' });
            queryClient.invalidateQueries({
                queryKey: ['cloud', 'recorder', 'sessions', workspaceId],
            });
        },
    });

    const deleteSessionMutation = useMutation({
        mutationFn: (id: string) => cloudRecorderApi.deleteSession(id),
        onSuccess: () =>
            queryClient.invalidateQueries({
                queryKey: ['cloud', 'recorder', 'sessions', workspaceId],
            }),
    });

    const trainMutation = useMutation({
        mutationFn: () =>
            cloudRecorderApi.trainClone(trainTarget!.id, { name: trainName }),
        onSuccess: (clone) => {
            setActionMessage(`Training queued — clone ${clone.id.slice(0, 8)} (status: ${clone.status})`);
            setTrainTarget(null);
            setTrainName('');
            queryClient.invalidateQueries({ queryKey: ['cloud', 'recorder', 'clones', workspaceId] });
        },
        onError: (err: Error) => setActionMessage(`Train failed: ${err.message}`),
    });

    const replayMutation = useMutation({
        mutationFn: () =>
            cloudRecorderApi.replaySession(replayTarget!.id, {
                target_url: replayUrl || undefined,
            }),
        onSuccess: (run) => {
            setActionMessage(
                `Replay queued — run ${run.id.slice(0, 8)}. Live events on Cloud Test Runs.`,
            );
            setReplayTarget(null);
            setReplayUrl('');
        },
        onError: (err: Error) => setActionMessage(`Replay failed: ${err.message}`),
    });

    const deleteCloneMutation = useMutation({
        mutationFn: (id: string) => cloudRecorderApi.deleteCloneModel(id),
        onSuccess: () =>
            queryClient.invalidateQueries({
                queryKey: ['cloud', 'recorder', 'clones', workspaceId],
            }),
    });

    if (!workspaceId) {
        return (
            <div className="p-6 max-w-7xl mx-auto">
                <div className="bg-yellow-50 dark:bg-yellow-900/20 text-yellow-800 dark:text-yellow-300 p-4 rounded-lg">
                    Select a workspace to manage capture sessions.
                </div>
            </div>
        );
    }

    const sessions = sessionsQuery.data ?? [];
    const clones = clonesQuery.data ?? [];

    return (
        <div className="p-6 max-w-7xl mx-auto">
            <div className="flex justify-between items-start mb-6">
                <div>
                    <h1 className="text-2xl font-bold text-gray-900 dark:text-gray-100 mb-2 flex items-center gap-2">
                        <Tape className="w-6 h-6 text-cyan-500" />
                        Recorder & Behavioral Cloning
                    </h1>
                    <p className="text-gray-600 dark:text-gray-400">
                        Group captured exchanges into sessions; replay them for verification or train a
                        behavioral clone model that mocks the upstream behavior.
                    </p>
                </div>
                <div className="flex gap-2">
                    <button
                        onClick={() => sessionsQuery.refetch()}
                        className="flex items-center px-3 py-2 border border-gray-300 dark:border-gray-600 hover:bg-gray-50 dark:hover:bg-gray-700 rounded-lg text-sm"
                    >
                        <RefreshCw
                            className={`w-4 h-4 mr-2 ${sessionsQuery.isFetching ? 'animate-spin' : ''}`}
                        />
                        Refresh
                    </button>
                    <button
                        onClick={() => setShowCreate(true)}
                        className="flex items-center px-4 py-2 bg-cyan-600 hover:bg-cyan-700 text-white rounded-lg font-medium"
                    >
                        <Plus className="w-4 h-4 mr-2" />
                        New Capture Session
                    </button>
                </div>
            </div>

            {actionMessage && (
                <div className="mb-4 bg-green-50 dark:bg-green-900/20 text-green-800 dark:text-green-400 p-4 rounded-lg text-sm flex items-center justify-between">
                    <span>{actionMessage}</span>
                    <button onClick={() => setActionMessage(null)} className="text-xs underline">
                        dismiss
                    </button>
                </div>
            )}

            <h2 className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-3">
                Capture Sessions
            </h2>
            {sessions.length === 0 && !sessionsQuery.isLoading ? (
                <div className="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-8 text-center mb-8">
                    <Tape className="w-12 h-12 mx-auto text-gray-400 mb-3" />
                    <h3 className="text-base font-medium text-gray-900 dark:text-gray-100 mb-2">
                        No sessions yet
                    </h3>
                    <p className="text-gray-500 dark:text-gray-400 mb-4 text-sm">
                        Create a session and assign captures to it from the recorder UI.
                    </p>
                    <button
                        onClick={() => setShowCreate(true)}
                        className="px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700"
                    >
                        Create First Session
                    </button>
                </div>
            ) : (
                <div className="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 overflow-hidden mb-8">
                    <table className="w-full text-left text-sm">
                        <thead className="bg-gray-50 dark:bg-gray-900/50 border-b border-gray-200 dark:border-gray-700">
                            <tr>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Name</th>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Created</th>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400 text-right">Actions</th>
                            </tr>
                        </thead>
                        <tbody className="divide-y divide-gray-200 dark:divide-gray-700">
                            {sessions.map((s) => (
                                <SessionRow
                                    key={s.id}
                                    session={s}
                                    onTrain={() => {
                                        setTrainTarget(s);
                                        setTrainName(`${s.name}-clone`);
                                    }}
                                    onReplay={() => setReplayTarget(s)}
                                    onDelete={() => {
                                        if (confirm(`Delete session "${s.name}"?`))
                                            deleteSessionMutation.mutate(s.id);
                                    }}
                                />
                            ))}
                        </tbody>
                    </table>
                </div>
            )}

            <h2 className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-3 flex items-center gap-2">
                <Brain className="w-4 h-4" />
                Clone Models
            </h2>
            {clones.length === 0 ? (
                <div className="bg-gray-50 dark:bg-gray-900/50 rounded-lg p-6 text-sm text-gray-500 dark:text-gray-400 italic">
                    No clone models yet. Train one from a capture session above.
                </div>
            ) : (
                <div className="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 overflow-hidden">
                    <table className="w-full text-left text-sm">
                        <thead className="bg-gray-50 dark:bg-gray-900/50 border-b border-gray-200 dark:border-gray-700">
                            <tr>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Name</th>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Status</th>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Runner</th>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Artifact</th>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400 text-right">Actions</th>
                            </tr>
                        </thead>
                        <tbody className="divide-y divide-gray-200 dark:divide-gray-700">
                            {clones.map((c) => (
                                <CloneRow
                                    key={c.id}
                                    clone={c}
                                    onDelete={() => {
                                        if (confirm(`Delete clone "${c.name}"?`))
                                            deleteCloneMutation.mutate(c.id);
                                    }}
                                />
                            ))}
                        </tbody>
                    </table>
                </div>
            )}

            {showCreate && (
                <CreateSessionModal
                    state={draft}
                    setState={setDraft}
                    onClose={() => setShowCreate(false)}
                    onSubmit={() => createMutation.mutate()}
                    submitting={createMutation.isPending}
                    error={createMutation.error ? (createMutation.error as Error).message : null}
                />
            )}

            {trainTarget && (
                <TrainModal
                    session={trainTarget}
                    name={trainName}
                    setName={setTrainName}
                    onClose={() => setTrainTarget(null)}
                    onSubmit={() => trainMutation.mutate()}
                    submitting={trainMutation.isPending}
                    error={trainMutation.error ? (trainMutation.error as Error).message : null}
                />
            )}

            {replayTarget && (
                <ReplayModal
                    session={replayTarget}
                    url={replayUrl}
                    setUrl={setReplayUrl}
                    onClose={() => setReplayTarget(null)}
                    onSubmit={() => replayMutation.mutate()}
                    submitting={replayMutation.isPending}
                    error={replayMutation.error ? (replayMutation.error as Error).message : null}
                />
            )}
        </div>
    );
};

const SessionRow: React.FC<{
    session: CaptureSession;
    onTrain: () => void;
    onReplay: () => void;
    onDelete: () => void;
}> = ({ session, onTrain, onReplay, onDelete }) => (
    <tr className="hover:bg-gray-50 dark:hover:bg-gray-800/50">
        <td className="px-6 py-4">
            <div className="font-medium text-gray-900 dark:text-gray-100">{session.name}</div>
            {session.description && (
                <div className="text-xs text-gray-500 mt-0.5">{session.description}</div>
            )}
        </td>
        <td className="px-6 py-4 text-xs text-gray-600 dark:text-gray-300">
            {new Date(session.created_at).toLocaleString()}
        </td>
        <td className="px-6 py-4 text-right space-x-1">
            <button
                onClick={onReplay}
                className="p-2 text-blue-600 hover:bg-blue-50 dark:hover:bg-blue-900/20 rounded-lg"
                title="Replay against a target URL"
            >
                <PlayCircle className="w-4 h-4" />
            </button>
            <button
                onClick={onTrain}
                className="p-2 text-purple-600 hover:bg-purple-50 dark:hover:bg-purple-900/20 rounded-lg"
                title="Train behavioral clone"
            >
                <Brain className="w-4 h-4" />
            </button>
            <button
                onClick={onDelete}
                className="p-2 text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20 rounded-lg"
                title="Delete session"
            >
                <Trash2 className="w-4 h-4" />
            </button>
        </td>
    </tr>
);

const CloneRow: React.FC<{ clone: CloneModel; onDelete: () => void }> = ({ clone, onDelete }) => {
    const styles: Record<string, string> = {
        ready:
            'bg-green-50 text-green-700 border-green-200 dark:bg-green-900/20 dark:text-green-400 dark:border-green-900/30',
        training:
            'bg-blue-50 text-blue-700 border-blue-200 dark:bg-blue-900/20 dark:text-blue-400 dark:border-blue-900/30',
        failed:
            'bg-red-50 text-red-700 border-red-200 dark:bg-red-900/20 dark:text-red-400 dark:border-red-900/30',
    };
    return (
        <tr className="hover:bg-gray-50 dark:hover:bg-gray-800/50">
            <td className="px-6 py-4 font-medium">{clone.name}</td>
            <td className="px-6 py-4">
                <span
                    className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium border ${
                        styles[clone.status] ??
                        'bg-gray-100 text-gray-700 border-gray-200 dark:bg-gray-800 dark:text-gray-400 dark:border-gray-700'
                    }`}
                >
                    {clone.status}
                </span>
            </td>
            <td className="px-6 py-4 text-xs text-gray-600 dark:text-gray-300">
                {clone.runner_seconds != null ? `${clone.runner_seconds}s` : '—'}
            </td>
            <td className="px-6 py-4 font-mono text-xs text-gray-500 truncate max-w-[200px]">
                {clone.artifact_url ?? '—'}
            </td>
            <td className="px-6 py-4 text-right">
                <button
                    onClick={onDelete}
                    className="p-2 text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20 rounded-lg"
                    title="Delete clone"
                >
                    <Trash2 className="w-4 h-4" />
                </button>
            </td>
        </tr>
    );
};

const CreateSessionModal: React.FC<{
    state: { name: string; description: string };
    setState: React.Dispatch<React.SetStateAction<{ name: string; description: string }>>;
    onClose: () => void;
    onSubmit: () => void;
    submitting: boolean;
    error: string | null;
}> = ({ state, setState, onClose, onSubmit, submitting, error }) => (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm">
        <div className="bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-md w-full border border-gray-200 dark:border-gray-700">
            <div className="p-6 border-b border-gray-200 dark:border-gray-700">
                <h2 className="text-xl font-semibold">New Capture Session</h2>
            </div>
            <div className="p-6 space-y-4">
                {error && (
                    <div className="bg-red-50 dark:bg-red-900/20 text-red-700 dark:text-red-400 p-3 rounded text-sm">
                        {error}
                    </div>
                )}
                <input
                    type="text"
                    value={state.name}
                    onChange={(e) => setState({ ...state, name: e.target.value })}
                    placeholder="Session name"
                    className="w-full px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-lg outline-none focus:ring-2 focus:ring-cyan-500"
                />
                <textarea
                    value={state.description}
                    onChange={(e) => setState({ ...state, description: e.target.value })}
                    placeholder="Description (optional)"
                    rows={3}
                    className="w-full px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-lg outline-none focus:ring-2 focus:ring-cyan-500"
                />
            </div>
            <div className="p-6 border-t border-gray-200 dark:border-gray-700 flex justify-end gap-3">
                <button onClick={onClose} className="px-4 py-2">
                    Cancel
                </button>
                <button
                    onClick={onSubmit}
                    disabled={!state.name || submitting}
                    className="px-4 py-2 bg-cyan-600 hover:bg-cyan-700 text-white rounded-lg disabled:opacity-50"
                >
                    {submitting ? 'Creating…' : 'Create'}
                </button>
            </div>
        </div>
    </div>
);

const TrainModal: React.FC<{
    session: CaptureSession;
    name: string;
    setName: (s: string) => void;
    onClose: () => void;
    onSubmit: () => void;
    submitting: boolean;
    error: string | null;
}> = ({ session, name, setName, onClose, onSubmit, submitting, error }) => (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm">
        <div className="bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-md w-full border border-gray-200 dark:border-gray-700">
            <div className="p-6 border-b border-gray-200 dark:border-gray-700">
                <h2 className="text-xl font-semibold flex items-center gap-2">
                    <Brain className="w-5 h-5 text-purple-500" />
                    Train Clone from "{session.name}"
                </h2>
                <p className="text-xs text-gray-500 mt-1">
                    Trains a behavioral clone model from the captures in this session. Plan limit:{' '}
                    <code className="font-mono">max_clone_models</code> applies.
                </p>
            </div>
            <div className="p-6 space-y-4">
                {error && (
                    <div className="bg-red-50 dark:bg-red-900/20 text-red-700 dark:text-red-400 p-3 rounded text-sm">
                        {error}
                    </div>
                )}
                <input
                    type="text"
                    value={name}
                    onChange={(e) => setName(e.target.value)}
                    placeholder="Clone model name"
                    className="w-full px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-lg outline-none focus:ring-2 focus:ring-purple-500"
                />
            </div>
            <div className="p-6 border-t border-gray-200 dark:border-gray-700 flex justify-end gap-3">
                <button onClick={onClose} className="px-4 py-2">
                    Cancel
                </button>
                <button
                    onClick={onSubmit}
                    disabled={!name || submitting}
                    className="px-4 py-2 bg-purple-600 hover:bg-purple-700 text-white rounded-lg disabled:opacity-50"
                >
                    {submitting ? 'Queueing…' : 'Train Clone'}
                </button>
            </div>
        </div>
    </div>
);

const ReplayModal: React.FC<{
    session: CaptureSession;
    url: string;
    setUrl: (s: string) => void;
    onClose: () => void;
    onSubmit: () => void;
    submitting: boolean;
    error: string | null;
}> = ({ session, url, setUrl, onClose, onSubmit, submitting, error }) => (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm">
        <div className="bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-md w-full border border-gray-200 dark:border-gray-700">
            <div className="p-6 border-b border-gray-200 dark:border-gray-700">
                <h2 className="text-xl font-semibold flex items-center gap-2">
                    <PlayCircle className="w-5 h-5 text-blue-500" />
                    Replay "{session.name}"
                </h2>
                <p className="text-xs text-gray-500 mt-1">
                    Triggers a replay test_run. Live progress streams on the Cloud Test Runs page.
                </p>
            </div>
            <div className="p-6 space-y-4">
                {error && (
                    <div className="bg-red-50 dark:bg-red-900/20 text-red-700 dark:text-red-400 p-3 rounded text-sm">
                        {error}
                    </div>
                )}
                <div className="space-y-2">
                    <label className="block text-sm font-medium">Target URL (optional)</label>
                    <input
                        type="url"
                        value={url}
                        onChange={(e) => setUrl(e.target.value)}
                        placeholder="https://api.example.com (defaults to synthetic mode)"
                        className="w-full px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-lg outline-none focus:ring-2 focus:ring-blue-500 font-mono text-xs"
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
                    {submitting ? 'Queueing…' : 'Trigger Replay'}
                </button>
            </div>
        </div>
    </div>
);
