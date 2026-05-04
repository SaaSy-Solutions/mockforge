/**
 * Cloud Contract — monitored services + diff runs (#8).
 *
 * Workspace-scoped page wrapping cloudContractApi. Each MonitoredService
 * captures a (base_url, openapi_spec_url, traffic_source) tuple that
 * the diff executor uses to compare live behavior against the spec.
 */
import React, { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
    Plus,
    RefreshCw,
    Trash2,
    Play,
    GitCompare,
    AlertTriangle,
    ChevronRight,
} from 'lucide-react';
import { isCloudMode } from '../utils/cloudMode';
import { useWorkspaceStore } from '../stores/useWorkspaceStore';
import {
    cloudContractApi,
    type MonitoredService,
    type ContractDiffFinding,
} from '../services/api/cloudContract';

const SEVERITY_STYLES: Record<string, string> = {
    breaking:
        'bg-red-50 text-red-700 border-red-200 dark:bg-red-900/20 dark:text-red-400 dark:border-red-900/30',
    non_breaking:
        'bg-yellow-50 text-yellow-700 border-yellow-200 dark:bg-yellow-900/20 dark:text-yellow-400 dark:border-yellow-900/30',
    cosmetic:
        'bg-blue-50 text-blue-700 border-blue-200 dark:bg-blue-900/20 dark:text-blue-400 dark:border-blue-900/30',
};

