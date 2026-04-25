import React, { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/Card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Radio, Send, Users, Hash, RefreshCw, AlertCircle, X } from 'lucide-react';
import { useToast } from '@/components/ui/ToastProvider';
import { authenticatedFetch } from '@/utils/apiClient';

interface MqttBrokerStats {
  connected_clients: number;
  active_topics: number;
  retained_messages: number;
  total_subscriptions: number;
}

interface MqttClient {
  client_id: string;
  connected_at?: string;
  subscriptions?: string[];
}

const API_BASE = '/__mockforge/api/mqtt';

async function apiFetch<T>(url: string, init?: RequestInit): Promise<T> {
  const response = await authenticatedFetch(url, init);
  if (!response.ok) {
    const body = await response.json().catch(() => ({}));
    if (response.status === 503) {
      throw new Error('MQTT broker is not enabled or not available');
    }
    throw new Error(body.message || body.error || `Request failed (${response.status})`);
  }
  return response.json() as Promise<T>;
}

const fetchStats = () => apiFetch<MqttBrokerStats>(`${API_BASE}/stats`);
const fetchClients = () => apiFetch<{ clients: (string | MqttClient)[] }>(`${API_BASE}/clients`);
const fetchTopics = () => apiFetch<{ topics: string[] }>(`${API_BASE}/topics`);
const disconnectClient = (clientId: string) =>
  apiFetch<unknown>(`${API_BASE}/clients/${encodeURIComponent(clientId)}`, { method: 'DELETE' });
const publishMessage = (body: { topic: string; payload: string; qos: number; retain: boolean }) =>
  apiFetch<unknown>(`${API_BASE}/publish`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });

