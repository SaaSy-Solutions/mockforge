/**
 * Cloud Incidents — org-scoped incident dashboard (#3).
 *
 * Distinct from the existing IncidentDashboardPage which renders drift
 * incidents (specific consumer-impact subsystem). This page wraps the
 * generic cloud incidents feature: anything raised through Incident::raise
 * (chaos abort, contract drift, hosted-mock health, external monitor) lands
 * here.
 */
import React, { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
    AlertTriangle,
    CheckCircle2,
    Clock,
    RefreshCw,
    AlertCircle,
    Activity,
    ChevronRight,
} from 'lucide-react';
import { isCloudMode } from '../utils/cloudMode';
import { useCloudOrgId } from '../hooks/useCloudOrgId';
import {
    cloudIncidentsApi,
    type Incident,
    type IncidentSeverity,
    type IncidentStats,
    type IncidentStatus,
} from '../services/api/cloudIncidents';

const SEVERITY_STYLES: Record<IncidentSeverity, string> = {
    critical:
        'bg-red-50 text-red-700 border-red-200 dark:bg-red-900/20 dark:text-red-400 dark:border-red-900/30',
    high: 'bg-orange-50 text-orange-700 border-orange-200 dark:bg-orange-900/20 dark:text-orange-400 dark:border-orange-900/30',
    medium:
        'bg-yellow-50 text-yellow-700 border-yellow-200 dark:bg-yellow-900/20 dark:text-yellow-400 dark:border-yellow-900/30',
    low: 'bg-blue-50 text-blue-700 border-blue-200 dark:bg-blue-900/20 dark:text-blue-400 dark:border-blue-900/30',
};

const STATUS_STYLES: Record<IncidentStatus, string> = {
    open: 'bg-red-50 text-red-700 border-red-200 dark:bg-red-900/20 dark:text-red-400 dark:border-red-900/30',
    acknowledged:
        'bg-yellow-50 text-yellow-700 border-yellow-200 dark:bg-yellow-900/20 dark:text-yellow-400 dark:border-yellow-900/30',
    resolved:
        'bg-green-50 text-green-700 border-green-200 dark:bg-green-900/20 dark:text-green-400 dark:border-green-900/30',
    reopened:
        'bg-orange-50 text-orange-700 border-orange-200 dark:bg-orange-900/20 dark:text-orange-400 dark:border-orange-900/30',
};

export const CloudIncidentsPage: React.FC = () => {
    if (!isCloudMode()) {
        return (
            <div className="p-6 max-w-7xl mx-auto">
                <div className="bg-blue-50 dark:bg-blue-900/20 text-blue-800 dark:text-blue-300 p-4 rounded-lg">
                    Cloud incidents only fire in cloud mode (the dispatcher worker is part of the
                    registry).
                </div>
            </div>
        );
    }
    return <CloudView />;
};

