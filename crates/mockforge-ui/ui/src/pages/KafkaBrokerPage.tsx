import React, { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/Card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Database, Send, Layers, Users, RefreshCw, AlertCircle } from 'lucide-react';
import { useToast } from '@/components/ui/ToastProvider';
import { authenticatedFetch } from '@/utils/apiClient';

interface KafkaStats {
  topics: number;
  partitions: number;
  consumer_groups: number;
  messages_produced: number;
  messages_consumed: number;
}

interface KafkaTopicInfo {
  name: string;
  partitions: number;
  replication_factor: number;
}

interface KafkaConsumerGroupInfo {
  group_id: string;
  members: number;
  state: string;
}

const API_BASE = '/__mockforge/api/kafka';

async function apiFetch<T>(url: string, init?: RequestInit): Promise<T> {
  const response = await authenticatedFetch(url, init);
  if (!response.ok) {
    const body = await response.json().catch(() => ({}));
    if (response.status === 503) {
      throw new Error('Kafka broker is not enabled or not available');
    }
    throw new Error(body.message || body.error || `Request failed (${response.status})`);
  }
  return response.json() as Promise<T>;
}

const fetchStats = () => apiFetch<KafkaStats>(`${API_BASE}/stats`);
const fetchTopics = () => apiFetch<{ topics: KafkaTopicInfo[] }>(`${API_BASE}/topics`);
const fetchGroups = () => apiFetch<{ groups: KafkaConsumerGroupInfo[] }>(`${API_BASE}/groups`);
const produceMessage = (body: {
  topic: string;
  key?: string;
  value: string;
  partition?: number;
}) =>
  apiFetch<unknown>(`${API_BASE}/produce`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });

export function KafkaBrokerPage() {
  const { showToast } = useToast();
  const queryClient = useQueryClient();

  const [topic, setTopic] = useState('');
  const [key, setKey] = useState('');
  const [value, setValue] = useState('');
  const [partition, setPartition] = useState('');

  const statsQuery = useQuery({
    queryKey: ['kafka-stats'],
    queryFn: fetchStats,
    refetchInterval: 5000,
  });

  const topicsQuery = useQuery({
    queryKey: ['kafka-topics'],
    queryFn: fetchTopics,
    refetchInterval: 5000,
  });

  const groupsQuery = useQuery({
    queryKey: ['kafka-groups'],
    queryFn: fetchGroups,
    refetchInterval: 5000,
  });

  const produceMutation = useMutation({
    mutationFn: produceMessage,
    onSuccess: () => {
      showToast('Message produced', 'success');
      setValue('');
      queryClient.invalidateQueries({ queryKey: ['kafka-stats'] });
    },
    onError: (err: Error) => showToast(err.message, 'error'),
  });

  const handleProduce = (e: React.FormEvent) => {
    e.preventDefault();
    if (!topic.trim() || !value.trim()) {
      showToast('Topic and value are required', 'error');
      return;
    }
    const partitionNum = partition.trim() ? Number(partition) : undefined;
    if (partition.trim() && Number.isNaN(partitionNum!)) {
      showToast('Partition must be a number', 'error');
      return;
    }
    produceMutation.mutate({
      topic: topic.trim(),
      key: key.trim() || undefined,
      value,
      partition: partitionNum,
    });
  };

  const error = statsQuery.error || topicsQuery.error || groupsQuery.error;

  return (
    <div className="container mx-auto px-4 py-6 max-w-7xl">
      <div className="mb-6 flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold flex items-center gap-2">
            <Database className="h-6 w-6" />
            Kafka Broker
          </h1>
          <p className="text-sm text-muted-foreground">
            Monitor topics, consumer groups, and produce test messages.
          </p>
        </div>
        <Button
          variant="outline"
          size="sm"
          onClick={() => {
            statsQuery.refetch();
            topicsQuery.refetch();
            groupsQuery.refetch();
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
              {error instanceof Error ? error.message : 'Failed to load Kafka broker data'}
            </p>
          </CardContent>
        </Card>
      )}

      <div className="grid grid-cols-2 md:grid-cols-5 gap-4 mb-6">
        <StatCard label="Topics" value={statsQuery.data?.topics ?? '—'} />
        <StatCard label="Partitions" value={statsQuery.data?.partitions ?? '—'} />
        <StatCard label="Consumer groups" value={statsQuery.data?.consumer_groups ?? '—'} />
        <StatCard label="Produced" value={statsQuery.data?.messages_produced ?? '—'} />
        <StatCard label="Consumed" value={statsQuery.data?.messages_consumed ?? '—'} />
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4 mb-6">
        <Card>
          <CardHeader>
            <CardTitle className="text-base flex items-center gap-2">
              <Layers className="h-4 w-4" />
              Topics
            </CardTitle>
          </CardHeader>
          <CardContent>
            {!topicsQuery.data?.topics.length ? (
              <p className="text-muted-foreground text-sm text-center py-6">No topics</p>
            ) : (
              <div className="space-y-2 max-h-[320px] overflow-y-auto">
                {topicsQuery.data.topics.map((t) => (
                  <div
                    key={t.name}
                    className="p-2 border rounded text-sm flex items-center justify-between"
                  >
                    <div>
                      <div className="font-mono text-xs">{t.name}</div>
                      <div className="text-xs text-muted-foreground">
                        {t.partitions} partition{t.partitions === 1 ? '' : 's'} • RF {t.replication_factor}
                      </div>
                    </div>
                    <Button size="sm" variant="ghost" onClick={() => setTopic(t.name)}>
                      Use
                    </Button>
                  </div>
                ))}
              </div>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="text-base flex items-center gap-2">
              <Users className="h-4 w-4" />
              Consumer groups
            </CardTitle>
          </CardHeader>
          <CardContent>
            {!groupsQuery.data?.groups.length ? (
              <p className="text-muted-foreground text-sm text-center py-6">No consumer groups</p>
            ) : (
              <div className="space-y-2 max-h-[320px] overflow-y-auto">
                {groupsQuery.data.groups.map((g) => (
                  <div key={g.group_id} className="p-2 border rounded text-sm">
                    <div className="font-mono text-xs">{g.group_id}</div>
                    <div className="text-xs text-muted-foreground">
                      {g.members} member{g.members === 1 ? '' : 's'} • state: {g.state}
                    </div>
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
            Produce message
          </CardTitle>
          <CardDescription>Produce a message to a Kafka topic.</CardDescription>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleProduce} className="space-y-3">
            <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
              <div className="md:col-span-2">
                <Label>Topic</Label>
                <Input
                  value={topic}
                  onChange={(e) => setTopic(e.target.value)}
                  placeholder="orders.created"
                />
              </div>
              <div>
                <Label>Partition (optional)</Label>
                <Input
                  value={partition}
                  onChange={(e) => setPartition(e.target.value)}
                  placeholder="auto"
                  inputMode="numeric"
                />
              </div>
            </div>
            <div>
              <Label>Key (optional)</Label>
              <Input
                value={key}
                onChange={(e) => setKey(e.target.value)}
                placeholder="user-id-42"
              />
            </div>
            <div>
              <Label>Value</Label>
              <textarea
                value={value}
                onChange={(e) => setValue(e.target.value)}
                rows={5}
                className="w-full px-3 py-2 border border-border rounded-md bg-card font-mono text-sm"
                placeholder='{"orderId": 123, "total": 99.99}'
              />
            </div>
            <div className="flex justify-end">
              <Button type="submit" disabled={produceMutation.isPending}>
                {produceMutation.isPending ? 'Producing…' : 'Produce'}
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