export const CloudContractPage: React.FC = () => {
    if (!isCloudMode()) {
        return (
            <div className="p-6 max-w-7xl mx-auto">
                <div className="bg-blue-50 dark:bg-blue-900/20 text-blue-800 dark:text-blue-300 p-4 rounded-lg">
                    Cloud contract diff only runs in cloud mode (the diff executor + run history live in
                    the registry).
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
    const [draft, setDraft] = useState({
        name: '',
        base_url: '',
        openapi_spec_url: '',
        traffic_source: 'recorder',
    });
    const [historyFor, setHistoryFor] = useState<MonitoredService | null>(null);
    const [runMessage, setRunMessage] = useState<string | null>(null);

    const workspaceId = activeWorkspace?.id;

    const servicesQuery = useQuery({
        queryKey: ['cloud', 'contract', 'services', workspaceId],
        queryFn: () => cloudContractApi.listMonitoredServices(workspaceId!),
        enabled: !!workspaceId,
    });

    const fitnessQuery = useQuery({
        queryKey: ['cloud', 'contract', 'fitness', workspaceId],
        queryFn: () => cloudContractApi.listFitnessFunctions(workspaceId!),
        enabled: !!workspaceId,
    });

    const createMutation = useMutation({
        mutationFn: () => cloudContractApi.createMonitoredService(workspaceId!, draft),
        onSuccess: () => {
            setShowCreate(false);
            setDraft({ name: '', base_url: '', openapi_spec_url: '', traffic_source: 'recorder' });
            queryClient.invalidateQueries({ queryKey: ['cloud', 'contract', 'services', workspaceId] });
        },
    });

    const deleteMutation = useMutation({
        mutationFn: (id: string) => cloudContractApi.deleteMonitoredService(id),
        onSuccess: () =>
            queryClient.invalidateQueries({
                queryKey: ['cloud', 'contract', 'services', workspaceId],
            }),
    });

    const triggerMutation = useMutation({
        mutationFn: (id: string) => cloudContractApi.triggerDiff(id),
        onSuccess: (run) =>
            setRunMessage(`Diff run queued — id ${run.id.slice(0, 8)}. Live events on Cloud Test Runs.`),
        onError: (err: Error) => setRunMessage(`Trigger failed: ${err.message}`),
    });

    if (!workspaceId) {
        return (
            <div className="p-6 max-w-7xl mx-auto">
                <div className="bg-yellow-50 dark:bg-yellow-900/20 text-yellow-800 dark:text-yellow-300 p-4 rounded-lg">
                    Select a workspace to manage monitored services.
                </div>
            </div>
        );
    }

    const services = servicesQuery.data ?? [];
    const fitness = fitnessQuery.data ?? [];

    return (
        <div className="p-6 max-w-7xl mx-auto">
            <div className="flex justify-between items-start mb-6">
                <div>
                    <h1 className="text-2xl font-bold text-gray-900 dark:text-gray-100 mb-2 flex items-center gap-2">
                        <GitCompare className="w-6 h-6 text-pink-500" />
                        Cloud Contract & Verification
                    </h1>
                    <p className="text-gray-600 dark:text-gray-400">
                        Track upstream services and run drift diffs against their OpenAPI spec. Live
                        traffic source determines what the diff is measured against.
                    </p>
                </div>
                <div className="flex gap-2">
                    <button
                        onClick={() => servicesQuery.refetch()}
                        className="flex items-center px-3 py-2 border border-gray-300 dark:border-gray-600 hover:bg-gray-50 dark:hover:bg-gray-700 rounded-lg text-sm"
                    >
                        <RefreshCw
                            className={`w-4 h-4 mr-2 ${servicesQuery.isFetching ? 'animate-spin' : ''}`}
                        />
                        Refresh
                    </button>
                    <button
                        onClick={() => setShowCreate(true)}
                        className="flex items-center px-4 py-2 bg-pink-600 hover:bg-pink-700 text-white rounded-lg font-medium"
                    >
                        <Plus className="w-4 h-4 mr-2" />
                        New Monitored Service
                    </button>
                </div>
            </div>

            {runMessage && (
                <div className="mb-4 bg-green-50 dark:bg-green-900/20 text-green-800 dark:text-green-400 p-4 rounded-lg text-sm flex items-center justify-between">
                    <span>{runMessage}</span>
                    <button onClick={() => setRunMessage(null)} className="text-xs underline">
                        dismiss
                    </button>
                </div>
            )}

            {servicesQuery.isError && (
                <div className="mb-4 bg-red-50 dark:bg-red-900/20 text-red-700 dark:text-red-400 p-4 rounded-lg text-sm">
                    {(servicesQuery.error as Error).message}
                </div>
            )}

            <h2 className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-3">
                Monitored Services
            </h2>
            {services.length === 0 && !servicesQuery.isLoading ? (
                <div className="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-12 text-center mb-8">
                    <GitCompare className="w-16 h-16 mx-auto text-gray-400 mb-4" />
                    <h3 className="text-lg font-medium text-gray-900 dark:text-gray-100 mb-2">
                        No monitored services yet
                    </h3>
                    <p className="text-gray-500 dark:text-gray-400 mb-6">
                        Add a service to start tracking spec drift.
                    </p>
                    <button
                        onClick={() => setShowCreate(true)}
                        className="px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700"
                    >
                        Add First Service
                    </button>
                </div>
            ) : (
                <div className="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 overflow-hidden mb-8">
                    <table className="w-full text-left text-sm">
                        <thead className="bg-gray-50 dark:bg-gray-900/50 border-b border-gray-200 dark:border-gray-700">
                            <tr>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Name</th>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Base URL</th>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Spec URL</th>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Traffic</th>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400 text-right">Actions</th>
                            </tr>
                        </thead>
                        <tbody className="divide-y divide-gray-200 dark:divide-gray-700">
                            {services.map((s) => (
                                <ServiceRow
                                    key={s.id}
                                    svc={s}
                                    onTrigger={() => triggerMutation.mutate(s.id)}
                                    onHistory={() => setHistoryFor(s)}
                                    onDelete={() => {
                                        if (confirm(`Delete monitored service "${s.name}"?`))
                                            deleteMutation.mutate(s.id);
                                    }}
                                />
                            ))}
                        </tbody>
                    </table>
                </div>
            )}

            <h2 className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-3">
                Fitness Functions <span className="text-xs text-gray-500 ml-2">(read-only, authoring soon)</span>
            </h2>
            {fitness.length === 0 ? (
                <div className="bg-gray-50 dark:bg-gray-900/50 rounded-lg p-6 text-sm text-gray-500 dark:text-gray-400 italic">
                    No fitness functions yet — they're scoring rules the verification suite executor
                    runs against monitored services.
                </div>
            ) : (
                <div className="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 overflow-hidden">
                    <table className="w-full text-left text-sm">
                        <thead className="bg-gray-50 dark:bg-gray-900/50 border-b border-gray-200 dark:border-gray-700">
                            <tr>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Name</th>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Kind</th>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Enabled</th>
                            </tr>
                        </thead>
                        <tbody className="divide-y divide-gray-200 dark:divide-gray-700">
                            {fitness.map((f) => (
                                <tr key={f.id} className="hover:bg-gray-50 dark:hover:bg-gray-800/50">
                                    <td className="px-6 py-4 font-medium">{f.name}</td>
                                    <td className="px-6 py-4 text-xs text-gray-600 dark:text-gray-300">{f.kind}</td>
                                    <td className="px-6 py-4 text-xs">{f.enabled ? '✅' : '⏸️'}</td>
                                </tr>
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

            {historyFor && <HistoryModal service={historyFor} onClose={() => setHistoryFor(null)} />}
        </div>
    );
};

const ServiceRow: React.FC<{
    svc: MonitoredService;
    onTrigger: () => void;
    onHistory: () => void;
    onDelete: () => void;
}> = ({ svc, onTrigger, onHistory, onDelete }) => (
    <tr className="hover:bg-gray-50 dark:hover:bg-gray-800/50">
        <td className="px-6 py-4 font-medium">{svc.name}</td>
        <td className="px-6 py-4 font-mono text-xs text-gray-600 dark:text-gray-300 truncate max-w-[200px]">
            {svc.base_url}
        </td>
        <td className="px-6 py-4 font-mono text-xs text-gray-600 dark:text-gray-300 truncate max-w-[200px]">
            {svc.openapi_spec_url}
        </td>
        <td className="px-6 py-4 text-xs">{svc.traffic_source}</td>
        <td className="px-6 py-4 text-right space-x-1">
            <button
                onClick={onTrigger}
                className="p-2 text-pink-600 hover:bg-pink-50 dark:hover:bg-pink-900/20 rounded-lg"
                title="Trigger diff run"
            >
                <Play className="w-4 h-4" />
            </button>
            <button
                onClick={onHistory}
                className="p-2 text-blue-600 hover:bg-blue-50 dark:hover:bg-blue-900/20 rounded-lg"
                title="View diff history"
            >
                <GitCompare className="w-4 h-4" />
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
    state: { name: string; base_url: string; openapi_spec_url: string; traffic_source: string };
    setState: React.Dispatch<
        React.SetStateAction<{
            name: string;
            base_url: string;
            openapi_spec_url: string;
            traffic_source: string;
        }>
    >;
    onClose: () => void;
    onSubmit: () => void;
    submitting: boolean;
    error: string | null;
}> = ({ state, setState, onClose, onSubmit, submitting, error }) => (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm">
        <div className="bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-md w-full border border-gray-200 dark:border-gray-700">
            <div className="p-6 border-b border-gray-200 dark:border-gray-700">
                <h2 className="text-xl font-semibold">New Monitored Service</h2>
            </div>
            <div className="p-6 space-y-4">
                {error && (
                    <div className="bg-red-50 dark:bg-red-900/20 text-red-700 dark:text-red-400 p-3 rounded text-sm">
                        {error}
                    </div>
                )}
                <div className="space-y-2">
                    <label className="block text-sm font-medium">Name</label>
                    <input
                        type="text"
                        value={state.name}
                        onChange={(e) => setState({ ...state, name: e.target.value })}
                        className="w-full px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-lg outline-none focus:ring-2 focus:ring-pink-500"
                    />
                </div>
                <div className="space-y-2">
                    <label className="block text-sm font-medium">Base URL</label>
                    <input
                        type="url"
                        value={state.base_url}
                        onChange={(e) => setState({ ...state, base_url: e.target.value })}
                        placeholder="https://api.example.com"
                        className="w-full px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-lg outline-none focus:ring-2 focus:ring-pink-500 font-mono text-xs"
                    />
                </div>
                <div className="space-y-2">
                    <label className="block text-sm font-medium">OpenAPI spec URL</label>
                    <input
                        type="url"
                        value={state.openapi_spec_url}
                        onChange={(e) => setState({ ...state, openapi_spec_url: e.target.value })}
                        placeholder="https://api.example.com/openapi.json"
                        className="w-full px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-lg outline-none focus:ring-2 focus:ring-pink-500 font-mono text-xs"
                    />
                </div>
                <div className="space-y-2">
                    <label className="block text-sm font-medium">Traffic source</label>
                    <select
                        value={state.traffic_source}
                        onChange={(e) => setState({ ...state, traffic_source: e.target.value })}
                        className="w-full px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-lg outline-none focus:ring-2 focus:ring-pink-500"
                    >
                        <option value="recorder">recorder (replay captures)</option>
                        <option value="synthetic">synthetic (executor-generated)</option>
                        <option value="live">live (production traffic — careful)</option>
                    </select>
                </div>
            </div>
            <div className="p-6 border-t border-gray-200 dark:border-gray-700 flex justify-end gap-3">
                <button onClick={onClose} className="px-4 py-2">
                    Cancel
                </button>
                <button
                    onClick={onSubmit}
                    disabled={!state.name || !state.base_url || !state.openapi_spec_url || submitting}
                    className="px-4 py-2 bg-pink-600 hover:bg-pink-700 text-white rounded-lg disabled:opacity-50"
                >
                    {submitting ? 'Creating…' : 'Create'}
                </button>
            </div>
        </div>
    </div>
);

const HistoryModal: React.FC<{ service: MonitoredService; onClose: () => void }> = ({
    service,
    onClose,
}) => {
    const runsQuery = useQuery({
        queryKey: ['cloud', 'contract', 'diff-runs', service.id],
        queryFn: () => cloudContractApi.listDiffRuns(service.id),
    });
    const [selectedRunId, setSelectedRunId] = useState<string | null>(null);
    const findingsQuery = useQuery({
        queryKey: ['cloud', 'contract', 'findings', selectedRunId],
        queryFn: () => cloudContractApi.listFindings(selectedRunId!),
        enabled: !!selectedRunId,
    });

    return (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm">
            <div className="bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-3xl w-full max-h-[80vh] overflow-y-auto border border-gray-200 dark:border-gray-700">
                <div className="p-6 border-b border-gray-200 dark:border-gray-700 sticky top-0 bg-white dark:bg-gray-800 flex items-start justify-between">
                    <div>
                        <h2 className="text-xl font-semibold">{service.name} — Diff History</h2>
                    </div>
                    <button onClick={onClose} className="text-gray-400 hover:text-gray-600">
                        ✕
                    </button>
                </div>
                <div className="p-6 space-y-3">
                    {(runsQuery.data ?? []).map((r) => (
                        <div
                            key={r.id}
                            className="border border-gray-200 dark:border-gray-700 rounded-lg p-3 cursor-pointer hover:bg-gray-50 dark:hover:bg-gray-800/50"
                            onClick={() => setSelectedRunId(r.id === selectedRunId ? null : r.id)}
                        >
                            <div className="flex items-center justify-between">
                                <div className="font-mono text-xs">{r.id.slice(0, 8)}</div>
                                <div className="flex items-center gap-3">
                                    <span className="text-xs text-gray-500">{r.status}</span>
                                    <span className="text-xs text-gray-500">
                                        {new Date(r.triggered_at).toLocaleString()}
                                    </span>
                                    <ChevronRight
                                        className={`w-4 h-4 text-gray-400 transition-transform ${
                                            r.id === selectedRunId ? 'rotate-90' : ''
                                        }`}
                                    />
                                </div>
                            </div>
                            {r.id === selectedRunId && (
                                <div className="mt-3 pt-3 border-t border-gray-200 dark:border-gray-700 space-y-2">
                                    {findingsQuery.isLoading && (
                                        <div className="text-xs text-gray-500">Loading findings…</div>
                                    )}
                                    {(findingsQuery.data ?? []).map((f: ContractDiffFinding) => (
                                        <FindingCard key={f.id} finding={f} />
                                    ))}
                                    {findingsQuery.data?.length === 0 && (
                                        <div className="text-xs text-green-600 dark:text-green-400 italic">
                                            No findings — spec matches behavior.
                                        </div>
                                    )}
                                </div>
                            )}
                        </div>
                    ))}
                    {runsQuery.data?.length === 0 && (
                        <div className="text-sm text-gray-500 italic">
                            No diff runs yet. Trigger one from the services list.
                        </div>
                    )}
                </div>
            </div>
        </div>
    );
};

const FindingCard: React.FC<{ finding: ContractDiffFinding }> = ({ finding }) => (
    <div
        className={`p-2 rounded text-xs border ${
            SEVERITY_STYLES[finding.severity] ??
            'bg-gray-100 text-gray-700 border-gray-200 dark:bg-gray-800 dark:text-gray-400 dark:border-gray-700'
        }`}
    >
        <div className="flex items-center gap-2 mb-1">
            <AlertTriangle className="w-3 h-3" />
            <span className="font-medium">{finding.severity}</span>
            {finding.endpoint && <span className="font-mono text-xs">{finding.endpoint}</span>}
        </div>
        <div>{finding.description}</div>
    </div>
);
