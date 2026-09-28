/**
 * Notification Channels — cloud-only page for configuring incident
 * dispatch destinations (#3).
 *
 * Channels are org-scoped. Each channel is a webhook / Slack /
 * email / pagerduty target the dispatcher worker fires for incidents
 * the routing rules send its way. The test-fire button posts a synthetic
 * incident through the same path so the operator can validate the
 * channel before a real incident depends on it.
 */
import React, { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
    Plus,
    RefreshCw,
    Trash2,
    Mail,
    MessageSquare,
    Webhook as WebhookIcon,
    Bell,
    Power,
    PowerOff,
    Send,
} from 'lucide-react';
import { isCloudMode } from '../utils/cloudMode';
import { useCloudOrgId } from '../hooks/useCloudOrgId';
import {
    cloudNotificationsApi,
    type NotificationChannel,
    type NotificationChannelKind,
    type TestFireResult,
} from '../services/api/cloudNotifications';

const KIND_META: Record<NotificationChannelKind, { label: string; Icon: React.FC<{ className?: string }> }> = {
    email: { label: 'Email', Icon: Mail },
    slack: { label: 'Slack', Icon: MessageSquare },
    pagerduty: { label: 'PagerDuty', Icon: Bell },
    webhook: { label: 'Webhook', Icon: WebhookIcon },
};

// Each channel kind stores its destination under a different config key
// (see incident_dispatcher.rs): webhook/slack `url`, email `to`,
// pagerduty `routing_key`. The form collects one "target" and maps it.
const TARGET_FIELD: Record<
    NotificationChannelKind,
    { label: string; placeholder: string; help: string; inputType: string; mono: boolean }
> = {
    webhook: {
        label: 'Webhook URL',
        placeholder: 'https://example.com/webhook',
        help: 'We POST a JSON payload to this URL when an incident opens or resolves.',
        inputType: 'url',
        mono: true,
    },
    slack: {
        label: 'Slack incoming-webhook URL',
        placeholder: 'https://hooks.slack.com/services/...',
        help: "Paste the incoming-webhook URL from the Slack app's setup page.",
        inputType: 'url',
        mono: true,
    },
    email: {
        label: 'Recipients',
        placeholder: 'oncall@example.com, team@example.com',
        help: 'One or more email addresses, separated by commas.',
        inputType: 'text',
        mono: false,
    },
    pagerduty: {
        label: 'PagerDuty integration key',
        placeholder: '32-character Events API v2 routing key',
        help: 'Find it under Service > Integrations > Events API v2 in PagerDuty.',
        inputType: 'text',
        mono: true,
    },
};

function buildChannelConfig(kind: NotificationChannelKind, target: string): Record<string, unknown> {
    const value = target.trim();
    switch (kind) {
        case 'email':
            return {
                to: value
                    .split(',')
                    .map((addr) => addr.trim())
                    .filter(Boolean),
            };
        case 'pagerduty':
            return { routing_key: value };
        default:
            return { url: value };
    }
}

function describeChannelTarget(channel: NotificationChannel): string {
    const config = channel.config as Record<string, unknown>;
    if (channel.kind === 'email') {
        const to = config.to;
        if (Array.isArray(to)) return to.join(', ') || '(not set)';
        return typeof to === 'string' && to ? to : '(not set)';
    }
    if (channel.kind === 'pagerduty') {
        const key = config.routing_key ?? config.integration_key;
        return typeof key === 'string' && key ? `routing key ending ${key.slice(-4)}` : '(not set)';
    }
    return typeof config.url === 'string' && config.url ? config.url : '(not set)';
}

export const NotificationChannelsPage: React.FC = () => {
    if (!isCloudMode()) {
        return (
            <div className="p-6 max-w-7xl mx-auto">
                <div className="bg-blue-50 dark:bg-blue-900/20 text-blue-800 dark:text-blue-300 p-4 rounded-lg">
                    Notification channels live in cloud mode only — local mock servers don't dispatch incidents.
                </div>
            </div>
        );
    }
    return <CloudView />;
};

