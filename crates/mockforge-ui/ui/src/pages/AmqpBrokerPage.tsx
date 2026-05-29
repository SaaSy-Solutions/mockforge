import React, { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/Card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Network, Inbox, Send, RefreshCw, AlertCircle, Trash2, Plus, Link } from 'lucide-react';
import { useToast } from '@/components/ui/ToastProvider';
import { authenticatedFetch } from '@/utils/apiClient';

interface AmqpBrokerStats {
  exchanges: number;
  queues: number;
  bindings: number;
  buffered_messages: number;
  connections_active: number;
  channels_active: number;
  messages_published_total: number;
  messages_consumed_total: number;
}

interface AmqpBindingInfo {
  queue: string;
  routing_key: string;
}

interface AmqpExchangeInfo {
  name: string;
  type: string;
  durable: boolean;
  auto_delete: boolean;
  bindings: AmqpBindingInfo[];
}

interface AmqpQueueInfo {
  name: string;
  durable: boolean;
  exclusive: boolean;
  auto_delete: boolean;
  message_count: number;
  consumer_count: number;
}

const API_BASE = '/__mockforge/api/amqp';
const EXCHANGE_TYPES = ['direct', 'fanout', 'topic', 'headers'] as const;

async function apiFetch<T>(url: string, init?: RequestInit): Promise<T> {
  const response = await authenticatedFetch(url, init);
  if (!response.ok) {
    const body = await response.json().catch(() => ({}));
    if (response.status === 503) {
      throw new Error('AMQP broker is not enabled or not available');
    }
    throw new Error(body.message || body.error || `Request failed (${response.status})`);
  }
  // Some endpoints return an empty/plain body on success.
  return response.json().catch(() => ({})) as Promise<T>;
}

const fetchStats = () => apiFetch<AmqpBrokerStats>(`${API_BASE}/stats`);
const fetchExchanges = () => apiFetch<{ exchanges: AmqpExchangeInfo[] }>(`${API_BASE}/exchanges`);
const fetchQueues = () => apiFetch<{ queues: AmqpQueueInfo[] }>(`${API_BASE}/queues`);

const jsonPost = (url: string, body: unknown) =>
  apiFetch<unknown>(url, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });

