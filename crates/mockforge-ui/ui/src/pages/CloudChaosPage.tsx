/**
 * Cloud Chaos — campaign authoring + run history (#7).
 *
 * Workspace-scoped: read activeWorkspace from useWorkspaceStore. Each
 * campaign has a target (hosted_mock or external URL), a faults array
 * in `config`, and a safety_config (max_duration_ms, kill switches).
 * Reports lists per-run summaries written by the cross-table mirror
 * when the runner finishes a chaos_campaign test_run.
 */
import React, { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
    Plus,
    RefreshCw,
    Trash2,
    Zap,
    AlertTriangle,
    Play,
    History,
    ChevronRight,
} from 'lucide-react';
import { isCloudMode } from '../utils/cloudMode';
import { useWorkspaceStore } from '../stores/useWorkspaceStore';
import {
    cloudChaosApi,
    type ChaosCampaign,
    type ChaosCampaignReport,
    type ChaosTargetKind,
} from '../services/api/cloudChaos';

export const CloudChaosPage: React.FC = () => {
    if (!isCloudMode()) {
        return (
            <div className="p-6 max-w-7xl mx-auto">
                <div className="bg-blue-50 dark:bg-blue-900/20 text-blue-800 dark:text-blue-300 p-4 rounded-lg">
                    Cloud chaos campaigns only fire in cloud mode (the runner pool is part of the cloud
                    infra).
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
    const [draft, setDraft] = useState<{
        name: string;
        description: string;
        target_kind: ChaosTargetKind;
        target_ref: string;
        config: string;
        safety_config: string;
    }>({
        name: '',
        description: '',
        target_kind: 'hosted_mock',
        target_ref: '',
        config: JSON.stringify(
            {
                faults: [
                    { kind: 'latency', duration_ms: 500 },
                    { kind: 'error_rate', duration_ms: 1000 },
                ],
            },
            null,
            2,
        ),
        safety_config: JSON.stringify({ max_duration_ms: 30000 }, null, 2),
    });
    const [draftError, setDraftError] = useState<string | null>(null);
    const [reportsForCampaign, setReportsForCampaign] = useState<ChaosCampaign | null>(null);

    const workspaceId = activeWorkspace?.id;

    const campaignsQuery = useQuery({
        queryKey: ['cloud', 'chaos', 'campaigns', workspaceId],
        queryFn: () => cloudChaosApi.listCampaigns(workspaceId!),
        enabled: !!workspaceId,
    });

    const createMutation = useMutation({
        mutationFn: () => {
            let config: Record<string, unknown>;
            let safety: Record<string, unknown>;
            try {
                config = JSON.parse(draft.config);
                safety = JSON.parse(draft.safety_config);
            } catch (e) {
                throw new Error(`Invalid JSON: ${(e as Error).message}`);
            }
            return cloudChaosApi.createCampaign(workspaceId!, {
                name: draft.name,
                description: draft.description || undefined,
                target_kind: draft.target_kind,
                target_ref: draft.target_ref,
                config,
                safety_config: safety,
            });
        },
        onSuccess: () => {
            setShowCreate(false);
            setDraftError(null);
            queryClient.invalidateQueries({
                queryKey: ['cloud', 'chaos', 'campaigns', workspaceId],
            });
        },
        onError: (err: Error) => setDraftError(err.message),
    });

    const deleteMutation = useMutation({
        mutationFn: (id: string) => cloudChaosApi.deleteCampaign(id),
        onSuccess: () =>
            queryClient.invalidateQueries({
                queryKey: ['cloud', 'chaos', 'campaigns', workspaceId],
            }),
    });

    const runMutation = useMutation({
        mutationFn: (id: string) => cloudChaosApi.triggerRun(id),
    });

    if (!workspaceId) {
        return (
            <div className="p-6 max-w-7xl mx-auto">
                <div className="bg-yellow-50 dark:bg-yellow-900/20 text-yellow-800 dark:text-yellow-300 p-4 rounded-lg">
                    Select a workspace to manage chaos campaigns.
                </div>
            </div>
        );
    }

    const campaigns = campaignsQuery.data ?? [];

    return (
        <div className="p-6 max-w-7xl mx-auto">
            <div className="flex justify-between items-start mb-6">
                <div>
                    <h1 className="text-2xl font-bold text-gray-900 dark:text-gray-100 mb-2 flex items-center gap-2">
                        <Zap className="w-6 h-6 text-orange-500" />
                        Cloud Chaos Campaigns
                    </h1>
                    <p className="text-gray-600 dark:text-gray-400">
                        Author resilience experiments. Each run becomes a test_run with{' '}
                        <code className="font-mono text-xs">kind=chaos_campaign</code>; safety caps abort
                        runaway faults inline.
                    </p>
                </div>
                <div className="flex gap-2">
                    <button
                        onClick={() => campaignsQuery.refetch()}
                        className="flex items-center px-3 py-2 border border-gray-300 dark:border-gray-600 hover:bg-gray-50 dark:hover:bg-gray-700 rounded-lg text-sm"
                        disabled={campaignsQuery.isFetching}
                    >
                        <RefreshCw
                            className={`w-4 h-4 mr-2 ${campaignsQuery.isFetching ? 'animate-spin' : ''}`}
                        />
                        Refresh
                    </button>
                    <button
                        onClick={() => setShowCreate(true)}
                        className="flex items-center px-4 py-2 bg-orange-600 hover:bg-orange-700 text-white rounded-lg font-medium"
                    >
                        <Plus className="w-4 h-4 mr-2" />
                        New Campaign
                    </button>
                </div>
            </div>

            {campaignsQuery.isError && (
                <div className="mb-4 bg-red-50 dark:bg-red-900/20 text-red-700 dark:text-red-400 p-4 rounded-lg text-sm">
                    {(campaignsQuery.error as Error).message}
                </div>
            )}

            {runMutation.isSuccess && runMutation.data && (
                <div className="mb-4 bg-green-50 dark:bg-green-900/20 text-green-700 dark:text-green-400 p-4 rounded-lg text-sm">
                    Run queued — id {runMutation.data.id.slice(0, 8)}. Watch live events on the Cloud Test
                    Runs page.
                </div>
            )}
            {runMutation.isError && (
                <div className="mb-4 bg-red-50 dark:bg-red-900/20 text-red-700 dark:text-red-400 p-4 rounded-lg text-sm">
                    {(runMutation.error as Error).message}
                </div>
            )}

            {campaigns.length === 0 && !campaignsQuery.isLoading ? (
                <div className="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-12 text-center">
                    <Zap className="w-16 h-16 mx-auto text-gray-400 mb-4" />
                    <h3 className="text-lg font-medium text-gray-900 dark:text-gray-100 mb-2">
                        No campaigns yet
                    </h3>
                    <p className="text-gray-500 dark:text-gray-400 mb-6">
                        Author a campaign to start running chaos against your hosted mocks or external
                        services.
                    </p>
                    <button
                        onClick={() => setShowCreate(true)}
                        className="px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700"
                    >
                        Author First Campaign
                    </button>
                </div>
            ) : (
                <div className="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 overflow-hidden">
                    <table className="w-full text-left text-sm">
                        <thead className="bg-gray-50 dark:bg-gray-900/50 border-b border-gray-200 dark:border-gray-700">
                            <tr>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Name</th>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Target</th>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Faults</th>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Safety</th>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400 text-right">Actions</th>
                            </tr>
                        </thead>
                        <tbody className="divide-y divide-gray-200 dark:divide-gray-700">
                            {campaigns.map((c) => (
                                <CampaignRow
                                    key={c.id}
                                    campaign={c}
                                    onRun={() => runMutation.mutate(c.id)}
                                    onReports={() => setReportsForCampaign(c)}
                                    onDelete={() => {
                                        if (confirm(`Delete campaign "${c.name}"?`))
                                            deleteMutation.mutate(c.id);
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
                    onClose={() => {
                        setShowCreate(false);
                        setDraftError(null);
                    }}
                    onSubmit={() => createMutation.mutate()}
                    submitting={createMutation.isPending}
                    error={draftError}
                />
            )}

            {reportsForCampaign && (
                <ReportsModal
                    campaign={reportsForCampaign}
                    onClose={() => setReportsForCampaign(null)}
                />
            )}
        </div>
    );
};

const CampaignRow: React.FC<{
    campaign: ChaosCampaign;
    onRun: () => void;
    onReports: () => void;
    onDelete: () => void;
}> = ({ campaign, onRun, onReports, onDelete }) => {
    const faults = (campaign.config as { faults?: unknown[] }).faults ?? [];
    const maxMs = (campaign.safety_config as { max_duration_ms?: number }).max_duration_ms;
    return (
        <tr className="hover:bg-gray-50 dark:hover:bg-gray-800/50">
            <td className="px-6 py-4">
                <div className="font-medium text-gray-900 dark:text-gray-100">{campaign.name}</div>
                {campaign.description && (
                    <div className="text-xs text-gray-500 mt-0.5">{campaign.description}</div>
                )}
            </td>
            <td className="px-6 py-4">
                <div className="text-xs">
                    <div className="text-gray-500">{campaign.target_kind}</div>
                    <div className="font-mono text-gray-700 dark:text-gray-300 truncate max-w-[200px]">
                        {campaign.target_ref}
                    </div>
                </div>
            </td>
            <td className="px-6 py-4 text-gray-600 dark:text-gray-300">
                {Array.isArray(faults) ? faults.length : 0}
            </td>
            <td className="px-6 py-4 text-xs text-gray-500">
                {maxMs ? `max ${maxMs}ms` : <span className="italic">no cap</span>}
            </td>
            <td className="px-6 py-4 text-right space-x-1">
                <button
                    onClick={onRun}
                    className="p-2 text-orange-600 hover:bg-orange-50 dark:hover:bg-orange-900/20 rounded-lg"
                    title="Trigger run"
                >
                    <Play className="w-4 h-4" />
                </button>
                <button
                    onClick={onReports}
                    className="p-2 text-blue-600 hover:bg-blue-50 dark:hover:bg-blue-900/20 rounded-lg"
                    title="View report history"
                >
                    <History className="w-4 h-4" />
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
};

const CreateModal: React.FC<{
    state: {
        name: string;
        description: string;
        target_kind: ChaosTargetKind;
        target_ref: string;
        config: string;
        safety_config: string;
    };
    setState: React.Dispatch<
        React.SetStateAction<{
            name: string;
            description: string;
            target_kind: ChaosTargetKind;
            target_ref: string;
            config: string;
            safety_config: string;
        }>
    >;
    onClose: () => void;
    onSubmit: () => void;
    submitting: boolean;
    error: string | null;
}> = ({ state, setState, onClose, onSubmit, submitting, error }) => (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm">
        <div className="bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-2xl w-full max-h-[85vh] overflow-y-auto border border-gray-200 dark:border-gray-700">
            <div className="p-6 border-b border-gray-200 dark:border-gray-700 sticky top-0 bg-white dark:bg-gray-800">
                <h2 className="text-xl font-semibold flex items-center gap-2">
                    <Zap className="w-5 h-5 text-orange-500" />
                    New Chaos Campaign
                </h2>
            </div>
            <div className="p-6 space-y-4">
                {error && (
                    <div className="bg-red-50 dark:bg-red-900/20 text-red-700 dark:text-red-400 p-3 rounded text-sm whitespace-pre-wrap">
                        {error}
                    </div>
                )}
                <div className="space-y-2">
                    <label className="block text-sm font-medium">Name</label>
                    <input
                        type="text"
                        value={state.name}
                        onChange={(e) => setState({ ...state, name: e.target.value })}
                        placeholder="e.g., Payment p99 latency hammer"
                        className="w-full px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-lg outline-none focus:ring-2 focus:ring-orange-500"
                    />
                </div>
                <div className="space-y-2">
                    <label className="block text-sm font-medium">Description (optional)</label>
                    <input
                        type="text"
                        value={state.description}
                        onChange={(e) => setState({ ...state, description: e.target.value })}
                        className="w-full px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-lg outline-none focus:ring-2 focus:ring-orange-500"
                    />
                </div>
                <div className="grid grid-cols-2 gap-3">
                    <div className="space-y-2">
                        <label className="block text-sm font-medium">Target kind</label>
                        <select
                            value={state.target_kind}
                            onChange={(e) =>
                                setState({
                                    ...state,
                                    target_kind: e.target.value as ChaosTargetKind,
                                })
                            }
                            className="w-full px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-lg outline-none focus:ring-2 focus:ring-orange-500"
                        >
                            <option value="hosted_mock">hosted_mock</option>
                            <option value="external">external</option>
                        </select>
                    </div>
                    <div className="space-y-2">
                        <label className="block text-sm font-medium">Target ref</label>
                        <input
                            type="text"
                            value={state.target_ref}
                            onChange={(e) => setState({ ...state, target_ref: e.target.value })}
                            placeholder={
                                state.target_kind === 'hosted_mock'
                                    ? 'deployment-id or slug'
                                    : 'https://api.example.com'
                            }
                            className="w-full px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-lg outline-none focus:ring-2 focus:ring-orange-500 font-mono text-xs"
                        />
                    </div>
                </div>
                <div className="space-y-2">
                    <label className="block text-sm font-medium">
                        Config (JSON; the executor reads <code>config.faults</code>)
                    </label>
                    <textarea
                        value={state.config}
                        onChange={(e) => setState({ ...state, config: e.target.value })}
                        rows={6}
                        className="w-full px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-lg outline-none focus:ring-2 focus:ring-orange-500 font-mono text-xs"
                    />
                </div>
                <div className="space-y-2">
                    <label className="block text-sm font-medium">
                        Safety config (JSON; <code>max_duration_ms</code> aborts runs)
                    </label>
                    <textarea
                        value={state.safety_config}
                        onChange={(e) => setState({ ...state, safety_config: e.target.value })}
                        rows={3}
                        className="w-full px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-lg outline-none focus:ring-2 focus:ring-orange-500 font-mono text-xs"
                    />
                </div>
            </div>
            <div className="p-6 border-t border-gray-200 dark:border-gray-700 flex justify-end gap-3 sticky bottom-0 bg-white dark:bg-gray-800">
                <button onClick={onClose} className="px-4 py-2">
                    Cancel
                </button>
                <button
                    onClick={onSubmit}
                    disabled={!state.name || !state.target_ref || submitting}
                    className="px-4 py-2 bg-orange-600 hover:bg-orange-700 text-white rounded-lg disabled:opacity-50"
                >
                    {submitting ? 'Creating…' : 'Create'}
                </button>
            </div>
        </div>
    </div>
);

const ReportsModal: React.FC<{ campaign: ChaosCampaign; onClose: () => void }> = ({
    campaign,
    onClose,
}) => {
    const reportsQuery = useQuery({
        queryKey: ['cloud', 'chaos', 'reports', campaign.id],
        queryFn: () => cloudChaosApi.listReports(campaign.id),
    });

    return (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm">
            <div className="bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-3xl w-full max-h-[80vh] overflow-y-auto border border-gray-200 dark:border-gray-700">
                <div className="p-6 border-b border-gray-200 dark:border-gray-700 sticky top-0 bg-white dark:bg-gray-800 flex items-start justify-between">
                    <div>
                        <h2 className="text-xl font-semibold">{campaign.name} — Run History</h2>
                        <p className="text-xs text-gray-500 mt-1">
                            One report row per chaos_campaign run, written by the cross-table mirror when
                            the runner finishes.
                        </p>
                    </div>
                    <button onClick={onClose} className="text-gray-400 hover:text-gray-600">
                        ✕
                    </button>
                </div>
                <div className="p-6 space-y-3">
                    {reportsQuery.isLoading && <div className="text-sm text-gray-500">Loading…</div>}
                    {reportsQuery.data && reportsQuery.data.length === 0 && (
                        <div className="text-sm text-gray-500 italic">
                            No runs yet — trigger one from the campaigns list.
                        </div>
                    )}
                    {(reportsQuery.data ?? []).map((r) => (
                        <ReportCard key={r.id} report={r} />
                    ))}
                </div>
            </div>
        </div>
    );
};

const ReportCard: React.FC<{ report: ChaosCampaignReport }> = ({ report }) => (
    <div
        className={`border rounded-lg p-4 ${
            report.aborted
                ? 'border-red-200 dark:border-red-900/30 bg-red-50/50 dark:bg-red-900/10'
                : 'border-gray-200 dark:border-gray-700'
        }`}
    >
        <div className="flex items-start justify-between mb-2">
            <div className="text-sm font-mono">run {report.run_id.slice(0, 8)}</div>
            <div className="text-xs text-gray-500">{new Date(report.created_at).toLocaleString()}</div>
        </div>
        <div className="grid grid-cols-2 gap-3 text-xs mb-2">
            <div>
                <span className="text-gray-500">faults: </span>
                <span className="font-mono">{report.fault_count}</span>
            </div>
            <div>
                <span className="text-gray-500">aborted: </span>
                {report.aborted ? (
                    <span className="text-red-600 dark:text-red-400 inline-flex items-center gap-1">
                        <AlertTriangle className="w-3 h-3" />
                        yes
                    </span>
                ) : (
                    <span className="text-green-600 dark:text-green-400">no</span>
                )}
            </div>
        </div>
        {report.abort_reason && (
            <div className="text-xs text-red-700 dark:text-red-400 mb-2">
                <span className="font-medium">Abort reason: </span>
                {report.abort_reason}
            </div>
        )}
        {report.summary && (
            <details className="text-xs">
                <summary className="cursor-pointer text-gray-600 dark:text-gray-400 inline-flex items-center gap-1">
                    <ChevronRight className="w-3 h-3" />
                    summary
                </summary>
                <pre className="mt-2 bg-gray-100 dark:bg-gray-900 p-2 rounded overflow-x-auto">
                    {JSON.stringify(report.summary, null, 2)}
                </pre>
            </details>
        )}
    </div>
);