const CloudView: React.FC = () => {
    const orgId = useCloudOrgId();
    const queryClient = useQueryClient();
    const [showCreate, setShowCreate] = useState(false);
    const [draft, setDraft] = useState<{
        name: string;
        kind: NotificationChannelKind;
        target: string;
    }>({ name: '', kind: 'webhook', target: '' });
    const [fireResult, setFireResult] = useState<{ id: string; result: TestFireResult } | null>(null);

    const channelsQuery = useQuery({
        queryKey: ['cloud', 'notification-channels', orgId],
        queryFn: () => cloudNotificationsApi.listChannels(orgId!),
        enabled: !!orgId,
    });

    const createMutation = useMutation({
        mutationFn: () =>
            cloudNotificationsApi.createChannel(orgId!, {
                name: draft.name,
                kind: draft.kind,
                config: buildChannelConfig(draft.kind, draft.target),
                enabled: true,
            }),
        onSuccess: () => {
            setShowCreate(false);
            setDraft({ name: '', kind: 'webhook', target: '' });
            queryClient.invalidateQueries({ queryKey: ['cloud', 'notification-channels', orgId] });
        },
    });

    const deleteMutation = useMutation({
        mutationFn: (id: string) => cloudNotificationsApi.deleteChannel(orgId!, id),
        onSuccess: () =>
            queryClient.invalidateQueries({ queryKey: ['cloud', 'notification-channels', orgId] }),
    });

    const toggleMutation = useMutation({
        mutationFn: ({ id, enabled }: { id: string; enabled: boolean }) =>
            cloudNotificationsApi.updateChannel(orgId!, id, { enabled }),
        onSuccess: () =>
            queryClient.invalidateQueries({ queryKey: ['cloud', 'notification-channels', orgId] }),
    });

    const fireMutation = useMutation({
        mutationFn: (id: string) => cloudNotificationsApi.testFire(orgId!, id),
        onSuccess: (result, id) => setFireResult({ id, result }),
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

    const channels = channelsQuery.data ?? [];

    return (
        <div className="p-6 max-w-7xl mx-auto">
            <div className="flex justify-between items-start mb-8">
                <div>
                    <h1 className="text-2xl font-bold text-gray-900 dark:text-gray-100 mb-2">
                        Notification Channels
                    </h1>
                    <p className="text-gray-600 dark:text-gray-400">
                        Configure where incidents fan out. Add a webhook URL or Slack incoming-webhook to start
                        receiving alerts. Email + PagerDuty channels are accepted but currently record skipped
                        attempts until those providers land.
                    </p>
                </div>
                <div className="flex gap-2">
                    <button
                        onClick={() => channelsQuery.refetch()}
                        className="flex items-center px-3 py-2 border border-gray-300 dark:border-gray-600 hover:bg-gray-50 dark:hover:bg-gray-700 rounded-lg text-sm"
                        disabled={channelsQuery.isFetching}
                    >
                        <RefreshCw className={`w-4 h-4 mr-2 ${channelsQuery.isFetching ? 'animate-spin' : ''}`} />
                        Refresh
                    </button>
                    <button
                        onClick={() => setShowCreate(true)}
                        className="flex items-center px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg font-medium"
                    >
                        <Plus className="w-4 h-4 mr-2" />
                        Add Channel
                    </button>
                </div>
            </div>

            {channelsQuery.isError && (
                <div className="mb-4 bg-red-50 dark:bg-red-900/20 text-red-700 dark:text-red-400 p-4 rounded-lg text-sm">
                    {(channelsQuery.error as Error).message}
                </div>
            )}

            {fireResult && (
                <div
                    className={`mb-4 p-4 rounded-lg text-sm whitespace-pre-wrap ${
                        fireResult.result.ok
                            ? 'bg-green-50 dark:bg-green-900/20 text-green-700 dark:text-green-400'
                            : 'bg-red-50 dark:bg-red-900/20 text-red-700 dark:text-red-400'
                    }`}
                >
                    <div className="flex items-start justify-between gap-3">
                        <div>
                            <div className="font-medium">
                                Test fire {fireResult.result.ok ? 'succeeded' : fireResult.result.skipped ? 'skipped' : 'failed'}
                            </div>
                            <div className="text-xs mt-1 font-mono">
                                {JSON.stringify(fireResult.result, null, 2)}
                            </div>
                        </div>
                        <button onClick={() => setFireResult(null)} className="text-xs underline">
                            dismiss
                        </button>
                    </div>
                </div>
            )}

            {channels.length === 0 && !channelsQuery.isLoading ? (
                <div className="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-12 text-center">
                    <Bell className="w-16 h-16 mx-auto text-gray-400 mb-4" />
                    <h3 className="text-lg font-medium text-gray-900 dark:text-gray-100 mb-2">No channels configured</h3>
                    <p className="text-gray-500 dark:text-gray-400 mb-6">
                        Without a channel the dispatcher has nowhere to send incidents.
                    </p>
                    <button
                        onClick={() => setShowCreate(true)}
                        className="px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700"
                    >
                        Add First Channel
                    </button>
                </div>
            ) : (
                <div className="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 overflow-hidden">
                    <table className="w-full text-left text-sm">
                        <thead className="bg-gray-50 dark:bg-gray-900/50 border-b border-gray-200 dark:border-gray-700">
                            <tr>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Name</th>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Kind</th>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Target</th>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Status</th>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400 text-right">Actions</th>
                            </tr>
                        </thead>
                        <tbody className="divide-y divide-gray-200 dark:divide-gray-700">
                            {channels.map((c) => (
                                <ChannelRow
                                    key={c.id}
                                    channel={c}
                                    onToggle={() => toggleMutation.mutate({ id: c.id, enabled: !c.enabled })}
                                    onDelete={() => {
                                        if (confirm(`Delete channel "${c.name}"?`)) deleteMutation.mutate(c.id);
                                    }}
                                    onTestFire={() => fireMutation.mutate(c.id)}
                                    fireDisabled={fireMutation.isPending}
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
        </div>
    );
};

const ChannelRow: React.FC<{
    channel: NotificationChannel;
    onToggle: () => void;
    onDelete: () => void;
    onTestFire: () => void;
    fireDisabled: boolean;
}> = ({ channel, onToggle, onDelete, onTestFire, fireDisabled }) => {
    const meta = KIND_META[channel.kind] ?? { label: channel.kind, Icon: WebhookIcon };
    const Icon = meta.Icon;
    const target = describeChannelTarget(channel);
    return (
        <tr className="hover:bg-gray-50 dark:hover:bg-gray-800/50">
            <td className="px-6 py-4 font-medium text-gray-900 dark:text-gray-100">{channel.name}</td>
            <td className="px-6 py-4">
                <span className="inline-flex items-center text-gray-700 dark:text-gray-300">
                    <Icon className="w-4 h-4 mr-2" />
                    {meta.label}
                </span>
            </td>
            <td className="px-6 py-4 font-mono text-gray-600 dark:text-gray-300 truncate max-w-[280px]" title={target}>{target}</td>
            <td className="px-6 py-4">
                <span
                    className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium border ${
                        channel.enabled
                            ? 'bg-green-50 text-green-700 border-green-200 dark:bg-green-900/20 dark:text-green-400 dark:border-green-900/30'
                            : 'bg-gray-100 text-gray-700 border-gray-200 dark:bg-gray-800 dark:text-gray-400 dark:border-gray-700'
                    }`}
                >
                    {channel.enabled ? 'enabled' : 'disabled'}
                </span>
            </td>
            <td className="px-6 py-4 text-right space-x-1">
                <button
                    onClick={onTestFire}
                    disabled={!channel.enabled || fireDisabled}
                    className="p-2 text-blue-600 hover:bg-blue-50 dark:hover:bg-blue-900/20 rounded-lg disabled:opacity-50"
                    title="Test fire"
                >
                    <Send className="w-4 h-4" />
                </button>
                <button
                    onClick={onToggle}
                    className="p-2 text-gray-600 hover:bg-gray-50 dark:hover:bg-gray-700 rounded-lg"
                    title={channel.enabled ? 'Disable' : 'Enable'}
                >
                    {channel.enabled ? <PowerOff className="w-4 h-4" /> : <Power className="w-4 h-4" />}
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
    state: { name: string; kind: NotificationChannelKind; target: string };
    setState: (s: { name: string; kind: NotificationChannelKind; target: string }) => void;
    onClose: () => void;
    onSubmit: () => void;
    submitting: boolean;
    error: string | null;
}> = ({ state, setState, onClose, onSubmit, submitting, error }) => (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm">
        <div className="bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-md w-full border border-gray-200 dark:border-gray-700">
            <div className="p-6 border-b border-gray-200 dark:border-gray-700">
                <h2 className="text-xl font-semibold text-gray-900 dark:text-gray-100">Add Notification Channel</h2>
            </div>
            <div className="p-6 space-y-4">
                {error && (
                    <div className="bg-red-50 dark:bg-red-900/20 text-red-700 dark:text-red-400 p-3 rounded text-sm">
                        {error}
                    </div>
                )}
                <div className="space-y-2">
                    <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">Name</label>
                    <input
                        type="text"
                        value={state.name}
                        onChange={(e) => setState({ ...state, name: e.target.value })}
                        placeholder="e.g., #incidents Slack"
                        className="w-full px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-lg outline-none focus:ring-2 focus:ring-blue-500"
                    />
                </div>
                <div className="space-y-2">
                    <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">Kind</label>
                    <select
                        value={state.kind}
                        onChange={(e) => setState({ ...state, kind: e.target.value as NotificationChannelKind })}
                        className="w-full px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-lg outline-none focus:ring-2 focus:ring-blue-500"
                    >
                        <option value="webhook">Webhook</option>
                        <option value="slack">Slack (incoming-webhook)</option>
                        <option value="email">Email</option>
                        <option value="pagerduty">PagerDuty</option>
                    </select>
                </div>
                <div className="space-y-2">
                    <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">
                        {TARGET_FIELD[state.kind].label}
                    </label>
                    <input
                        type={TARGET_FIELD[state.kind].inputType}
                        value={state.target}
                        onChange={(e) => setState({ ...state, target: e.target.value })}
                        placeholder={TARGET_FIELD[state.kind].placeholder}
                        className={`w-full px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-lg outline-none focus:ring-2 focus:ring-blue-500 ${
                            TARGET_FIELD[state.kind].mono ? 'font-mono text-xs' : 'text-sm'
                        }`}
                    />
                    <p className="text-xs text-gray-500">{TARGET_FIELD[state.kind].help}</p>
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
                    disabled={!state.name || !state.target.trim() || submitting}
                    className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg font-medium disabled:opacity-50 disabled:cursor-not-allowed"
                >
                    {submitting ? 'Creating…' : 'Create'}
                </button>
            </div>
        </div>
    </div>
);
