import React, { useState, useCallback } from 'react';
import {
    Database,
    Table as TableIcon,
    History,
    Clock,
    RotateCcw,
    Search,
    Save,
    Settings,
    Loader2,
    AlertCircle,
    Trash2,
    RefreshCw,
} from 'lucide-react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { consistencyApi, snapshotsApi } from '../services/api';
import { cloudConsistencyApi } from '../services/api/cloudConsistency';
import { isCloudMode } from '../utils/cloudMode';
import { useWorkspaceStore } from '../stores/useWorkspaceStore';

type Tab = 'entities' | 'data' | 'snapshots' | 'settings';

interface EntitySummary {
    entityType: string;
    count: number;
    fields: number;
    lastModified: string;
}

export const VirtualBackendsPage: React.FC = () => {
    const [activeTab, setActiveTab] = useState<Tab>('entities');
    const [selectedEntityType, setSelectedEntityType] = useState<string | null>(null);
    const [searchQuery, setSearchQuery] = useState('');
    const [snapshotName, setSnapshotName] = useState('');
    const [snapshotDescription, setSnapshotDescription] = useState('');
    const [showCreateSnapshot, setShowCreateSnapshot] = useState(false);
    const queryClient = useQueryClient();

    // Cloud-mode branching (#461). The entities tab swaps over to the
    // registry's per-workspace consistency endpoints; the snapshots tab
    // points at the dedicated `cloud-snapshots` page since cloud snapshots
    // identify by UUID rather than name and warrant their own UI.
    const cloudMode = isCloudMode();
    const activeWorkspace = useWorkspaceStore((s) => s.activeWorkspace);
    const workspaceId = activeWorkspace?.id;

    // Fetch entities from consistency API
    const {
        data: entitiesData,
        isLoading: entitiesLoading,
        error: entitiesError,
    } = useQuery({
        queryKey: ['virtual-backend', 'entities', cloudMode ? workspaceId ?? null : 'local'],
        // In cloud mode we need an active workspace before we can hit the
        // per-workspace endpoint; the `enabled` flag below short-circuits
        // the query until one is selected.
        queryFn: () =>
            cloudMode
                ? cloudConsistencyApi.listEntities(workspaceId!)
                : consistencyApi.listEntities(),
        enabled: !cloudMode || !!workspaceId,
    });

    // Fetch snapshots
    const {
        data: snapshotsData,
        isLoading: snapshotsLoading,
        error: snapshotsError,
    } = useQuery({
        queryKey: ['virtual-backend', 'snapshots'],
        queryFn: () => snapshotsApi.listSnapshots(),
    });

    // Save snapshot mutation
    const saveSnapshotMutation = useMutation({
        mutationFn: ({ name, description }: { name: string; description?: string }) =>
            snapshotsApi.saveSnapshot(name, 'default', description),
        onSuccess: () => {
            queryClient.invalidateQueries({ queryKey: ['virtual-backend', 'snapshots'] });
            setSnapshotName('');
            setSnapshotDescription('');
            setShowCreateSnapshot(false);
        },
    });

    // Load snapshot mutation
    const loadSnapshotMutation = useMutation({
        mutationFn: (name: string) => snapshotsApi.loadSnapshot(name),
        onSuccess: () => {
            queryClient.invalidateQueries({ queryKey: ['virtual-backend'] });
        },
    });

    // Delete snapshot mutation
    const deleteSnapshotMutation = useMutation({
        mutationFn: (name: string) => snapshotsApi.deleteSnapshot(name),
        onSuccess: () => {
            queryClient.invalidateQueries({ queryKey: ['virtual-backend', 'snapshots'] });
        },
    });

    // Group entities by type for the summary view
    const entitySummaries: EntitySummary[] = React.useMemo(() => {
        if (!entitiesData?.entities) return [];
        const grouped = new Map<string, typeof entitiesData.entities>();
        for (const entity of entitiesData.entities) {
            const existing = grouped.get(entity.entity_type) ?? [];
            existing.push(entity);
            grouped.set(entity.entity_type, existing);
        }
        return Array.from(grouped.entries()).map(([type, entities]) => {
            const allKeys = new Set<string>();
            let lastModified = '';
            for (const e of entities) {
                if (e.data && typeof e.data === 'object') {
                    Object.keys(e.data).forEach((k) => allKeys.add(k));
                }
                if (!lastModified || e.updated_at > lastModified) {
                    lastModified = e.updated_at;
                }
            }
            return {
                entityType: type,
                count: entities.length,
                fields: allKeys.size,
                lastModified,
            };
        });
    }, [entitiesData]);

    // Filter entity records for the data tab
    const filteredRecords = React.useMemo(() => {
        if (!entitiesData?.entities || !selectedEntityType) return [];
        return entitiesData.entities
            .filter((e) => e.entity_type === selectedEntityType)
            .filter((e) => {
                if (!searchQuery) return true;
                const str = JSON.stringify(e.data).toLowerCase();
                return str.includes(searchQuery.toLowerCase());
            });
    }, [entitiesData, selectedEntityType, searchQuery]);

    const formatTime = useCallback((isoString: string) => {
        if (!isoString) return 'N/A';
        const date = new Date(isoString);
        const now = new Date();
        const diffMs = now.getTime() - date.getTime();
        const diffMins = Math.floor(diffMs / 60000);
        if (diffMins < 1) return 'Just now';
        if (diffMins < 60) return `${diffMins} min${diffMins === 1 ? '' : 's'} ago`;
        const diffHours = Math.floor(diffMins / 60);
        if (diffHours < 24) return `${diffHours} hour${diffHours === 1 ? '' : 's'} ago`;
        return date.toLocaleDateString();
    }, []);

    return (
        <div className="p-6 max-w-7xl mx-auto h-full flex flex-col">
            {/* Header */}
            <div className="flex justify-between items-start mb-6">
                <div>
                    <div className="flex items-center gap-2 mb-2">
                        <h1 className="text-2xl font-bold text-foreground">
                            Virtual Backend
                        </h1>
                        <span className="px-2 py-0.5 rounded-full bg-success-100 text-success-700 dark:bg-success-900/30 dark:text-success-400 text-xs font-medium border border-success-200 dark:border-success-900/50">
                            Running
                        </span>
                    </div>
                    <p className="text-muted-foreground">
                        Manage your stateful mock database, entities, and time-travel snapshots.
                    </p>
                </div>
                <div className="flex gap-2">
                    <button className="flex items-center px-3 py-2 bg-card border border-border rounded-lg text-foreground hover:bg-muted dark:hover:bg-gray-700 transition-colors">
                        <Clock className="w-4 h-4 mr-2" />
                        Simulate Time
                    </button>
                    <button
                        onClick={() => queryClient.invalidateQueries({ queryKey: ['virtual-backend'] })}
                        className="flex items-center px-3 py-2 bg-card border border-border rounded-lg text-foreground hover:bg-muted dark:hover:bg-gray-700 transition-colors"
                    >
                        <RefreshCw className="w-4 h-4 mr-2" />
                        Refresh
                    </button>
                </div>
            </div>

            {/* Tabs */}
            <div className="flex border-b border-border mb-6">
                <button
                    onClick={() => setActiveTab('entities')}
                    className={`px-4 py-2 font-medium text-sm flex items-center gap-2 border-b-2 transition-colors ${activeTab === 'entities'
                            ? 'border-info-600 text-info-600 dark:text-info-400'
                            : 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300'
                        }`}
                >
                    <Database className="w-4 h-4" />
                    Entities & Schema
                </button>
                <button
                    onClick={() => setActiveTab('data')}
                    className={`px-4 py-2 font-medium text-sm flex items-center gap-2 border-b-2 transition-colors ${activeTab === 'data'
                            ? 'border-info-600 text-info-600 dark:text-info-400'
                            : 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300'
                        }`}
                >
                    <TableIcon className="w-4 h-4" />
                    Data Explorer
                </button>
                <button
                    onClick={() => setActiveTab('snapshots')}
                    className={`px-4 py-2 font-medium text-sm flex items-center gap-2 border-b-2 transition-colors ${activeTab === 'snapshots'
                            ? 'border-info-600 text-info-600 dark:text-info-400'
                            : 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300'
                        }`}
                >
                    <History className="w-4 h-4" />
                    Snapshots & Time Travel
                </button>
                <button
                    onClick={() => setActiveTab('settings')}
                    className={`px-4 py-2 font-medium text-sm flex items-center gap-2 border-b-2 transition-colors ${activeTab === 'settings'
                            ? 'border-info-600 text-info-600 dark:text-info-400'
                            : 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300'
                        }`}
                >
                    <Settings className="w-4 h-4" />
                    Configuration
                </button>
            </div>

            {/* Content */}
            <div className="flex-1 bg-card rounded-xl shadow-sm border border-border overflow-hidden">
                {activeTab === 'entities' && cloudMode && !workspaceId ? (
                    <div className="flex flex-col items-center justify-center py-16 text-muted-foreground">
                        <Database className="w-12 h-12 mb-3 opacity-20" />
                        <p>Select a workspace to view virtual entities.</p>
                    </div>
                ) : null}

                {activeTab === 'entities' && (!cloudMode || !!workspaceId) && (
                    <div className="p-0">
                        {entitiesLoading ? (
                            <div className="flex items-center justify-center py-16">
                                <Loader2 className="w-8 h-8 animate-spin text-muted-foreground" />
                            </div>
                        ) : entitiesError ? (
                            <div className="flex flex-col items-center justify-center py-16 text-muted-foreground">
                                <AlertCircle className="w-12 h-12 mb-3 opacity-30" />
                                <p>Failed to load entities</p>
                                <p className="text-sm mt-1">{entitiesError instanceof Error ? entitiesError.message : 'Unknown error'}</p>
                            </div>
                        ) : entitySummaries.length === 0 ? (
                            <div className="flex flex-col items-center justify-center py-16 text-muted-foreground">
                                <Database className="w-12 h-12 mb-3 opacity-20" />
                                <p>No entities registered yet</p>
                                <p className="text-sm mt-1">Entities appear here as they are created through API interactions.</p>
                            </div>
                        ) : (
                            <table className="w-full text-left text-sm">
                                <thead className="bg-muted/50 border-b border-border">
                                    <tr>
                                        <th className="px-6 py-4 font-medium text-muted-foreground">Entity Type</th>
                                        <th className="px-6 py-4 font-medium text-muted-foreground">Records</th>
                                        <th className="px-6 py-4 font-medium text-muted-foreground">Fields</th>
                                        <th className="px-6 py-4 font-medium text-muted-foreground">Last Modified</th>
                                        <th className="px-6 py-4 font-medium text-muted-foreground text-right">Actions</th>
                                    </tr>
                                </thead>
                                <tbody className="divide-y divide-border">
                                    {entitySummaries.map((entity) => (
                                        <tr key={entity.entityType} className="hover:bg-accent hover:text-accent-foreground/50 transition-colors">
                                            <td className="px-6 py-4">
                                                <div className="flex items-center gap-3">
                                                    <div className="p-2 bg-info-50 dark:bg-info-900/20 rounded text-info-600 dark:text-info-400">
                                                        <TableIcon className="w-4 h-4" />
                                                    </div>
                                                    <span className="font-medium text-foreground">{entity.entityType}</span>
                                                </div>
                                            </td>
                                            <td className="px-6 py-4 text-muted-foreground">{entity.count.toLocaleString()}</td>
                                            <td className="px-6 py-4 text-muted-foreground">{entity.fields}</td>
                                            <td className="px-6 py-4 text-muted-foreground">{formatTime(entity.lastModified)}</td>
                                            <td className="px-6 py-4 text-right">
                                                <button
                                                    onClick={() => {
                                                        setSelectedEntityType(entity.entityType);
                                                        setActiveTab('data');
                                                    }}
                                                    className="text-info-600 hover:text-info-700 dark:text-info-400 dark:hover:text-info-300 font-medium text-sm"
                                                >
                                                    View Data
                                                </button>
                                            </td>
                                        </tr>
                                    ))}
                                </tbody>
                            </table>
                        )}
                    </div>
                )}

                {activeTab === 'data' && (
                    <div className="flex flex-col h-full">
                        <div className="p-4 border-b border-border flex gap-4 items-center bg-muted/30">
                            <select
                                value={selectedEntityType ?? ''}
                                onChange={(e) => setSelectedEntityType(e.target.value || null)}
                                className="px-3 py-2 bg-card border border-border rounded-lg text-sm"
                            >
                                <option value="">Select entity type...</option>
                                {entitySummaries.map((e) => (
                                    <option key={e.entityType} value={e.entityType}>
                                        {e.entityType} ({e.count})
                                    </option>
                                ))}
                            </select>
                            <div className="relative flex-1 max-w-md">
                                <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
                                <input
                                    type="text"
                                    placeholder="Search records..."
                                    value={searchQuery}
                                    onChange={(e) => setSearchQuery(e.target.value)}
                                    className="w-full pl-9 pr-4 py-2 bg-card border border-border rounded-lg text-sm focus:ring-2 focus:ring-blue-500 outline-none"
                                />
                            </div>
                        </div>
                        {!selectedEntityType ? (
                            <div className="flex-1 flex items-center justify-center text-muted-foreground">
                                <div className="text-center">
                                    <TableIcon className="w-12 h-12 mx-auto mb-3 opacity-20" />
                                    <p>Select an entity type to view records</p>
                                </div>
                            </div>
                        ) : entitiesLoading ? (
                            <div className="flex-1 flex items-center justify-center">
                                <Loader2 className="w-8 h-8 animate-spin text-muted-foreground" />
                            </div>
                        ) : filteredRecords.length === 0 ? (
                            <div className="flex-1 flex items-center justify-center text-muted-foreground">
                                <div className="text-center">
                                    <TableIcon className="w-12 h-12 mx-auto mb-3 opacity-20" />
                                    <p>No records found for &quot;{selectedEntityType}&quot;</p>
                                </div>
                            </div>
                        ) : (
                            <div className="overflow-auto flex-1">
                                <table className="w-full text-left text-sm">
                                    <thead className="bg-muted/50 border-b border-border sticky top-0">
                                        <tr>
                                            <th className="px-6 py-3 font-medium text-muted-foreground">ID</th>
                                            <th className="px-6 py-3 font-medium text-muted-foreground">Data</th>
                                            <th className="px-6 py-3 font-medium text-muted-foreground">Protocols</th>
                                            <th className="px-6 py-3 font-medium text-muted-foreground">Updated</th>
                                        </tr>
                                    </thead>
                                    <tbody className="divide-y divide-border">
                                        {filteredRecords.map((record) => (
                                            <tr key={`${record.entity_type}:${record.entity_id}`} className="hover:bg-accent hover:text-accent-foreground/50">
                                                <td className="px-6 py-3 font-mono text-xs text-foreground">
                                                    {record.entity_id}
                                                </td>
                                                <td className="px-6 py-3">
                                                    <pre className="text-xs text-muted-foreground max-w-lg truncate">
                                                        {JSON.stringify(record.data, null, 2).slice(0, 200)}
                                                    </pre>
                                                </td>
                                                <td className="px-6 py-3">
                                                    <div className="flex gap-1 flex-wrap">
                                                        {record.seen_in_protocols.map((p) => (
                                                            <span
                                                                key={p}
                                                                className="px-1.5 py-0.5 bg-muted dark:bg-gray-700 rounded text-xs text-muted-foreground"
                                                            >
                                                                {p}
                                                            </span>
                                                        ))}
                                                    </div>
                                                </td>
                                                <td className="px-6 py-3 text-muted-foreground text-xs">
                                                    {formatTime(record.updated_at)}
                                                </td>
                                            </tr>
                                        ))}
                                    </tbody>
                                </table>
                            </div>
                        )}
                    </div>
                )}

                {activeTab === 'snapshots' && cloudMode && (
                    // Cloud snapshots identify by UUID (not name) and are
                    // surfaced on the dedicated Cloud Snapshots page (#10).
                    // Pointing both UIs at the same data would just duplicate
                    // the controls — link out instead of half-wiring it here.
                    <div className="flex flex-col items-center justify-center py-16 text-muted-foreground">
                        <History className="w-12 h-12 mb-3 opacity-30" />
                        <p>Snapshots in cloud mode live on the dedicated page.</p>
                        <a
                            href="/cloud-snapshots"
                            className="mt-3 inline-flex items-center px-3 py-2 bg-primary text-primary-foreground hover:bg-primary/90 rounded-lg text-sm font-medium transition-colors"
                        >
                            <History className="w-4 h-4 mr-2" />
                            Open Cloud Snapshots
                        </a>
                    </div>
                )}

                {activeTab === 'snapshots' && !cloudMode && (
                    <div className="p-6">
                        <div className="flex justify-between items-center mb-6">
                            <h3 className="text-lg font-medium text-foreground">Database Snapshots</h3>
                            <button
                                onClick={() => setShowCreateSnapshot(!showCreateSnapshot)}
                                className="flex items-center px-3 py-2 bg-primary text-primary-foreground hover:bg-primary/90 rounded-lg text-sm font-medium transition-colors"
                            >
                                <Save className="w-4 h-4 mr-2" />
                                Create Snapshot
                            </button>
                        </div>

                        {showCreateSnapshot && (
                            <div className="mb-6 p-4 border border-info-200 dark:border-info-800 rounded-lg bg-info-50 dark:bg-info-900/20">
                                <div className="flex gap-3 items-end">
                                    <div className="flex-1">
                                        <label className="block text-sm font-medium text-foreground mb-1">Name</label>
                                        <input
                                            type="text"
                                            value={snapshotName}
                                            onChange={(e) => setSnapshotName(e.target.value)}
                                            placeholder="e.g., pre-migration-backup"
                                            className="w-full px-3 py-2 bg-card border border-border rounded-lg text-sm"
                                        />
                                    </div>
                                    <div className="flex-1">
                                        <label className="block text-sm font-medium text-foreground mb-1">Description</label>
                                        <input
                                            type="text"
                                            value={snapshotDescription}
                                            onChange={(e) => setSnapshotDescription(e.target.value)}
                                            placeholder="Optional description..."
                                            className="w-full px-3 py-2 bg-card border border-border rounded-lg text-sm"
                                        />
                                    </div>
                                    <button
                                        onClick={() => {
                                            if (snapshotName.trim()) {
                                                saveSnapshotMutation.mutate({
                                                    name: snapshotName.trim(),
                                                    description: snapshotDescription.trim() || undefined,
                                                });
                                            }
                                        }}
                                        disabled={!snapshotName.trim() || saveSnapshotMutation.isPending}
                                        className="px-4 py-2 bg-primary hover:bg-primary/90 disabled:opacity-50 text-white rounded-lg text-sm font-medium transition-colors"
                                    >
                                        {saveSnapshotMutation.isPending ? 'Saving...' : 'Save'}
                                    </button>
                                </div>
                            </div>
                        )}

                        {snapshotsLoading ? (
                            <div className="flex items-center justify-center py-12">
                                <Loader2 className="w-8 h-8 animate-spin text-muted-foreground" />
                            </div>
                        ) : snapshotsError ? (
                            <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
                                <AlertCircle className="w-12 h-12 mb-3 opacity-30" />
                                <p>Failed to load snapshots</p>
                            </div>
                        ) : !snapshotsData?.snapshots?.length ? (
                            <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
                                <History className="w-12 h-12 mb-3 opacity-20" />
                                <p>No snapshots yet</p>
                                <p className="text-sm mt-1">Create a snapshot to save the current state.</p>
                            </div>
                        ) : (
                            <div className="grid gap-4">
                                {snapshotsData.snapshots.map((snap) => (
                                    <div key={snap.name} className="flex items-center justify-between p-4 border border-border rounded-lg hover:border-info-300 dark:hover:border-info-700 transition-colors bg-card">
                                        <div className="flex items-start gap-4">
                                            <div className="p-3 bg-purple-50 dark:bg-purple-900/20 rounded-lg text-purple-600 dark:text-purple-400">
                                                <History className="w-6 h-6" />
                                            </div>
                                            <div>
                                                <h4 className="font-medium text-foreground">{snap.name}</h4>
                                                {snap.description && (
                                                    <p className="text-sm text-muted-foreground mt-1">{snap.description}</p>
                                                )}
                                                <div className="flex items-center gap-4 mt-2 text-xs text-muted-foreground">
                                                    <span>{formatTime(snap.created_at)}</span>
                                                    <span>&#8226;</span>
                                                    <span>Workspace: {snap.workspace}</span>
                                                </div>
                                            </div>
                                        </div>
                                        <div className="flex items-center gap-2">
                                            <button
                                                onClick={() => loadSnapshotMutation.mutate(snap.name)}
                                                disabled={loadSnapshotMutation.isPending}
                                                className="p-2 text-muted-foreground hover:text-info-600 hover:bg-info-50 dark:hover:bg-info-900/20 rounded-lg transition-colors"
                                                title="Restore"
                                            >
                                                <RotateCcw className="w-5 h-5" />
                                            </button>
                                            <button
                                                onClick={() => deleteSnapshotMutation.mutate(snap.name)}
                                                disabled={deleteSnapshotMutation.isPending}
                                                className="p-2 text-muted-foreground hover:text-danger-600 hover:bg-danger-50 dark:hover:bg-danger-900/20 rounded-lg transition-colors"
                                                title="Delete"
                                            >
                                                <Trash2 className="w-5 h-5" />
                                            </button>
                                        </div>
                                    </div>
                                ))}
                            </div>
                        )}
                    </div>
                )}

                {activeTab === 'settings' && (
                    <div className="p-6">
                        <div className="max-w-2xl">
                            <h3 className="text-lg font-medium text-foreground mb-4">Engine Configuration</h3>
                            <div className="space-y-4">
                                <div className="p-4 border border-border rounded-lg">
                                    <label className="block text-sm font-medium text-foreground mb-2">Storage Backend</label>
                                    <select className="w-full p-2 bg-card border border-border rounded-md">
                                        <option>SQLite (Persistent)</option>
                                        <option>In-Memory (Fast)</option>
                                        <option>JSON File (Portable)</option>
                                    </select>
                                </div>
                                <div className="p-4 border border-border rounded-lg">
                                    <label className="block text-sm font-medium text-foreground mb-2">Auto-Snapshot Interval</label>
                                    <select className="w-full p-2 bg-card border border-border rounded-md">
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
