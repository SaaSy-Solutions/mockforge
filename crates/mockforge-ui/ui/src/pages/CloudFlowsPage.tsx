/**
 * Cloud Flows — unified scenario / orchestration / state-machine /
 * chain editor backed by the flows table (#9 + #14).
 *
 * Workspace-scoped. Each flow has a kind discriminator + versioned
 * config (FlowVersion). Changes save as new versions; current_version_id
 * points at the version triggers + UI reads. Run trigger reuses the
 * test_runs lifecycle.
 */
import React, { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
    Plus,
    RefreshCw,
    Trash2,
    Play,
    GitBranch,
    History as HistoryIcon,
    Save,
} from 'lucide-react';
import { isCloudMode } from '../utils/cloudMode';
import { useWorkspaceStore } from '../stores/useWorkspaceStore';
import {
    cloudFlowsApi,
    type Flow,
    type FlowKind,
    type FlowVersion,
} from '../services/api/cloudFlows';

const KIND_OPTIONS: FlowKind[] = ['scenario', 'orchestration', 'state_machine', 'chain'];

const STARTER_CONFIG: Record<FlowKind, Record<string, unknown>> = {
    scenario: { nodes: [{ id: 'start', name: 'start' }, { id: 'end', name: 'end' }] },
    orchestration: { nodes: [{ id: 'fetch', name: 'fetch' }, { id: 'process', name: 'process' }] },
    state_machine: { states: [{ id: 'idle', name: 'idle' }, { id: 'running', name: 'running' }] },
    chain: { steps: [{ id: 'request', name: 'request' }, { id: 'assert', name: 'assert' }] },
};

