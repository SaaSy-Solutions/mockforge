import React, { useState } from 'react';
import {
    Plus,
    RefreshCw,
    Trash2,
    Globe,
    Copy,
    Wifi,
    WifiOff,
    ExternalLink
} from 'lucide-react';

interface Tunnel {
    id: string;
    name: string;
    local_port: number;
    public_url: string;
    status: 'active' | 'inactive' | 'error';
    created_at: string;
    region: string;
}

export const TunnelsPage: React.FC = () => {
    const [tunnels, setTunnels] = useState<Tunnel[]>([
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
        const tunnel: Tunnel = {
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

    const handleDelete = (id: string) => {
        setTunnels(tunnels.filter(t => t.id !== id));
    };

    const copyToClipboard = (text: string) => {
        navigator.clipboard.writeText(text);
    };

    return (
        <div className="p-6 max-w-7xl mx-auto">
            <div className="flex justify-between items-start mb-8">
                <div>
                    <h1 className="text-2xl font-bold text-gray-900 dark:text-gray-100 mb-2">
                        Tunnels
                    </h1>
                    <p className="text-gray-600 dark:text-gray-400">
                        Expose your local mock servers to the internet via secure tunnels.
                    </p>
                </div>
                <button
                    onClick={() => setIsCreateModalOpen(true)}
                    className="flex items-center px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg font-medium transition-colors"
                >
                    <Plus className="w-4 h-4 mr-2" />
                    Start Tunnel
                </button>
            </div>

            {tunnels.length === 0 ? (
                <div className="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-12 text-center">
                    <Globe className="w-16 h-16 mx-auto text-gray-400 mb-4" />
                    <h3 className="text-lg font-medium text-gray-900 dark:text-gray-100 mb-2">
                        No Active Tunnels
                    </h3>
                    <p className="text-gray-500 dark:text-gray-400 mb-6">
                        Create a tunnel to share your local mocks with external services or teammates.
                    </p>
                    <button
                        onClick={() => setIsCreateModalOpen(true)}
                        className="px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
                    >
                        Create First Tunnel
                    </button>
                </div>
            ) : (
                <div className="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 overflow-hidden">
                    <div className="overflow-x-auto">
                        <table className="w-full text-left text-sm">
                            <thead className="bg-gray-50 dark:bg-gray-900/50 border-b border-gray-200 dark:border-gray-700">
                                <tr>
                                    <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Name</th>
                                    <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Status</th>
                                    <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Local Port</th>
                                    <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Public URL</th>
                                    <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Region</th>
                                    <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400 text-right">Actions</th>
                                </tr>
                            </thead>
                            <tbody className="divide-y divide-gray-200 dark:divide-gray-700">
                                {tunnels.map((tunnel) => (
                                    <tr key={tunnel.id} className="hover:bg-gray-50 dark:hover:bg-gray-800/50 transition-colors">
                                        <td className="px-6 py-4">
                                            <div className="font-medium text-gray-900 dark:text-gray-100">{tunnel.name}</div>
                                            <div className="text-xs text-gray-500 font-mono mt-0.5">{tunnel.id}</div>
                                        </td>
                                        <td className="px-6 py-4">
                                            <span className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium border ${tunnel.status === 'active'
                                                    ? 'bg-green-50 text-green-700 border-green-200 dark:bg-green-900/20 dark:text-green-400 dark:border-green-900/30'
                                                    : 'bg-gray-100 text-gray-700 border-gray-200 dark:bg-gray-800 dark:text-gray-400 dark:border-gray-700'
                                                }`}>
                                                {tunnel.status === 'active' ? <Wifi className="w-3 h-3 mr-1" /> : <WifiOff className="w-3 h-3 mr-1" />}
                                                {tunnel.status}
                                            </span>
                                        </td>
                                        <td className="px-6 py-4 text-gray-600 dark:text-gray-300 font-mono">
                                            {tunnel.local_port}
                                        </td>
                                        <td className="px-6 py-4">
                                            <div className="flex items-center gap-2">
                                                <span className="font-mono text-gray-600 dark:text-gray-300 truncate max-w-[200px]">
                                                    {tunnel.public_url}
                                                </span>
                                                <button
                                                    onClick={() => copyToClipboard(tunnel.public_url)}
                                                    className="p-1 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 rounded transition-colors"
                                                    title="Copy URL"
                                                >
                                                    <Copy className="w-3.5 h-3.5" />
                                                </button>
                                                <a
                                                    href={tunnel.public_url}
                                                    target="_blank"
                                                    rel="noopener noreferrer"
                                                    className="p-1 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 rounded transition-colors"
                                                    title="Open URL"
                                                >
                                                    <ExternalLink className="w-3.5 h-3.5" />
                                                </a>
                                            </div>
                                        </td>
                                        <td className="px-6 py-4 text-gray-600 dark:text-gray-300">
                                            {tunnel.region}
                                        </td>
                                        <td className="px-6 py-4 text-right">
                                            <button
                                                onClick={() => handleDelete(tunnel.id)}
                                                className="p-2 text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20 rounded-lg transition-colors"
                                                title="Stop Tunnel"
                                            >
                                                <Trash2 className="w-4 h-4" />
                                            </button>
                                        </td>
                                    </tr>
                                ))}
                            </tbody>
                        </table>
                    </div>
                </div>
            )}

            {/* Create Modal */}
            {isCreateModalOpen && (
                <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm">
                    <div className="bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-md w-full border border-gray-200 dark:border-gray-700">
                        <div className="p-6 border-b border-gray-200 dark:border-gray-700">
                            <h2 className="text-xl font-semibold text-gray-900 dark:text-gray-100">Start New Tunnel</h2>
                        </div>
                        <div className="p-6 space-y-4">
                            <div className="bg-blue-50 dark:bg-blue-900/20 text-blue-700 dark:text-blue-300 p-4 rounded-lg text-sm">
                                This will create a secure tunnel from a public URL to your local machine.
                            </div>

                            <div className="space-y-2">
                                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">
                                    Tunnel Name
                                </label>
                                <input
                                    type="text"
                                    value={newTunnel.name}
                                    onChange={(e) => setNewTunnel({ ...newTunnel, name: e.target.value })}
                                    placeholder="e.g., My Payment Mock"
                                    className="w-full px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500 outline-none transition-all"
                                />
                            </div>

                            <div className="space-y-2">
                                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">
                                    Local Port
                                </label>
                                <input
                                    type="number"
                                    value={newTunnel.port}
                                    onChange={(e) => setNewTunnel({ ...newTunnel, port: e.target.value })}
                                    placeholder="8080"
                                    className="w-full px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500 outline-none transition-all"
                                />
                                <p className="text-xs text-gray-500">The port your mock server is running on locally</p>
                            </div>
                        </div>
                        <div className="p-6 border-t border-gray-200 dark:border-gray-700 flex justify-end gap-3">
                            <button
                                onClick={() => setIsCreateModalOpen(false)}
                                className="px-4 py-2 text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
                            >
                                Cancel
                            </button>
                            <button
                                onClick={handleCreate}
                                disabled={!newTunnel.name || !newTunnel.port}
                                className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
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