export function MqttBrokerPage() {
  const { showToast } = useToast();
  const queryClient = useQueryClient();

  const [topic, setTopic] = useState('');
  const [payload, setPayload] = useState('');
  const [qos, setQos] = useState(0);
  const [retain, setRetain] = useState(false);

  const statsQuery = useQuery({
    queryKey: ['mqtt-stats'],
    queryFn: fetchStats,
    refetchInterval: 5000,
  });

  const clientsQuery = useQuery({
    queryKey: ['mqtt-clients'],
    queryFn: fetchClients,
    refetchInterval: 5000,
  });

  const topicsQuery = useQuery({
    queryKey: ['mqtt-topics'],
    queryFn: fetchTopics,
    refetchInterval: 5000,
  });

  const disconnectMutation = useMutation({
    mutationFn: disconnectClient,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['mqtt-clients'] });
      queryClient.invalidateQueries({ queryKey: ['mqtt-stats'] });
      showToast('Client disconnected', 'success');
    },
    onError: (err: Error) => showToast(err.message, 'error'),
  });

  const publishMutation = useMutation({
    mutationFn: publishMessage,
    onSuccess: () => {
      showToast('Message published', 'success');
      setPayload('');
      queryClient.invalidateQueries({ queryKey: ['mqtt-topics'] });
    },
    onError: (err: Error) => showToast(err.message, 'error'),
  });

  const handlePublish = (e: React.FormEvent) => {
    e.preventDefault();
    if (!topic.trim() || !payload.trim()) {
      showToast('Topic and payload are required', 'error');
      return;
    }
    publishMutation.mutate({ topic: topic.trim(), payload, qos, retain });
  };

  const error = statsQuery.error || clientsQuery.error || topicsQuery.error;
  const clients = clientsQuery.data?.clients ?? [];
  const topics = topicsQuery.data?.topics ?? [];

  const getClientId = (c: string | MqttClient): string =>
    typeof c === 'string' ? c : c.client_id;

  return (
    <div className="container mx-auto px-4 py-6 max-w-7xl">
      <div className="mb-6 flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold flex items-center gap-2">
            <Radio className="h-6 w-6" />
            MQTT Broker
          </h1>
          <p className="text-sm text-muted-foreground">
            Monitor clients, topics, and publish test messages.
          </p>
        </div>
        <Button
          variant="outline"
          size="sm"
          onClick={() => {
            statsQuery.refetch();
            clientsQuery.refetch();
            topicsQuery.refetch();
          }}
        >
          <RefreshCw className="h-4 w-4 mr-1" />
          Refresh
        </Button>
      </div>

      {error && (
        <Card className="mb-4 border-red-200 bg-red-50 dark:bg-red-900/10 dark:border-red-800">
          <CardContent className="pt-4 flex items-start gap-2">
            <AlertCircle className="h-5 w-5 text-red-600 flex-shrink-0 mt-0.5" />
            <p className="text-sm text-red-800 dark:text-red-200">
              {error instanceof Error ? error.message : 'Failed to load MQTT broker data'}
            </p>
          </CardContent>
        </Card>
      )}

      {/* Stats */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-6">
        <StatCard label="Connected clients" value={statsQuery.data?.connected_clients ?? '—'} />
        <StatCard label="Active topics" value={statsQuery.data?.active_topics ?? '—'} />
        <StatCard label="Retained messages" value={statsQuery.data?.retained_messages ?? '—'} />
        <StatCard label="Subscriptions" value={statsQuery.data?.total_subscriptions ?? '—'} />
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4 mb-6">
        <Card>
          <CardHeader>
            <CardTitle className="text-base flex items-center gap-2">
              <Users className="h-4 w-4" />
              Connected clients ({clients.length})
            </CardTitle>
          </CardHeader>
          <CardContent>
            {clients.length === 0 ? (
              <p className="text-muted-foreground text-sm text-center py-6">No connected clients</p>
            ) : (
              <div className="space-y-2 max-h-[300px] overflow-y-auto">
                {clients.map((c) => {
                  const clientId = getClientId(c);
                  return (
                    <div
                      key={clientId}
                      className="flex items-center justify-between p-2 border rounded text-sm"
                    >
                      <code className="font-mono text-xs">{clientId}</code>
                      <Button
                        size="sm"
                        variant="ghost"
                        disabled={disconnectMutation.isPending}
                        onClick={() => disconnectMutation.mutate(clientId)}
                      >
                        <X className="h-3.5 w-3.5" />
                      </Button>
                    </div>
                  );
                })}
              </div>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="text-base flex items-center gap-2">
              <Hash className="h-4 w-4" />
              Active topics ({topics.length})
            </CardTitle>
          </CardHeader>
          <CardContent>
            {topics.length === 0 ? (
              <p className="text-muted-foreground text-sm text-center py-6">No active topics</p>
            ) : (
              <div className="space-y-1 max-h-[300px] overflow-y-auto">
                {topics.map((t) => (
                  <div key={t} className="p-2 border rounded text-sm flex justify-between">
                    <code className="font-mono text-xs">{t}</code>
                    <Button size="sm" variant="ghost" onClick={() => setTopic(t)}>
                      Use
                    </Button>
                  </div>
                ))}
              </div>
            )}
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="text-base flex items-center gap-2">
            <Send className="h-4 w-4" />
            Publish message
          </CardTitle>
          <CardDescription>Send a message to an MQTT topic.</CardDescription>
        </CardHeader>
        <CardContent>
          <form onSubmit={handlePublish} className="space-y-3">
            <div>
              <Label>Topic</Label>
              <Input
                value={topic}
                onChange={(e) => setTopic(e.target.value)}
                placeholder="sensors/temperature/room1"
              />
            </div>
            <div>
              <Label>Payload</Label>
              <textarea
                value={payload}
                onChange={(e) => setPayload(e.target.value)}
                rows={4}
                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-700 rounded-md bg-white dark:bg-gray-800 font-mono text-sm"
                placeholder='{"temp": 22.5}'
              />
            </div>
            <div className="flex gap-4 items-end">
              <div>
                <Label>QoS</Label>
                <select
                  value={qos}
                  onChange={(e) => setQos(Number(e.target.value))}
                  className="block border rounded px-2 py-1.5 text-sm bg-background"
                >
                  <option value={0}>0 — At most once</option>
                  <option value={1}>1 — At least once</option>
                  <option value={2}>2 — Exactly once</option>
                </select>
              </div>
              <label className="flex items-center gap-2 text-sm">
                <input
                  type="checkbox"
                  checked={retain}
                  onChange={(e) => setRetain(e.target.checked)}
                />
                Retain
              </label>
              <Button type="submit" disabled={publishMutation.isPending} className="ml-auto">
                {publishMutation.isPending ? 'Publishing…' : 'Publish'}
              </Button>
            </div>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}

function StatCard({ label, value }: { label: string; value: number | string }) {
  return (
    <Card>
      <CardContent className="pt-4">
        <div className="text-xs text-muted-foreground">{label}</div>
        <div className="text-2xl font-bold">{value}</div>
      </CardContent>
    </Card>
  );
}