const CloudView: React.FC = () => {
    const orgId = useCloudOrgId();
    const queryClient = useQueryClient();
    const [statusFilter, setStatusFilter] = useState<IncidentStatus | 'all'>('open');
    const [selected, setSelected] = useState<Incident | null>(null);

    const incidentsQuery = useQuery({
        queryKey: ['cloud', 'incidents', orgId, statusFilter],
        queryFn: () =>
            cloudIncidentsApi.listForOrg(orgId!, {
                status: statusFilter === 'all' ? undefined : statusFilter,
                limit: 100,
            }),
        enabled: !!orgId,
        refetchInterval: 30_000,
    });

    const statsQuery = useQuery({
        queryKey: ['cloud', 'incidents', 'stats', orgId],
        queryFn: () => cloudIncidentsApi.getStats(orgId!),
        enabled: !!orgId,
        refetchInterval: 60_000,
    });

    const ackMutation = useMutation({
        mutationFn: (id: string) => cloudIncidentsApi.acknowledge(id),
        onSuccess: () => {
            queryClient.invalidateQueries({ queryKey: ['cloud', 'incidents'] });
        },
    });

    const resolveMutation = useMutation({
        mutationFn: (id: string) => cloudIncidentsApi.resolve(id),
        onSuccess: () => {
            queryClient.invalidateQueries({ queryKey: ['cloud', 'incidents'] });
        },
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

    const incidents = incidentsQuery.data ?? [];

    return (
        <div className="p-6 max-w-7xl mx-auto">
            <div className="flex justify-between items-start mb-6">
                <div>
                    <h1 className="text-2xl font-bold text-gray-900 dark:text-gray-100 mb-2">Incidents</h1>
                    <p className="text-gray-600 dark:text-gray-400">
                        Org-wide incidents from chaos / contract / observability / external monitors.
                    </p>
                </div>
                <button
                    onClick={() => incidentsQuery.refetch()}
                    className="flex items-center px-3 py-2 border border-gray-300 dark:border-gray-600 hover:bg-gray-50 dark:hover:bg-gray-700 rounded-lg text-sm"
                    disabled={incidentsQuery.isFetching}
                >
                    <RefreshCw
                        className={`w-4 h-4 mr-2 ${incidentsQuery.isFetching ? 'animate-spin' : ''}`}
                    />
                    Refresh
                </button>
            </div>

            {statsQuery.data && <StatsRow stats={statsQuery.data} />}

            <div className="mb-4 flex gap-2">
                {(['open', 'acknowledged', 'resolved', 'all'] as const).map((s) => (
                    <button
                        key={s}
                        onClick={() => setStatusFilter(s)}
                        className={`px-3 py-1.5 text-sm rounded-lg border ${
                            statusFilter === s
                                ? 'bg-blue-600 text-white border-blue-600'
                                : 'bg-white dark:bg-gray-800 border-gray-300 dark:border-gray-600 hover:bg-gray-50 dark:hover:bg-gray-700'
                        }`}
                    >
                        {s}
                    </button>
                ))}
            </div>

            {incidentsQuery.isError && (
                <div className="mb-4 bg-red-50 dark:bg-red-900/20 text-red-700 dark:text-red-400 p-4 rounded-lg text-sm">
                    {(incidentsQuery.error as Error).message}
                </div>
            )}

            {incidents.length === 0 && !incidentsQuery.isLoading ? (
                <div className="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-12 text-center">
                    <CheckCircle2 className="w-16 h-16 mx-auto text-green-400 mb-4" />
                    <h3 className="text-lg font-medium text-gray-900 dark:text-gray-100 mb-2">
                        Nothing to see here
                    </h3>
                    <p className="text-gray-500 dark:text-gray-400">
                        No {statusFilter === 'all' ? '' : statusFilter} incidents match your filters.
                    </p>
                </div>
            ) : (
                <div className="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 overflow-hidden">
                    <table className="w-full text-left text-sm">
                        <thead className="bg-gray-50 dark:bg-gray-900/50 border-b border-gray-200 dark:border-gray-700">
                            <tr>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">
                                    Title
                                </th>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">
                                    Severity
                                </th>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">
                                    Status
                                </th>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">
                                    Source
                                </th>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">
                                    Created
                                </th>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400 text-right">
                                    Actions
                                </th>
                            </tr>
                        </thead>
                        <tbody className="divide-y divide-gray-200 dark:divide-gray-700">
                            {incidents.map((i) => (
                                <IncidentRow
                                    key={i.id}
                                    incident={i}
                                    onAck={() => ackMutation.mutate(i.id)}
                                    onResolve={() => resolveMutation.mutate(i.id)}
                                    onView={() => setSelected(i)}
                                />
                            ))}
                        </tbody>
                    </table>
                </div>
            )}

            {selected && <DetailModal incident={selected} onClose={() => setSelected(null)} />}
        </div>
    );
};

const StatsRow: React.FC<{ stats: IncidentStats }> = ({ stats }) => {
    const formatMttr = (s: number | null) => {
        if (s === null) return '—';
        if (s < 60) return `${s.toFixed(0)}s`;
        if (s < 3600) return `${(s / 60).toFixed(1)}m`;
        return `${(s / 3600).toFixed(1)}h`;
    };
    return (
        <div className="grid grid-cols-1 md:grid-cols-4 gap-4 mb-6">
            <StatCard label="Open" value={stats.open.total} icon={AlertCircle} color="red" />
            <StatCard
                label="Resolved (30d)"
                value={stats.resolved_30d.total}
                icon={CheckCircle2}
                color="green"
            />
            <StatCard
                label="MTTR (30d)"
                value={formatMttr(stats.mttr_seconds_30d)}
                icon={Clock}
                color="blue"
            />
            <StatCard
                label="Notifications (24h)"
                value={stats.notification_attempts_24h}
                icon={Activity}
                color="purple"
            />
        </div>
    );
};

const StatCard: React.FC<{
    label: string;
    value: number | string;
    icon: React.FC<{ className?: string }>;
    color: 'red' | 'green' | 'blue' | 'purple';
}> = ({ label, value, icon: Icon, color }) => {
    const styles = {
        red: 'text-red-600 dark:text-red-400',
        green: 'text-green-600 dark:text-green-400',
        blue: 'text-blue-600 dark:text-blue-400',
        purple: 'text-purple-600 dark:text-purple-400',
    };
    return (
        <div className="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-4">
            <div className={`flex items-center gap-2 mb-2 ${styles[color]}`}>
                <Icon className="w-4 h-4" />
                <span className="text-xs font-medium uppercase tracking-wider">{label}</span>
            </div>
            <div className="text-2xl font-bold text-gray-900 dark:text-gray-100">{value}</div>
        </div>
    );
};

const IncidentRow: React.FC<{
    incident: Incident;
    onAck: () => void;
    onResolve: () => void;
    onView: () => void;
}> = ({ incident, onAck, onResolve, onView }) => (
    <tr
        className="hover:bg-gray-50 dark:hover:bg-gray-800/50 cursor-pointer"
        onClick={onView}
    >
        <td className="px-6 py-4">
            <div className="font-medium text-gray-900 dark:text-gray-100">{incident.title}</div>
            <div className="text-xs text-gray-500 font-mono mt-0.5">{incident.id.slice(0, 8)}</div>
        </td>
        <td className="px-6 py-4">
            <span
                className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium border ${SEVERITY_STYLES[incident.severity]}`}
            >
                {incident.severity}
            </span>
        </td>
        <td className="px-6 py-4">
            <span
                className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium border ${STATUS_STYLES[incident.status]}`}
            >
                {incident.status}
            </span>
        </td>
        <td className="px-6 py-4 text-gray-600 dark:text-gray-300 font-mono text-xs">{incident.source}</td>
        <td className="px-6 py-4 text-gray-600 dark:text-gray-300 text-xs">
            {new Date(incident.created_at).toLocaleString()}
        </td>
        <td
            className="px-6 py-4 text-right space-x-1"
            onClick={(e) => e.stopPropagation()}
        >
            {incident.status === 'open' && (
                <button
                    onClick={onAck}
                    className="px-2 py-1 text-xs rounded bg-yellow-100 text-yellow-800 hover:bg-yellow-200 dark:bg-yellow-900/30 dark:text-yellow-400 dark:hover:bg-yellow-900/50"
                >
                    Ack
                </button>
            )}
            {(incident.status === 'open' || incident.status === 'acknowledged') && (
                <button
                    onClick={onResolve}
                    className="px-2 py-1 text-xs rounded bg-green-100 text-green-800 hover:bg-green-200 dark:bg-green-900/30 dark:text-green-400 dark:hover:bg-green-900/50"
                >
                    Resolve
                </button>
            )}
            <ChevronRight className="w-4 h-4 inline text-gray-400" />
        </td>
    </tr>
);

const DetailModal: React.FC<{ incident: Incident; onClose: () => void }> = ({
    incident,
    onClose,
}) => {
    const eventsQuery = useQuery({
        queryKey: ['cloud', 'incidents', 'events', incident.id],
        queryFn: () => cloudIncidentsApi.listEvents(incident.id),
    });

    return (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm">
            <div className="bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-3xl w-full max-h-[80vh] overflow-y-auto border border-gray-200 dark:border-gray-700">
                <div className="p-6 border-b border-gray-200 dark:border-gray-700 sticky top-0 bg-white dark:bg-gray-800">
                    <div className="flex items-start justify-between">
                        <div>
                            <h2 className="text-xl font-semibold flex items-center gap-2">
                                <AlertTriangle className="w-5 h-5 text-red-500" />
                                {incident.title}
                            </h2>
                            <div className="mt-2 flex gap-2 text-xs">
                                <span
                                    className={`inline-flex items-center px-2.5 py-0.5 rounded-full font-medium border ${SEVERITY_STYLES[incident.severity]}`}
                                >
                                    {incident.severity}
                                </span>
                                <span
                                    className={`inline-flex items-center px-2.5 py-0.5 rounded-full font-medium border ${STATUS_STYLES[incident.status]}`}
                                >
                                    {incident.status}
                                </span>
                                <span className="text-gray-500 font-mono">{incident.source}</span>
                            </div>
                        </div>
                        <button onClick={onClose} className="text-gray-400 hover:text-gray-600">
                            ✕
                        </button>
                    </div>
                </div>
                <div className="p-6 space-y-4">
                    {incident.description && (
                        <div>
                            <h3 className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">Description</h3>
                            <p className="text-sm text-gray-600 dark:text-gray-400 whitespace-pre-wrap">
                                {incident.description}
                            </p>
                        </div>
                    )}
                    <div className="grid grid-cols-2 gap-4 text-xs">
                        <div>
                            <div className="text-gray-500 mb-1">Source ref</div>
                            <div className="font-mono">{incident.source_ref ?? '—'}</div>
                        </div>
                        <div>
                            <div className="text-gray-500 mb-1">Dedupe key</div>
                            <div className="font-mono break-all">{incident.dedupe_key}</div>
                        </div>
                        <div>
                            <div className="text-gray-500 mb-1">Created</div>
                            <div>{new Date(incident.created_at).toLocaleString()}</div>
                        </div>
                        <div>
                            <div className="text-gray-500 mb-1">Resolved</div>
                            <div>
                                {incident.resolved_at ? new Date(incident.resolved_at).toLocaleString() : '—'}
                            </div>
                        </div>
                    </div>
                    <div>
                        <h3 className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">Timeline</h3>
                        <div className="space-y-2 text-xs">
                            {(eventsQuery.data ?? []).map((e) => (
                                <div
                                    key={e.id}
                                    className="bg-gray-50 dark:bg-gray-900/50 rounded p-3 border border-gray-200 dark:border-gray-700"
                                >
                                    <div className="flex items-center justify-between mb-1">
                                        <span className="font-medium">{e.event_type}</span>
                                        <span className="text-gray-500">
                                            {new Date(e.created_at).toLocaleString()}
                                        </span>
                                    </div>
                                    {e.payload && (
                                        <pre className="text-xs text-gray-600 dark:text-gray-400 overflow-x-auto">
                                            {JSON.stringify(e.payload, null, 2)}
                                        </pre>
                                    )}
                                </div>
                            ))}
                            {eventsQuery.data?.length === 0 && (
                                <div className="text-gray-500 italic">No events yet.</div>
                            )}
                        </div>
                    </div>
                </div>
            </div>
        </div>
    );
};