export const CloudFlowsPage: React.FC = () => {
    if (!isCloudMode()) {
        return (
            <div className="p-6 max-w-7xl mx-auto">
                <div className="bg-blue-50 dark:bg-blue-900/20 text-blue-800 dark:text-blue-300 p-4 rounded-lg">
                    Cloud flows store collaborative state-machine / scenario / orchestration /
                    chain definitions in the registry. Local equivalents continue to live in the
                    existing per-kind pages.
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
    const [draft, setDraft] = useState<{ kind: FlowKind; name: string; description: string }>({
        kind: 'scenario',
        name: '',
        description: '',
    });
    const [editing, setEditing] = useState<Flow | null>(null);
    const [kindFilter, setKindFilter] = useState<FlowKind | 'all'>('all');
    const [runMessage, setRunMessage] = useState<string | null>(null);

    const workspaceId = activeWorkspace?.id;

    const flowsQuery = useQuery({
        queryKey: ['cloud', 'flows', workspaceId, kindFilter],
        queryFn: () =>
            cloudFlowsApi.listForWorkspace(workspaceId!, kindFilter === 'all' ? undefined : kindFilter),
        enabled: !!workspaceId,
    });

    const createMutation = useMutation({
        mutationFn: () =>
            cloudFlowsApi.create(workspaceId!, {
                kind: draft.kind,
                name: draft.name,
                description: draft.description || undefined,
                initial_config: STARTER_CONFIG[draft.kind],
            }),
        onSuccess: () => {
            setShowCreate(false);
            setDraft({ kind: 'scenario', name: '', description: '' });
            queryClient.invalidateQueries({ queryKey: ['cloud', 'flows', workspaceId] });
        },
    });

    const deleteMutation = useMutation({
        mutationFn: (id: string) => cloudFlowsApi.delete(id),
        onSuccess: () =>
            queryClient.invalidateQueries({ queryKey: ['cloud', 'flows', workspaceId] }),
    });

    const triggerMutation = useMutation({
        mutationFn: (id: string) => cloudFlowsApi.triggerRun(id),
        onSuccess: (run) =>
            setRunMessage(`Run queued — id ${run.id.slice(0, 8)}. Open Cloud Test Runs to tail events.`),
        onError: (err: Error) => setRunMessage(`Trigger failed: ${err.message}`),
    });

    if (!workspaceId) {
        return (
            <div className="p-6 max-w-7xl mx-auto">
                <div className="bg-yellow-50 dark:bg-yellow-900/20 text-yellow-800 dark:text-yellow-300 p-4 rounded-lg">
                    Select a workspace to manage flows.
                </div>
            </div>
        );
    }

    const flows = flowsQuery.data ?? [];

    return (
        <div className="p-6 max-w-7xl mx-auto">
            <div className="flex justify-between items-start mb-6">
                <div>
                    <h1 className="text-2xl font-bold text-gray-900 dark:text-gray-100 mb-2 flex items-center gap-2">
                        <GitBranch className="w-6 h-6 text-purple-500" />
                        Cloud Flows
                    </h1>
                    <p className="text-gray-600 dark:text-gray-400">
                        Versioned scenario / orchestration / state-machine / chain definitions.
                        Triggers reuse the test_runs lifecycle so live events stream on the Cloud
                        Test Runs page.
                    </p>
                </div>
                <div className="flex gap-2">
                    <button
                        onClick={() => flowsQuery.refetch()}
                        className="flex items-center px-3 py-2 border border-gray-300 dark:border-gray-600 hover:bg-gray-50 dark:hover:bg-gray-700 rounded-lg text-sm"
                    >
                        <RefreshCw className={`w-4 h-4 mr-2 ${flowsQuery.isFetching ? 'animate-spin' : ''}`} />
                        Refresh
                    </button>
                    <button
                        onClick={() => setShowCreate(true)}
                        className="flex items-center px-4 py-2 bg-purple-600 hover:bg-purple-700 text-white rounded-lg font-medium"
                    >
                        <Plus className="w-4 h-4 mr-2" />
                        New Flow
                    </button>
                </div>
            </div>

            <div className="mb-4 flex gap-2">
                {(['all', ...KIND_OPTIONS] as const).map((k) => (
                    <button
                        key={k}
                        onClick={() => setKindFilter(k)}
                        className={`px-3 py-1.5 text-sm rounded-lg border ${
                            kindFilter === k
                                ? 'bg-purple-600 text-white border-purple-600'
                                : 'bg-white dark:bg-gray-800 border-gray-300 dark:border-gray-600 hover:bg-gray-50 dark:hover:bg-gray-700'
                        }`}
                    >
                        {k.replace('_', ' ')}
                    </button>
                ))}
            </div>

            {runMessage && (
                <div className="mb-4 bg-green-50 dark:bg-green-900/20 text-green-800 dark:text-green-400 p-4 rounded-lg text-sm flex items-center justify-between">
                    <span>{runMessage}</span>
                    <button onClick={() => setRunMessage(null)} className="text-xs underline">
                        dismiss
                    </button>
                </div>
            )}

            {flowsQuery.isError && (
                <div className="mb-4 bg-red-50 dark:bg-red-900/20 text-red-700 dark:text-red-400 p-4 rounded-lg text-sm">
                    {(flowsQuery.error as Error).message}
                </div>
            )}

            {flows.length === 0 && !flowsQuery.isLoading ? (
                <div className="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-12 text-center">
                    <GitBranch className="w-16 h-16 mx-auto text-gray-400 mb-4" />
                    <h3 className="text-lg font-medium text-gray-900 dark:text-gray-100 mb-2">No flows yet</h3>
                    <p className="text-gray-500 dark:text-gray-400 mb-6">
                        Author your first flow — versioned configs survive across sessions and replay
                        deterministically when triggered.
                    </p>
                    <button
                        onClick={() => setShowCreate(true)}
                        className="px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700"
                    >
                        Create First Flow
                    </button>
                </div>
            ) : (
                <div className="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 overflow-hidden">
                    <table className="w-full text-left text-sm">
                        <thead className="bg-gray-50 dark:bg-gray-900/50 border-b border-gray-200 dark:border-gray-700">
                            <tr>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Name</th>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Kind</th>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Updated</th>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400 text-right">Actions</th>
                            </tr>
                        </thead>
                        <tbody className="divide-y divide-gray-200 dark:divide-gray-700">
                            {flows.map((f) => (
                                <FlowRow
                                    key={f.id}
                                    flow={f}
                                    onEdit={() => setEditing(f)}
                                    onTrigger={() => triggerMutation.mutate(f.id)}
                                    onDelete={() => {
                                        if (confirm(`Delete flow "${f.name}"?`)) deleteMutation.mutate(f.id);
                                    }}
                                />
                            ))}
                        </tbody>
                    </table>
                </div>
            )}

            {showCreate && (
                <CreateModal
                    state={draft}
                    setState={setDraft}
                    onClose={() => setShowCreate(false)}
                    onSubmit={() => createMutation.mutate()}
                    submitting={createMutation.isPending}
                    error={createMutation.error ? (createMutation.error as Error).message : null}
                />
            )}

            {editing && (
                <FlowEditorModal
                    flow={editing}
                    onClose={() => {
                        setEditing(null);
                        queryClient.invalidateQueries({ queryKey: ['cloud', 'flows', workspaceId] });
                    }}
                />
            )}
        </div>
    );
};

const FlowRow: React.FC<{
    flow: Flow;
    onEdit: () => void;
    onTrigger: () => void;
    onDelete: () => void;
}> = ({ flow, onEdit, onTrigger, onDelete }) => (
    <tr
        className="hover:bg-gray-50 dark:hover:bg-gray-800/50 cursor-pointer"
        onClick={onEdit}
    >
        <td className="px-6 py-4">
            <div className="font-medium text-gray-900 dark:text-gray-100">{flow.name}</div>
            {flow.description && (
                <div className="text-xs text-gray-500 mt-0.5">{flow.description}</div>
            )}
        </td>
        <td className="px-6 py-4">
            <span className="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium border bg-purple-50 text-purple-700 border-purple-200 dark:bg-purple-900/20 dark:text-purple-400 dark:border-purple-900/30">
                {flow.kind.replace('_', ' ')}
            </span>
        </td>
        <td className="px-6 py-4 text-xs text-gray-600 dark:text-gray-300">
            {new Date(flow.updated_at).toLocaleString()}
        </td>
        <td className="px-6 py-4 text-right space-x-1" onClick={(e) => e.stopPropagation()}>
            <button
                onClick={onTrigger}
                className="p-2 text-purple-600 hover:bg-purple-50 dark:hover:bg-purple-900/20 rounded-lg"
                title="Trigger run"
            >
                <Play className="w-4 h-4" />
            </button>
            <button
                onClick={onDelete}
                className="p-2 text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20 rounded-lg"
                title="Delete"
            >
                <Trash2 className="w-4 h-4" />
            </button>
        </td>
    </tr>
);

const CreateModal: React.FC<{
    state: { kind: FlowKind; name: string; description: string };
    setState: React.Dispatch<
        React.SetStateAction<{ kind: FlowKind; name: string; description: string }>
    >;
    onClose: () => void;
    onSubmit: () => void;
    submitting: boolean;
    error: string | null;
}> = ({ state, setState, onClose, onSubmit, submitting, error }) => (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm">
        <div className="bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-md w-full border border-gray-200 dark:border-gray-700">
            <div className="p-6 border-b border-gray-200 dark:border-gray-700">
                <h2 className="text-xl font-semibold">New Flow</h2>
                <p className="text-xs text-gray-500 mt-1">
                    Initial config seeded with a 2-node skeleton. Edit immediately or after creation.
                </p>
            </div>
            <div className="p-6 space-y-4">
                {error && (
                    <div className="bg-red-50 dark:bg-red-900/20 text-red-700 dark:text-red-400 p-3 rounded text-sm">
                        {error}
                    </div>
                )}
                <div className="space-y-2">
                    <label className="block text-sm font-medium">Kind</label>
                    <select
                        value={state.kind}
                        onChange={(e) => setState({ ...state, kind: e.target.value as FlowKind })}
                        className="w-full px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-lg outline-none focus:ring-2 focus:ring-purple-500"
                    >
                        {KIND_OPTIONS.map((k) => (
                            <option key={k} value={k}>
                                {k.replace('_', ' ')}
                            </option>
                        ))}
                    </select>
                </div>
                <div className="space-y-2">
                    <label className="block text-sm font-medium">Name</label>
                    <input
                        type="text"
                        value={state.name}
                        onChange={(e) => setState({ ...state, name: e.target.value })}
                        className="w-full px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-lg outline-none focus:ring-2 focus:ring-purple-500"
                    />
                </div>
                <div className="space-y-2">
                    <label className="block text-sm font-medium">Description (optional)</label>
                    <input
                        type="text"
                        value={state.description}
                        onChange={(e) => setState({ ...state, description: e.target.value })}
                        className="w-full px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-lg outline-none focus:ring-2 focus:ring-purple-500"
                    />
                </div>
            </div>
            <div className="p-6 border-t border-gray-200 dark:border-gray-700 flex justify-end gap-3">
                <button onClick={onClose} className="px-4 py-2">
                    Cancel
                </button>
                <button
                    onClick={onSubmit}
                    disabled={!state.name || submitting}
                    className="px-4 py-2 bg-purple-600 hover:bg-purple-700 text-white rounded-lg disabled:opacity-50"
                >
                    {submitting ? 'Creating…' : 'Create'}
                </button>
            </div>
        </div>
    </div>
);

const FlowEditorModal: React.FC<{ flow: Flow; onClose: () => void }> = ({ flow, onClose }) => {
    const queryClient = useQueryClient();
    const [config, setConfig] = useState('');
    const [changelog, setChangelog] = useState('');
    const [parseError, setParseError] = useState<string | null>(null);
    const [showVersions, setShowVersions] = useState(false);

    const versionQuery = useQuery({
        queryKey: ['cloud', 'flows', 'version', flow.id, flow.current_version_id],
        queryFn: () =>
            flow.current_version_id
                ? cloudFlowsApi.getVersion(flow.id, flow.current_version_id)
                : Promise.resolve(null),
    });

    const versionsQuery = useQuery({
        queryKey: ['cloud', 'flows', 'versions', flow.id],
        queryFn: () => cloudFlowsApi.listVersions(flow.id),
        enabled: showVersions,
    });

    React.useEffect(() => {
        if (versionQuery.data) {
            setConfig(JSON.stringify(versionQuery.data.config, null, 2));
        }
    }, [versionQuery.data]);

    const saveMutation = useMutation({
        mutationFn: () => {
            let parsed: Record<string, unknown>;
            try {
                parsed = JSON.parse(config);
                setParseError(null);
            } catch (e) {
                setParseError(`Invalid JSON: ${(e as Error).message}`);
                throw e;
            }
            return cloudFlowsApi.saveVersion(flow.id, {
                config: parsed,
                changelog: changelog || undefined,
                set_current: true,
            });
        },
        onSuccess: () => {
            queryClient.invalidateQueries({ queryKey: ['cloud', 'flows'] });
            setChangelog('');
        },
    });

    return (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm">
            <div className="bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-3xl w-full max-h-[85vh] overflow-y-auto border border-gray-200 dark:border-gray-700">
                <div className="p-6 border-b border-gray-200 dark:border-gray-700 sticky top-0 bg-white dark:bg-gray-800 flex items-start justify-between">
                    <div>
                        <h2 className="text-xl font-semibold">{flow.name}</h2>
                        <p className="text-xs text-gray-500 mt-1">
                            <span className="font-mono">{flow.kind}</span> · current version{' '}
                            {flow.current_version_id?.slice(0, 8) ?? '(none)'}
                        </p>
                    </div>
                    <button onClick={onClose} className="text-gray-400 hover:text-gray-600">
                        ✕
                    </button>
                </div>
                <div className="p-6 space-y-4">
                    {parseError && (
                        <div className="bg-red-50 dark:bg-red-900/20 text-red-700 dark:text-red-400 p-3 rounded text-sm">
                            {parseError}
                        </div>
                    )}
                    <div className="space-y-2">
                        <label className="block text-sm font-medium">
                            Config (JSON — saving creates a new version)
                        </label>
                        <textarea
                            value={config}
                            onChange={(e) => setConfig(e.target.value)}
                            rows={16}
                            className="w-full px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-lg outline-none focus:ring-2 focus:ring-purple-500 font-mono text-xs"
                        />
                    </div>
                    <div className="space-y-2">
                        <label className="block text-sm font-medium">Changelog (optional)</label>
                        <input
                            type="text"
                            value={changelog}
                            onChange={(e) => setChangelog(e.target.value)}
                            placeholder="What changed in this version?"
                            className="w-full px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-lg outline-none focus:ring-2 focus:ring-purple-500"
                        />
                    </div>
                    <div className="flex items-center justify-between">
                        <button
                            onClick={() => setShowVersions(!showVersions)}
                            className="text-sm text-blue-600 dark:text-blue-400 hover:underline inline-flex items-center"
                        >
                            <HistoryIcon className="w-4 h-4 mr-1" />
                            {showVersions ? 'Hide' : 'Show'} version history
                        </button>
                        <button
                            onClick={() => saveMutation.mutate()}
                            disabled={saveMutation.isPending}
                            className="flex items-center px-4 py-2 bg-purple-600 hover:bg-purple-700 text-white rounded-lg font-medium disabled:opacity-50"
                        >
                            <Save className="w-4 h-4 mr-2" />
                            Save Version
                        </button>
                    </div>
                    {showVersions && (
                        <div className="space-y-2 pt-3 border-t border-gray-200 dark:border-gray-700">
                            {(versionsQuery.data ?? []).map((v: FlowVersion) => (
                                <div
                                    key={v.id}
                                    className="bg-gray-50 dark:bg-gray-900/50 rounded p-2 text-xs flex justify-between"
                                >
                                    <span className="font-mono">v{v.version_number}</span>
                                    <span className="text-gray-600 dark:text-gray-400">
                                        {v.changelog ?? <span className="italic">no changelog</span>}
                                    </span>
                                    <span className="text-gray-500">
                                        {new Date(v.created_at).toLocaleString()}
                                    </span>
                                </div>
                            ))}
                        </div>
                    )}
                </div>
            </div>
        </div>
    );
};