export function AmqpBrokerPage() {
  const { showToast } = useToast();
  const queryClient = useQueryClient();

  // Declare-exchange form
  const [exName, setExName] = useState('');
  const [exType, setExType] = useState<(typeof EXCHANGE_TYPES)[number]>('direct');
  const [exDurable, setExDurable] = useState(false);

  // Declare-queue form
  const [queueName, setQueueName] = useState('');
  const [queueDurable, setQueueDurable] = useState(false);

  // Add-binding form
  const [bindExchange, setBindExchange] = useState('');
  const [bindQueue, setBindQueue] = useState('');
  const [bindRoutingKey, setBindRoutingKey] = useState('');

  // Publish form
  const [pubExchange, setPubExchange] = useState('');
  const [pubRoutingKey, setPubRoutingKey] = useState('');
  const [pubPayload, setPubPayload] = useState('');

  const statsQuery = useQuery({ queryKey: ['amqp-stats'], queryFn: fetchStats, refetchInterval: 5000 });
  const exchangesQuery = useQuery({
    queryKey: ['amqp-exchanges'],
    queryFn: fetchExchanges,
    refetchInterval: 5000,
  });
  const queuesQuery = useQuery({
    queryKey: ['amqp-queues'],
    queryFn: fetchQueues,
    refetchInterval: 5000,
  });

  const invalidateAll = () => {
    queryClient.invalidateQueries({ queryKey: ['amqp-stats'] });
    queryClient.invalidateQueries({ queryKey: ['amqp-exchanges'] });
    queryClient.invalidateQueries({ queryKey: ['amqp-queues'] });
  };

  const declareExchangeMutation = useMutation({
    mutationFn: (body: { name: string; type: string; durable: boolean }) =>
      jsonPost(`${API_BASE}/exchanges`, body),
    onSuccess: () => {
      showToast('Exchange declared', 'success');
      setExName('');
      invalidateAll();
    },
    onError: (err: Error) => showToast(err.message, 'error'),
  });

  const deleteExchangeMutation = useMutation({
    mutationFn: (name: string) =>
      apiFetch<unknown>(`${API_BASE}/exchanges/${encodeURIComponent(name)}`, { method: 'DELETE' }),
    onSuccess: () => {
      showToast('Exchange deleted', 'success');
      invalidateAll();
    },
    onError: (err: Error) => showToast(err.message, 'error'),
  });

  const declareQueueMutation = useMutation({
    mutationFn: (body: { name: string; durable: boolean }) => jsonPost(`${API_BASE}/queues`, body),
    onSuccess: () => {
      showToast('Queue declared', 'success');
      setQueueName('');
      invalidateAll();
    },
    onError: (err: Error) => showToast(err.message, 'error'),
  });

  const addBindingMutation = useMutation({
    mutationFn: (body: { exchange: string; queue: string; routing_key: string }) =>
      jsonPost(`${API_BASE}/exchanges/${encodeURIComponent(body.exchange)}/bindings`, {
        queue: body.queue,
        routing_key: body.routing_key,
      }),
    onSuccess: () => {
      showToast('Binding added', 'success');
      setBindQueue('');
      setBindRoutingKey('');
      invalidateAll();
    },
    onError: (err: Error) => showToast(err.message, 'error'),
  });

  const publishMutation = useMutation({
    mutationFn: (body: { exchange: string; routing_key: string; payload: string }) =>
      jsonPost(`${API_BASE}/publish`, body),
    onSuccess: (data) => {
      const queued = (data as { queued_to?: string[] })?.queued_to ?? [];
      showToast(
        queued.length > 0 ? `Published → ${queued.join(', ')}` : 'Published (no matching queue)',
        queued.length > 0 ? 'success' : 'info'
      );
      setPubPayload('');
      invalidateAll();
    },
    onError: (err: Error) => showToast(err.message, 'error'),
  });

  const handleDeclareExchange = (e: React.FormEvent) => {
    e.preventDefault();
    if (!exName.trim()) {
      showToast('Exchange name is required', 'error');
      return;
    }
    declareExchangeMutation.mutate({ name: exName.trim(), type: exType, durable: exDurable });
  };

  const handleDeclareQueue = (e: React.FormEvent) => {
    e.preventDefault();
    if (!queueName.trim()) {
      showToast('Queue name is required', 'error');
      return;
    }
    declareQueueMutation.mutate({ name: queueName.trim(), durable: queueDurable });
  };

  const handleAddBinding = (e: React.FormEvent) => {
    e.preventDefault();
    if (!bindExchange.trim() || !bindQueue.trim()) {
      showToast('Exchange and queue are required', 'error');
      return;
    }
    addBindingMutation.mutate({
      exchange: bindExchange.trim(),
      queue: bindQueue.trim(),
      routing_key: bindRoutingKey.trim(),
    });
  };

  const handlePublish = (e: React.FormEvent) => {
    e.preventDefault();
    if (!pubRoutingKey.trim()) {
      showToast('Routing key is required', 'error');
      return;
    }
    publishMutation.mutate({
      exchange: pubExchange.trim(),
      routing_key: pubRoutingKey.trim(),
      payload: pubPayload,
    });
  };

  const error = statsQuery.error || exchangesQuery.error || queuesQuery.error;
  const exchanges = exchangesQuery.data?.exchanges ?? [];
  const queues = queuesQuery.data?.queues ?? [];

  return (
    <div className="container mx-auto px-4 py-6 max-w-7xl">
      <div className="mb-6 flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold flex items-center gap-2">
            <Network className="h-6 w-6" />
            AMQP Broker
          </h1>
          <p className="text-sm text-muted-foreground">
            Inspect exchanges and queues, manage bindings, and publish test messages.
          </p>
        </div>
        <Button
          variant="outline"
          size="sm"
          onClick={() => {
            statsQuery.refetch();
            exchangesQuery.refetch();
            queuesQuery.refetch();
          }}
        >
          <RefreshCw className="h-4 w-4 mr-1" />
          Refresh
        </Button>
      </div>

      {error && (
        <Card className="mb-4 border-danger-200 bg-danger-50 dark:bg-danger-900/10 dark:border-danger-800">
          <CardContent className="pt-4 flex items-start gap-2">
            <AlertCircle className="h-5 w-5 text-danger-600 flex-shrink-0 mt-0.5" />
            <p className="text-sm text-danger-700 dark:text-danger-200">
              {error instanceof Error ? error.message : 'Failed to load AMQP broker data'}
            </p>
          </CardContent>
        </Card>
      )}

      {/* Stats */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-6">
        <StatCard label="Exchanges" value={statsQuery.data?.exchanges ?? '—'} />
        <StatCard label="Queues" value={statsQuery.data?.queues ?? '—'} />
        <StatCard label="Bindings" value={statsQuery.data?.bindings ?? '—'} />
        <StatCard label="Buffered messages" value={statsQuery.data?.buffered_messages ?? '—'} />
        <StatCard label="Active connections" value={statsQuery.data?.connections_active ?? '—'} />
        <StatCard label="Active channels" value={statsQuery.data?.channels_active ?? '—'} />
        <StatCard label="Published" value={statsQuery.data?.messages_published_total ?? '—'} />
        <StatCard label="Consumed" value={statsQuery.data?.messages_consumed_total ?? '—'} />
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4 mb-6">
        {/* Exchanges */}
        <Card>
          <CardHeader>
            <CardTitle className="text-base flex items-center gap-2">
              <Network className="h-4 w-4" />
              Exchanges ({exchanges.length})
            </CardTitle>
          </CardHeader>
          <CardContent>
            {exchanges.length === 0 ? (
              <p className="text-muted-foreground text-sm text-center py-6">No exchanges declared</p>
            ) : (
              <div className="space-y-2 max-h-[300px] overflow-y-auto">
                {exchanges.map((ex) => (
                  <div key={ex.name} className="p-2 border rounded text-sm">
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-2">
                        <code className="font-mono text-xs">{ex.name || '(default)'}</code>
                        <span className="text-xs px-1.5 py-0.5 rounded bg-muted text-muted-foreground">
                          {ex.type}
                        </span>
                        {ex.durable && (
                          <span className="text-xs text-muted-foreground">durable</span>
                        )}
                      </div>
                      {ex.name && (
                        <Button
                          size="sm"
                          variant="ghost"
                          disabled={deleteExchangeMutation.isPending}
                          onClick={() => deleteExchangeMutation.mutate(ex.name)}
                        >
                          <Trash2 className="h-3.5 w-3.5" />
                        </Button>
                      )}
                    </div>
                    {ex.bindings.length > 0 && (
                      <ul className="mt-1 pl-4 space-y-0.5">
                        {ex.bindings.map((b, i) => (
                          <li key={`${b.queue}-${b.routing_key}-${i}`} className="text-xs text-muted-foreground">
                            → <code className="font-mono">{b.queue}</code>
                            {b.routing_key && <> (key: <code className="font-mono">{b.routing_key}</code>)</>}
                          </li>
                        ))}
                      </ul>
                    )}
                  </div>
                ))}
              </div>
            )}

            <form onSubmit={handleDeclareExchange} className="mt-4 space-y-2 border-t pt-3">
              <Label>Declare exchange</Label>
              <Input value={exName} onChange={(e) => setExName(e.target.value)} placeholder="orders" />
              <div className="flex gap-3 items-end">
                <select
                  value={exType}
                  onChange={(e) => setExType(e.target.value as (typeof EXCHANGE_TYPES)[number])}
                  className="block border rounded px-2 py-1.5 text-sm bg-background"
                >
                  {EXCHANGE_TYPES.map((t) => (
                    <option key={t} value={t}>
                      {t}
                    </option>
                  ))}
                </select>
                <label className="flex items-center gap-2 text-sm">
                  <input type="checkbox" checked={exDurable} onChange={(e) => setExDurable(e.target.checked)} />
                  Durable
                </label>
                <Button type="submit" size="sm" disabled={declareExchangeMutation.isPending} className="ml-auto">
                  <Plus className="h-3.5 w-3.5 mr-1" />
                  Declare
                </Button>
              </div>
            </form>
          </CardContent>
        </Card>

        {/* Queues */}
        <Card>
          <CardHeader>
            <CardTitle className="text-base flex items-center gap-2">
              <Inbox className="h-4 w-4" />
              Queues ({queues.length})
            </CardTitle>
          </CardHeader>
          <CardContent>
            {queues.length === 0 ? (
              <p className="text-muted-foreground text-sm text-center py-6">No queues declared</p>
            ) : (
              <div className="space-y-2 max-h-[300px] overflow-y-auto">
                {queues.map((q) => (
                  <div key={q.name} className="flex items-center justify-between p-2 border rounded text-sm">
                    <div className="flex items-center gap-2">
                      <code className="font-mono text-xs">{q.name}</code>
                      {q.durable && <span className="text-xs text-muted-foreground">durable</span>}
                    </div>
                    <div className="flex items-center gap-3 text-xs text-muted-foreground">
                      <span>{q.message_count} msg</span>
                      <span>{q.consumer_count} cons</span>
                    </div>
                  </div>
                ))}
              </div>
            )}

            <form onSubmit={handleDeclareQueue} className="mt-4 space-y-2 border-t pt-3">
              <Label>Declare queue</Label>
              <Input
                value={queueName}
                onChange={(e) => setQueueName(e.target.value)}
                placeholder="orders.processing"
              />
              <div className="flex gap-3 items-center">
                <label className="flex items-center gap-2 text-sm">
                  <input
                    type="checkbox"
                    checked={queueDurable}
                    onChange={(e) => setQueueDurable(e.target.checked)}
                  />
                  Durable
                </label>
                <Button type="submit" size="sm" disabled={declareQueueMutation.isPending} className="ml-auto">
                  <Plus className="h-3.5 w-3.5 mr-1" />
                  Declare
                </Button>
              </div>
            </form>
          </CardContent>
        </Card>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
        {/* Add binding */}
        <Card>
          <CardHeader>
            <CardTitle className="text-base flex items-center gap-2">
              <Link className="h-4 w-4" />
              Add binding
            </CardTitle>
            <CardDescription>Bind a queue to an exchange with a routing key.</CardDescription>
          </CardHeader>
          <CardContent>
            <form onSubmit={handleAddBinding} className="space-y-3">
              <div>
                <Label>Exchange</Label>
                <Input
                  value={bindExchange}
                  onChange={(e) => setBindExchange(e.target.value)}
                  placeholder="orders"
                />
              </div>
              <div>
                <Label>Queue</Label>
                <Input
                  value={bindQueue}
                  onChange={(e) => setBindQueue(e.target.value)}
                  placeholder="orders.processing"
                />
              </div>
              <div>
                <Label>Routing key</Label>
                <Input
                  value={bindRoutingKey}
                  onChange={(e) => setBindRoutingKey(e.target.value)}
                  placeholder="order.created"
                />
              </div>
              <Button type="submit" disabled={addBindingMutation.isPending} className="w-full">
                {addBindingMutation.isPending ? 'Adding…' : 'Add binding'}
              </Button>
            </form>
          </CardContent>
        </Card>

        {/* Publish */}
        <Card>
          <CardHeader>
            <CardTitle className="text-base flex items-center gap-2">
              <Send className="h-4 w-4" />
              Publish message
            </CardTitle>
            <CardDescription>
              Route a message through an exchange (leave blank for the default exchange).
            </CardDescription>
          </CardHeader>
          <CardContent>
            <form onSubmit={handlePublish} className="space-y-3">
              <div>
                <Label>Exchange</Label>
                <Input
                  value={pubExchange}
                  onChange={(e) => setPubExchange(e.target.value)}
                  placeholder="(default exchange)"
                />
              </div>
              <div>
                <Label>Routing key</Label>
                <Input
                  value={pubRoutingKey}
                  onChange={(e) => setPubRoutingKey(e.target.value)}
                  placeholder="order.created"
                />
              </div>
              <div>
                <Label>Payload</Label>
                <textarea
                  value={pubPayload}
                  onChange={(e) => setPubPayload(e.target.value)}
                  rows={3}
                  className="w-full px-3 py-2 border border-border rounded-md bg-card font-mono text-sm"
                  placeholder='{"orderId": 123}'
                />
              </div>
              <Button type="submit" disabled={publishMutation.isPending} className="w-full">
                {publishMutation.isPending ? 'Publishing…' : 'Publish'}
              </Button>
            </form>
          </CardContent>
        </Card>
      </div>
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
