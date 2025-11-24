import React, { useState } from 'react';
import {
    Database,
    Table as TableIcon,
    History,
    Clock,
    Play,
    RotateCcw,
    Plus,
    Search,
    Save,
    MoreVertical,
    FileJson,
    Settings
} from 'lucide-react';

type Tab = 'entities' | 'data' | 'snapshots' | 'settings';

interface Entity {
    name: string;
    recordCount: number;
    columns: number;
    lastModified: string;
}

interface Snapshot {
    id: string;
    name: string;
    timestamp: string;
    description: string;
    size: string;
}

export const VirtualBackendsPage: React.FC = () => {
    const [activeTab, setActiveTab] = useState<Tab>('entities');

    // Mock Data
    const entities: Entity[] = [
        { name: 'users', recordCount: 150, columns: 8, lastModified: '2 mins ago' },
        { name: 'orders', recordCount: 1240, columns: 12, lastModified: 'Just now' },
        { name: 'products', recordCount: 56, columns: 6, lastModified: '1 hour ago' },
        { name: 'payments', recordCount: 890, columns: 10, lastModified: '5 mins ago' },
    ];

    const snapshots: Snapshot[] = [
        { id: 'snap_1', name: 'Pre-Migration Backup', timestamp: '2023-10-25 10:00 AM', description: 'Before applying v2 schema changes', size: '1.2 MB' },
        { id: 'snap_2', name: 'Clean State', timestamp: '2023-10-24 09:00 AM', description: 'Fresh database with seed data only', size: '0.5 MB' },
    ];

    return (
        <div className="p-6 max-w-7xl mx-auto h-full flex flex-col">
            {/* Header */}
            <div className="flex justify-between items-start mb-6">
                <div>
                    <div className="flex items-center gap-2 mb-2">
                        <h1 className="text-2xl font-bold text-gray-900 dark:text-gray-100">
                            Virtual Backend
                        </h1>
                        <span className="px-2 py-0.5 rounded-full bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400 text-xs font-medium border border-green-200 dark:border-green-900/50">
                            Running
                        </span>
                    </div>
                    <p className="text-gray-600 dark:text-gray-400">
                        Manage your stateful mock database, entities, and time-travel snapshots.
                    </p>
                </div>
                <div className="flex gap-2">
                    <button className="flex items-center px-3 py-2 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-lg text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors">
                        <Clock className="w-4 h-4 mr-2" />
                        Simulate Time
                    </button>
                    <button className="flex items-center px-3 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg font-medium transition-colors">
                        <Plus className="w-4 h-4 mr-2" />
                        New Entity
                    </button>
                </div>
            </div>

            {/* Tabs */}
            <div className="flex border-b border-gray-200 dark:border-gray-700 mb-6">
                <button
                    onClick={() => setActiveTab('entities')}
                    className={`px-4 py-2 font-medium text-sm flex items-center gap-2 border-b-2 transition-colors ${activeTab === 'entities'
                            ? 'border-blue-600 text-blue-600 dark:text-blue-400'
                            : 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300'
                        }`}
                >
                    <Database className="w-4 h-4" />
                    Entities & Schema
                </button>
                <button
                    onClick={() => setActiveTab('data')}
                    className={`px-4 py-2 font-medium text-sm flex items-center gap-2 border-b-2 transition-colors ${activeTab === 'data'
                            ? 'border-blue-600 text-blue-600 dark:text-blue-400'
                            : 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300'
                        }`}
                >
                    <TableIcon className="w-4 h-4" />
                    Data Explorer
                </button>
                <button
                    onClick={() => setActiveTab('snapshots')}
                    className={`px-4 py-2 font-medium text-sm flex items-center gap-2 border-b-2 transition-colors ${activeTab === 'snapshots'
                            ? 'border-blue-600 text-blue-600 dark:text-blue-400'
                            : 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300'
                        }`}
                >
                    <History className="w-4 h-4" />
                    Snapshots & Time Travel
                </button>
                <button
                    onClick={() => setActiveTab('settings')}
                    className={`px-4 py-2 font-medium text-sm flex items-center gap-2 border-b-2 transition-colors ${activeTab === 'settings'
                            ? 'border-blue-600 text-blue-600 dark:text-blue-400'
                            : 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300'
                        }`}
                >
                    <Settings className="w-4 h-4" />
                    Configuration
                </button>
            </div>

            {/* Content */}
            <div className="flex-1 bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 overflow-hidden">
                {activeTab === 'entities' && (
                    <div className="p-0">
                        <table className="w-full text-left text-sm">
                            <thead className="bg-gray-50 dark:bg-gray-900/50 border-b border-gray-200 dark:border-gray-700">
                                <tr>
                                    <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Entity Name</th>
                                    <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Records</th>
                                    <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Columns</th>
                                    <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Last Modified</th>
                                    <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400 text-right">Actions</th>
                                </tr>
                            </thead>
                            <tbody className="divide-y divide-gray-200 dark:divide-gray-700">
                                {entities.map((entity) => (
                                    <tr key={entity.name} className="hover:bg-gray-50 dark:hover:bg-gray-800/50 transition-colors">
                                        <td className="px-6 py-4">
                                            <div className="flex items-center gap-3">
                                                <div className="p-2 bg-blue-50 dark:bg-blue-900/20 rounded text-blue-600 dark:text-blue-400">
                                                    <TableIcon className="w-4 h-4" />
                                                </div>
                                                <span className="font-medium text-gray-900 dark:text-gray-100">{entity.name}</span>
                                            </div>
                                        </td>
                                        <td className="px-6 py-4 text-gray-600 dark:text-gray-300">{entity.recordCount.toLocaleString()}</td>
                                        <td className="px-6 py-4 text-gray-600 dark:text-gray-300">{entity.columns}</td>
                                        <td className="px-6 py-4 text-gray-500 dark:text-gray-400">{entity.lastModified}</td>
                                        <td className="px-6 py-4 text-right">
                                            <button className="text-blue-600 hover:text-blue-700 dark:text-blue-400 dark:hover:text-blue-300 font-medium text-sm">
                                                View Data
                                            </button>
                                        </td>
                                    </tr>
                                ))}
                            </tbody>
                        </table>
                    </div>
                )}

                {activeTab === 'data' && (
                    <div className="flex flex-col h-full">
                        <div className="p-4 border-b border-gray-200 dark:border-gray-700 flex gap-4 items-center bg-gray-50 dark:bg-gray-900/30">
                            <div className="relative flex-1 max-w-md">
                                <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-400" />
                                <input
                                    type="text"
                                    placeholder="Search records..."
                                    className="w-full pl-9 pr-4 py-2 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-lg text-sm focus:ring-2 focus:ring-blue-500 outline-none"
                                />
                            </div>
                            <div className="flex gap-2 ml-auto">
                                <button className="px-3 py-2 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-lg text-sm font-medium hover:bg-gray-50 dark:hover:bg-gray-700">
                                    Filter
                                </button>
                                <button className="px-3 py-2 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-lg text-sm font-medium hover:bg-gray-50 dark:hover:bg-gray-700">
                                    Export JSON
                                </button>
                            </div>
                        </div>
                        <div className="flex-1 flex items-center justify-center text-gray-500 dark:text-gray-400">
                            <div className="text-center">
                                <TableIcon className="w-12 h-12 mx-auto mb-3 opacity-20" />
                                <p>Select an entity to view records</p>
                            </div>
                        </div>
                    </div>
                )}

                {activeTab === 'snapshots' && (
                    <div className="p-6">
                        <div className="flex justify-between items-center mb-6">
                            <h3 className="text-lg font-medium text-gray-900 dark:text-gray-100">Database Snapshots</h3>
                            <button className="flex items-center px-3 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg text-sm font-medium transition-colors">
                                <Save className="w-4 h-4 mr-2" />
                                Create Snapshot
                            </button>
                        </div>

                        <div className="grid gap-4">
                            {snapshots.map((snap) => (
                                <div key={snap.id} className="flex items-center justify-between p-4 border border-gray-200 dark:border-gray-700 rounded-lg hover:border-blue-300 dark:hover:border-blue-700 transition-colors bg-white dark:bg-gray-800">
                                    <div className="flex items-start gap-4">
                                        <div className="p-3 bg-purple-50 dark:bg-purple-900/20 rounded-lg text-purple-600 dark:text-purple-400">
                                            <History className="w-6 h-6" />
                                        </div>
                                        <div>
                                            <h4 className="font-medium text-gray-900 dark:text-gray-100">{snap.name}</h4>
                                            <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">{snap.description}</p>
                                            <div className="flex items-center gap-4 mt-2 text-xs text-gray-400">
                                                <span>{snap.timestamp}</span>
                                                <span>•</span>
                                                <span>{snap.size}</span>
                                            </div>
                                        </div>
                                    </div>
                                    <div className="flex items-center gap-2">
                                        <button className="p-2 text-gray-500 hover:text-blue-600 hover:bg-blue-50 dark:hover:bg-blue-900/20 rounded-lg transition-colors" title="Restore">
                                            <RotateCcw className="w-5 h-5" />
                                        </button>
                                        <button className="p-2 text-gray-500 hover:text-gray-700 dark:hover:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors">
                                            <MoreVertical className="w-5 h-5" />
                                        </button>
                                    </div>
                                </div>
                            ))}
                        </div>
                    </div>
                )}

                {activeTab === 'settings' && (
                    <div className="p-6">
                        <div className="max-w-2xl">
                            <h3 className="text-lg font-medium text-gray-900 dark:text-gray-100 mb-4">Engine Configuration</h3>
                            <div className="space-y-4">
                                <div className="p-4 border border-gray-200 dark:border-gray-700 rounded-lg">
                                    <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">Storage Backend</label>
                                    <select className="w-full p-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-md">
                                        <option>SQLite (Persistent)</option>
                                        <option>In-Memory (Fast)</option>
                                        <option>JSON File (Portable)</option>
                                    </select>
                                </div>
                                <div className="p-4 border border-gray-200 dark:border-gray-700 rounded-lg">
                                    <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">Auto-Snapshot Interval</label>
                                    <select className="w-full p-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-md">
                                        <option>Disabled</option>
                                        <option>Every 1 hour</option>
                                        <option>Every 24 hours</option>
                                    </select>
                                </div>
                            </div>
                        </div>
                    </div>
                )}
            </div>
        </div>
    );
};
