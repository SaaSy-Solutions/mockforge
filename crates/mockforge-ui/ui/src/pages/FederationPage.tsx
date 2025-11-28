import React, { useState } from 'react';
import {
    Plus,
    Link as LinkIcon,
    Link2Off,
    Share2,
    Building2,
    Settings,
    CheckCircle2,
    AlertCircle
} from 'lucide-react';

interface FederatedWorkspace {
    id: string;
    name: string;
    url: string;
    status: 'connected' | 'disconnected' | 'error';
    shared_contracts: number;
    last_sync: string;
}

export const FederationPage: React.FC = () => {
    const [workspaces, setWorkspaces] = useState<FederatedWorkspace[]>([
        {
            id: 'ws_abc123',
            name: 'Payment Team',
            url: 'https://mockforge.payment-team.internal',
            status: 'connected',
            shared_contracts: 12,
            last_sync: new Date().toISOString(),
        },
        {
            id: 'ws_xyz789',
            name: 'Inventory Service',
            url: 'https://mockforge.inventory.internal',
            status: 'disconnected',
            shared_contracts: 5,
            last_sync: new Date(Date.now() - 86400000).toISOString(),
        },
    ]);
    const [isConnectModalOpen, setIsConnectModalOpen] = useState(false);
    const [newConnection, setNewConnection] = useState({ url: '', token: '' });

    const handleConnect = () => {
        const workspace: FederatedWorkspace = {
            id: `ws_${Math.random().toString(36).substr(2, 9)}`,
            name: 'New Workspace', // In real app, would fetch name from URL
            url: newConnection.url,
            status: 'connected',
            shared_contracts: 0,
            last_sync: new Date().toISOString(),
        };
        setWorkspaces([...workspaces, workspace]);
        setIsConnectModalOpen(false);
        setNewConnection({ url: '', token: '' });
    };

    const handleDisconnect = (id: string) => {
        setWorkspaces(workspaces.map(ws =>
            ws.id === id ? { ...ws, status: 'disconnected' } : ws
        ));
    };

    return (
        <div className="p-6 max-w-7xl mx-auto">
            <div className="flex justify-between items-start mb-8">
                <div>
                    <h1 className="text-2xl font-bold text-gray-900 dark:text-gray-100 mb-2">
                        Federation
                    </h1>
                    <p className="text-gray-600 dark:text-gray-400">
                        Connect and compose multiple MockForge workspaces into a unified virtual system.
                    </p>
                </div>
                <button
                    onClick={() => setIsConnectModalOpen(true)}
                    className="flex items-center px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg font-medium transition-colors"
                >
                    <Plus className="w-4 h-4 mr-2" />
                    Connect Workspace
                </button>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
                {/* Status Overview */}
                <div className="md:col-span-1">
                    <div className="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-6 h-full">
                        <h3 className="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-4">
                            Federation Status
                        </h3>
                        <div className="flex items-center gap-4 mb-6">
                            <div className="p-3 bg-blue-100 dark:bg-blue-900/30 rounded-full text-blue-600 dark:text-blue-400">
                                <Share2 className="w-8 h-8" />
                            </div>
                            <div>
                                <div className="text-3xl font-bold text-gray-900 dark:text-gray-100">
                                    {workspaces.filter(w => w.status === 'connected').length}
                                </div>
                                <div className="text-sm text-gray-500 dark:text-gray-400">Active Connections</div>
                            </div>
                        </div>
                        <hr className="border-gray-200 dark:border-gray-700 my-4" />
                        <p className="text-sm text-gray-600 dark:text-gray-400 leading-relaxed">
                            Federation allows you to import contracts, fixtures, and scenarios from other workspaces.
                            Changes in upstream workspaces can trigger alerts or automated updates.
                        </p>
                    </div>
                </div>

                {/* Connected Workspaces List */}
                <div className="md:col-span-2">
                    <div className="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 overflow-hidden">
                        <div className="p-6 border-b border-gray-200 dark:border-gray-700">
                            <h3 className="text-lg font-semibold text-gray-900 dark:text-gray-100">
                                Connected Workspaces
                            </h3>
                        </div>
                        <ul className="divide-y divide-gray-200 dark:divide-gray-700">
                            {workspaces.map((ws) => (
                                <li key={ws.id} className="p-6 hover:bg-gray-50 dark:hover:bg-gray-800/50 transition-colors">
                                    <div className="flex items-center justify-between">
                                        <div className="flex items-center gap-4">
                                            <div className="p-2 bg-gray-100 dark:bg-gray-700 rounded-lg text-gray-600 dark:text-gray-300">
                                                <Building2 className="w-6 h-6" />
                                            </div>
                                            <div>
                                                <div className="flex items-center gap-2">
                                                    <h4 className="font-medium text-gray-900 dark:text-gray-100">{ws.name}</h4>
                                                    <span className={`inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium border ${ws.status === 'connected'
                                                            ? 'bg-green-50 text-green-700 border-green-200 dark:bg-green-900/20 dark:text-green-400 dark:border-green-900/30'
                                                            : 'bg-gray-100 text-gray-700 border-gray-200 dark:bg-gray-800 dark:text-gray-400 dark:border-gray-700'
                                                        }`}>
                                                        {ws.status}
                                                    </span>
                                                </div>
                                                <div className="flex items-center gap-2 mt-1 text-sm text-gray-500 dark:text-gray-400">
                                                    <span className="font-mono">{ws.url}</span>
                                                    <span>•</span>
                                                    <span>{ws.shared_contracts} shared contracts</span>
                                                </div>
                                            </div>
                                        </div>
                                        <div className="flex items-center gap-2">
                                            <button className="p-2 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors">
                                                <Settings className="w-5 h-5" />
                                            </button>
                                            <button
                                                onClick={() => handleDisconnect(ws.id)}
                                                className={`p-2 rounded-lg transition-colors ${ws.status === 'connected'
                                                        ? 'text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20'
                                                        : 'text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700'
                                                    }`}
                                                title={ws.status === 'connected' ? 'Disconnect' : 'Connect'}
                                            >
                                                {ws.status === 'connected' ? <Link2Off className="w-5 h-5" /> : <LinkIcon className="w-5 h-5" />}
                                            </button>
                                        </div>
                                    </div>
                                </li>
                            ))}
                        </ul>
                    </div>
                </div>
            </div>

            {/* Connect Modal */}
            {isConnectModalOpen && (
                <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm">
                    <div className="bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-md w-full border border-gray-200 dark:border-gray-700">
                        <div className="p-6 border-b border-gray-200 dark:border-gray-700">
                            <h2 className="text-xl font-semibold text-gray-900 dark:text-gray-100">Connect Remote Workspace</h2>
                        </div>
                        <div className="p-6 space-y-4">
                            <div className="bg-blue-50 dark:bg-blue-900/20 text-blue-700 dark:text-blue-300 p-4 rounded-lg text-sm flex gap-3">
                                <AlertCircle className="w-5 h-5 shrink-0" />
                                <p>Enter the URL and access token of the MockForge workspace you want to connect to.</p>
                            </div>

                            <div className="space-y-2">
                                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">
                                    Workspace URL
                                </label>
                                <input
                                    type="text"
                                    value={newConnection.url}
                                    onChange={(e) => setNewConnection({ ...newConnection, url: e.target.value })}
                                    placeholder="https://mockforge.example.com"
                                    className="w-full px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500 outline-none transition-all"
                                />
                            </div>

                            <div className="space-y-2">
                                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">
                                    Access Token
                                </label>
                                <input
                                    type="password"
                                    value={newConnection.token}
                                    onChange={(e) => setNewConnection({ ...newConnection, token: e.target.value })}
                                    placeholder="mf_token_..."
                                    className="w-full px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500 outline-none transition-all"
                                />
                            </div>
                        </div>
                        <div className="p-6 border-t border-gray-200 dark:border-gray-700 flex justify-end gap-3">
                            <button
                                onClick={() => setIsConnectModalOpen(false)}
                                className="px-4 py-2 text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
                            >
                                Cancel
                            </button>
                            <button
                                onClick={handleConnect}
                                disabled={!newConnection.url || !newConnection.token}
                                className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                            >
                                Connect
                            </button>
                        </div>
                    </div>
                </div>
            )}
        </div>
    );
};
