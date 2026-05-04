import React, { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
    Plus,
    RefreshCw,
    Trash2,
    Globe,
    Copy,
    Wifi,
    WifiOff,
    ExternalLink,
    ShieldCheck,
    ShieldAlert,
} from 'lucide-react';
import { isCloudMode } from '../utils/cloudMode';
import { useCloudOrgId } from '../hooks/useCloudOrgId';
import {
    cloudTunnelsApi,
    type TunnelReservation,
} from '../services/api/cloudTunnels';

interface LocalTunnel {
    id: string;
    name: string;
    local_port: number;
    public_url: string;
    status: 'active' | 'inactive' | 'error';
    created_at: string;
    region: string;
}

export const TunnelsPage: React.FC = () => {
    return isCloudMode() ? <CloudTunnelsView /> : <LocalTunnelsView />;
};

// --- cloud view -------------------------------------------------------------

const CloudTunnelsView: React.FC = () => {
    const orgId = useCloudOrgId();
    const queryClient = useQueryClient();
    const [isCreateModalOpen, setIsCreateModalOpen] = useState(false);
    const [newTunnel, setNewTunnel] = useState({ name: '', subdomain: '', custom_domain: '' });
    const [verifyError, setVerifyError] = useState<string | null>(null);

    const tunnelsQuery = useQuery({
        queryKey: ['cloud', 'tunnels', orgId],
        queryFn: () => cloudTunnelsApi.listForOrg(orgId!),
        enabled: !!orgId,
    });

    const createMutation = useMutation({
        mutationFn: () =>
            cloudTunnelsApi.create(orgId!, {
                name: newTunnel.name,
                subdomain: newTunnel.subdomain,
                custom_domain: newTunnel.custom_domain || undefined,
            }),
        onSuccess: () => {
            setIsCreateModalOpen(false);
            setNewTunnel({ name: '', subdomain: '', custom_domain: '' });
            queryClient.invalidateQueries({ queryKey: ['cloud', 'tunnels', orgId] });
        },
    });

    const deleteMutation = useMutation({
        mutationFn: (id: string) => cloudTunnelsApi.delete(id),
        onSuccess: () =>
            queryClient.invalidateQueries({ queryKey: ['cloud', 'tunnels', orgId] }),
    });

    const verifyMutation = useMutation({
        mutationFn: (id: string) => cloudTunnelsApi.verifyCustomDomain(id),
        onSuccess: () => {
            setVerifyError(null);
            queryClient.invalidateQueries({ queryKey: ['cloud', 'tunnels', orgId] });
        },
        onError: (err: Error) => setVerifyError(err.message),
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

    const tunnels = tunnelsQuery.data ?? [];
    const copyToClipboard = (text: string) => navigator.clipboard.writeText(text);

    return (
        <div className="p-6 max-w-7xl mx-auto">
            <div className="flex justify-between items-start mb-8">
                <div>
                    <h1 className="text-2xl font-bold text-gray-900 dark:text-gray-100 mb-2">Tunnels</h1>
                    <p className="text-gray-600 dark:text-gray-400">
                        Reserve a subdomain on the MockForge relay so external services can reach your local mocks.
                    </p>
                </div>
                <div className="flex gap-2">
                    <button
                        onClick={() => tunnelsQuery.refetch()}
                        className="flex items-center px-3 py-2 border border-gray-300 dark:border-gray-600 hover:bg-gray-50 dark:hover:bg-gray-700 rounded-lg text-sm"
                        disabled={tunnelsQuery.isFetching}
                    >
                        <RefreshCw className={`w-4 h-4 mr-2 ${tunnelsQuery.isFetching ? 'animate-spin' : ''}`} />
                        Refresh
                    </button>
                    <button
                        onClick={() => setIsCreateModalOpen(true)}
                        className="flex items-center px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg font-medium"
                    >
                        <Plus className="w-4 h-4 mr-2" />
                        New Reservation
                    </button>
                </div>
            </div>

            {tunnelsQuery.isError && (
                <div className="mb-4 bg-red-50 dark:bg-red-900/20 text-red-700 dark:text-red-400 p-4 rounded-lg text-sm">
                    {(tunnelsQuery.error as Error).message}
                </div>
            )}
            {verifyError && (
                <div className="mb-4 bg-red-50 dark:bg-red-900/20 text-red-700 dark:text-red-400 p-4 rounded-lg text-sm whitespace-pre-wrap">
                    {verifyError}
                </div>
            )}

            {tunnels.length === 0 && !tunnelsQuery.isLoading ? (
                <EmptyState onCreate={() => setIsCreateModalOpen(true)} />
            ) : (
                <div className="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 overflow-hidden">
                    <div className="overflow-x-auto">
                        <table className="w-full text-left text-sm">
                            <thead className="bg-gray-50 dark:bg-gray-900/50 border-b border-gray-200 dark:border-gray-700">
                                <tr>
                                    <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Name</th>
                                    <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Status</th>
                                    <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Subdomain</th>
                                    <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Custom Domain</th>
                                    <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400 text-right">Actions</th>
                                </tr>
                            </thead>
                            <tbody className="divide-y divide-gray-200 dark:divide-gray-700">
                                {tunnels.map((t) => (
                                    <CloudTunnelRow
                                        key={t.id}
                                        tunnel={t}
                                        onCopy={copyToClipboard}
                                        onVerify={() => verifyMutation.mutate(t.id)}
                                        onDelete={() => {
                                            if (confirm(`Delete tunnel "${t.name}"?`)) deleteMutation.mutate(t.id);
                                        }}
                                        verifying={verifyMutation.isPending}
                                    />
                                ))}
                            </tbody>
                        </table>
                    </div>
                </div>
            )}

            {isCreateModalOpen && (
                <CreateModal
                    state={newTunnel}
                    setState={setNewTunnel}
                    onClose={() => setIsCreateModalOpen(false)}
                    onSubmit={() => createMutation.mutate()}
                    submitting={createMutation.isPending}
                    error={createMutation.error ? (createMutation.error as Error).message : null}
                />
            )}
        </div>
    );
};

const CloudTunnelRow: React.FC<{
    tunnel: TunnelReservation;
    onCopy: (s: string) => void;
    onVerify: () => void;
    onDelete: () => void;
    verifying: boolean;
}> = ({ tunnel, onCopy, onVerify, onDelete, verifying }) => {
    const isActive = tunnel.status === 'active';
    return (
        <tr className="hover:bg-gray-50 dark:hover:bg-gray-800/50 transition-colors">
            <td className="px-6 py-4">
                <div className="font-medium text-gray-900 dark:text-gray-100">{tunnel.name}</div>
                <div className="text-xs text-gray-500 font-mono mt-0.5">{tunnel.id}</div>
            </td>
            <td className="px-6 py-4">
                <span
                    className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium border ${
                        isActive
                            ? 'bg-green-50 text-green-700 border-green-200 dark:bg-green-900/20 dark:text-green-400 dark:border-green-900/30'
                            : 'bg-gray-100 text-gray-700 border-gray-200 dark:bg-gray-800 dark:text-gray-400 dark:border-gray-700'
                    }`}
                >
                    {isActive ? <Wifi className="w-3 h-3 mr-1" /> : <WifiOff className="w-3 h-3 mr-1" />}
                    {tunnel.status}
                </span>
            </td>
            <td className="px-6 py-4 font-mono text-gray-600 dark:text-gray-300">
                <div className="flex items-center gap-2">
                    <span>{tunnel.subdomain}</span>
                    <button
                        onClick={() => onCopy(tunnel.subdomain)}
                        className="p-1 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300"
                        title="Copy subdomain"
                    >
                        <Copy className="w-3.5 h-3.5" />
                    </button>
                </div>
            </td>
            <td className="px-6 py-4">
                {tunnel.custom_domain ? (
                    <div className="flex items-center gap-2">
                        <span className="font-mono text-gray-600 dark:text-gray-300 truncate max-w-[180px]">
                            {tunnel.custom_domain}
                        </span>
                        {tunnel.custom_domain_verified ? (
                            <span title="Verified" className="text-green-600 dark:text-green-400">
                                <ShieldCheck className="w-4 h-4" />
                            </span>
                        ) : (
                            <button
                                onClick={onVerify}
                                disabled={verifying}
                                className="inline-flex items-center px-2 py-0.5 rounded text-xs bg-yellow-100 text-yellow-800 hover:bg-yellow-200 dark:bg-yellow-900/30 dark:text-yellow-400 dark:hover:bg-yellow-900/50 disabled:opacity-50"
                                title="Run DNS verification"
                            >
                                <ShieldAlert className="w-3 h-3 mr-1" />
                                Verify
                            </button>
                        )}
                    </div>
                ) : (
                    <span className="text-gray-400 italic">none</span>
                )}
            </td>
            <td className="px-6 py-4 text-right">
                <button
                    onClick={onDelete}
                    className="p-2 text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20 rounded-lg"
                    title="Delete reservation"
                >
                    <Trash2 className="w-4 h-4" />
                </button>
            </td>
        </tr>
    );
};

const EmptyState: React.FC<{ onCreate: () => void }> = ({ onCreate }) => (
    <div className="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-12 text-center">
        <Globe className="w-16 h-16 mx-auto text-gray-400 mb-4" />
        <h3 className="text-lg font-medium text-gray-900 dark:text-gray-100 mb-2">No reservations yet</h3>
        <p className="text-gray-500 dark:text-gray-400 mb-6">
            Create a reservation to claim a public subdomain for your local mocks.
        </p>
        <button
            onClick={onCreate}
            className="px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700"
        >
            Create First Reservation
        </button>
    </div>
);

const CreateModal: React.FC<{
    state: { name: string; subdomain: string; custom_domain: string };
    setState: (s: { name: string; subdomain: string; custom_domain: string }) => void;
    onClose: () => void;
    onSubmit: () => void;
    submitting: boolean;
    error: string | null;
}> = ({ state, setState, onClose, onSubmit, submitting, error }) => (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm">
        <div className="bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-md w-full border border-gray-200 dark:border-gray-700">
            <div className="p-6 border-b border-gray-200 dark:border-gray-700">
                <h2 className="text-xl font-semibold text-gray-900 dark:text-gray-100">New Tunnel Reservation</h2>
            </div>
            <div className="p-6 space-y-4">
                {error && (
                    <div className="bg-red-50 dark:bg-red-900/20 text-red-700 dark:text-red-400 p-3 rounded text-sm whitespace-pre-wrap">
                        {error}
                    </div>
                )}
                <div className="space-y-2">
                    <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">Name</label>
                    <input
                        type="text"
                        value={state.name}
                        onChange={(e) => setState({ ...state, name: e.target.value })}
                        placeholder="e.g., Payment Service Dev"
                        className="w-full px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-lg outline-none focus:ring-2 focus:ring-blue-500"
                    />
                </div>
                <div className="space-y-2">
                    <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">Subdomain</label>
                    <input
                        type="text"
                        value={state.subdomain}
                        onChange={(e) => setState({ ...state, subdomain: e.target.value.toLowerCase() })}
                        placeholder="e.g., payments-dev"
                        className="w-full px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-lg outline-none focus:ring-2 focus:ring-blue-500 font-mono"
                    />
                    <p className="text-xs text-gray-500">3–40 lowercase chars, alphanumeric or hyphens.</p>
                </div>
                <div className="space-y-2">
                    <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">
                        Custom Domain <span className="text-gray-400">(optional)</span>
                    </label>
                    <input
                        type="text"
                        value={state.custom_domain}
                        onChange={(e) => setState({ ...state, custom_domain: e.target.value })}
                        placeholder="e.g., api.example.com"
                        className="w-full px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-lg outline-none focus:ring-2 focus:ring-blue-500 font-mono"
                    />
                    <p className="text-xs text-gray-500">
                        DNS verification runs against <code>_mockforge-verify.&lt;domain&gt;</code> after creation.
                    </p>
                </div>
            </div>
            <div className="p-6 border-t border-gray-200 dark:border-gray-700 flex justify-end gap-3">
                <button
                    onClick={onClose}
                    className="px-4 py-2 text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg"
                >
                    Cancel
                </button>
                <button
                    onClick={onSubmit}
                    disabled={!state.name || !state.subdomain || submitting}
                    className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg font-medium disabled:opacity-50 disabled:cursor-not-allowed"
                >
                    {submitting ? 'Creating…' : 'Create'}
                </button>
            </div>
        </div>
    </div>
);

// --- local view (sample data, retained for self-hosted demo) ---------------

const LocalTunnelsView: React.FC = () => {
    const [tunnels, setTunnels] = useState<LocalTunnel[]>([
        {
            id: 'tun_123',
            name: 'Payment Service Dev',
            local_port: 8080,
            public_url: 'https://payment-dev.mockforge.io',
            status: 'active',
            created_at: new Date().toISOString(),
            region: 'us-east-1',
        },
    ]);
    const [isCreateModalOpen, setIsCreateModalOpen] = useState(false);
    const [newTunnel, setNewTunnel] = useState({ name: '', port: '8080' });

    const handleCreate = () => {
        const tunnel: LocalTunnel = {
            id: `tun_${Math.random().toString(36).substr(2, 9)}`,
            name: newTunnel.name,
            local_port: parseInt(newTunnel.port),
            public_url: `https://${newTunnel.name.toLowerCase().replace(/\s+/g, '-')}.mockforge.io`,
            status: 'active',
            created_at: new Date().toISOString(),
            region: 'us-east-1',
        };
        setTunnels([...tunnels, tunnel]);
        setIsCreateModalOpen(false);
        setNewTunnel({ name: '', port: '8080' });
    };

    return (
        <div className="p-6 max-w-7xl mx-auto">
            <div className="bg-blue-50 dark:bg-blue-900/20 text-blue-800 dark:text-blue-300 p-4 rounded-lg mb-6 text-sm">
                Local mode shows sample data. In cloud mode this page reads real
                tunnel reservations from the registry.
            </div>
            <div className="flex justify-between items-start mb-8">
                <div>
                    <h1 className="text-2xl font-bold text-gray-900 dark:text-gray-100 mb-2">Tunnels</h1>
                </div>
                <button
                    onClick={() => setIsCreateModalOpen(true)}
                    className="flex items-center px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg font-medium"
                >
                    <Plus className="w-4 h-4 mr-2" />
                    Start Tunnel
                </button>
            </div>
            {tunnels.length > 0 && (
                <div className="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 overflow-hidden">
                    <table className="w-full text-left text-sm">
                        <thead className="bg-gray-50 dark:bg-gray-900/50 border-b border-gray-200 dark:border-gray-700">
                            <tr>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Name</th>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Local Port</th>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Public URL</th>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400 text-right">Actions</th>
                            </tr>
                        </thead>
                        <tbody className="divide-y divide-gray-200 dark:divide-gray-700">
                            {tunnels.map((t) => (
                                <tr key={t.id} className="hover:bg-gray-50 dark:hover:bg-gray-800/50">
                                    <td className="px-6 py-4 font-medium text-gray-900 dark:text-gray-100">{t.name}</td>
                                    <td className="px-6 py-4 font-mono">{t.local_port}</td>
                                    <td className="px-6 py-4 flex items-center gap-2">
                                        <span className="font-mono">{t.public_url}</span>
                                        <a href={t.public_url} target="_blank" rel="noopener noreferrer">
                                            <ExternalLink className="w-3.5 h-3.5 text-gray-400 hover:text-gray-600" />
                                        </a>
                                    </td>
                                    <td className="px-6 py-4 text-right">
                                        <button
                                            onClick={() => setTunnels(tunnels.filter((x) => x.id !== t.id))}
                                            className="p-2 text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20 rounded-lg"
                                        >
                                            <Trash2 className="w-4 h-4" />
                                        </button>
                                    </td>
                                </tr>
                            ))}
                        </tbody>
                    </table>
                </div>
            )}
            {isCreateModalOpen && (
                <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm">
                    <div className="bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-md w-full border border-gray-200 dark:border-gray-700">
                        <div className="p-6 border-b border-gray-200 dark:border-gray-700">
                            <h2 className="text-xl font-semibold">Start New Tunnel</h2>
                        </div>
                        <div className="p-6 space-y-4">
                            <input
                                type="text"
                                value={newTunnel.name}
                                onChange={(e) => setNewTunnel({ ...newTunnel, name: e.target.value })}
                                placeholder="Tunnel name"
                                className="w-full px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-lg"
                            />
                            <input
                                type="number"
                                value={newTunnel.port}
                                onChange={(e) => setNewTunnel({ ...newTunnel, port: e.target.value })}
                                placeholder="8080"
                                className="w-full px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-lg"
                            />
                        </div>
                        <div className="p-6 border-t border-gray-200 dark:border-gray-700 flex justify-end gap-3">
                            <button onClick={() => setIsCreateModalOpen(false)} className="px-4 py-2">
                                Cancel
                            </button>
                            <button
                                onClick={handleCreate}
                                disabled={!newTunnel.name || !newTunnel.port}
                                className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg disabled:opacity-50"
                            >
                                Start Tunnel
                            </button>
                        </div>
                    </div>
                </div>
            )}
        </div>
    );
};
