import { useEffect, useState } from 'react';
import {
  Activity,
  Layers,
  AlertCircle,
  TrendingUp,
  Clock,
  Zap,
  Loader2,
  Plus,
  RefreshCw,
  Trash2,
  BarChart3,
} from 'lucide-react';
import {
  PageHeader,
  ModernCard,
  MetricCard,
  Alert,
  Section,
  ModernBadge
} from '../components/ui/DesignSystem';
import { Button } from '../components/ui/button';
import { useWebSocket } from '../hooks/useWebSocket';
import { isCloudMode } from '../utils/cloudMode';
import { useCloudOrgId } from '../hooks/useCloudOrgId';
import {
  cloudObservabilityApi,
  type ObservabilitySavedQuery,
  type ExecuteSavedQueryResponse,
} from '../services/api/cloudObservability';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

interface DashboardStats {
  timestamp: string;
  events_last_hour: number;
  events_last_day: number;
  avg_latency_ms: number;
  faults_last_hour: number;
  active_alerts: number;
  scheduled_scenarios: number;
  active_orchestrations: number;
  active_replays: number;
  current_impact_score: number;
  top_endpoints: Array<[string, number]>;
}

interface AlertData {
  id: string;
  severity: 'Info' | 'Warning' | 'Critical';
  message: string;
  alert_type: string;
  fired_at: string;
  resolved_at?: string;
}

interface MetricsBucket {
  timestamp: string;
  total_events: number;
  avg_latency_ms: number;
  total_faults: number;
  rate_limit_violations: number;
  affected_endpoints: Record<string, number>;
}

export function ObservabilityPage() {
  if (isCloudMode()) {
    return <CloudObservabilityView />;
  }
  return <LocalObservabilityView />;
}

function LocalObservabilityView() {
  const [stats, setStats] = useState<DashboardStats | null>(null);
  const [alerts, setAlerts] = useState<AlertData[]>([]);
  const [recentMetrics, setRecentMetrics] = useState<MetricsBucket[]>([]);

  // WebSocket connection for real-time updates
  const { lastMessage, connected } = useWebSocket('/api/observability/ws');

  // Process WebSocket messages
  useEffect(() => {
    if (!lastMessage) return;

    try {
      const data = JSON.parse(String(lastMessage.data));

      switch (data.type) {
        case 'Stats':
          setStats(data.stats);
          break;
        case 'Metrics':
          setRecentMetrics(prev => [...prev.slice(-19), data.bucket]);
          break;
        case 'AlertFired':
          setAlerts(prev => [data.alert, ...prev]);
          break;
        case 'AlertResolved':
          setAlerts(prev => prev.map(a =>
            a.id === data.alert_id ? { ...a, resolved_at: new Date().toISOString() } : a
          ));
          break;
      }
    } catch (e) {
      console.error('Failed to parse WebSocket message:', e);
    }
  }, [lastMessage]);

  // Fetch initial stats
  useEffect(() => {
    fetch('/api/observability/stats', { credentials: 'include' })
      .then(res => res.json())
      .then(data => {
        if (data && typeof data === 'object' && !Array.isArray(data)) {
          setStats(data);
        }
      })
      .catch(console.error);

    fetch('/api/observability/alerts', { credentials: 'include' })
      .then(res => res.json())
      .then(data => setAlerts(Array.isArray(data) ? data : []))
      .catch(console.error);
  }, []);

  return (
    <div className="space-y-8">
      <PageHeader
        title="Observability Dashboard"
        subtitle="Real-time chaos engineering and system observability"
        action={
          <ModernBadge variant={connected ? 'success' : 'error'}>
            {connected ? 'Connected' : 'Disconnected'}
          </ModernBadge>
        }
      />

      {/* Key Metrics */}
      <Section
        title="Real-Time Metrics"
        subtitle="Live chaos engineering and system metrics"
      >
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
          <MetricCard
            title="Events (Last Hour)"
            value={stats?.events_last_hour?.toLocaleString() || '0'}
            subtitle="chaos events"
            icon={<Activity className="h-6 w-6" />}
          />
          <MetricCard
            title="Avg Latency"
            value={`${stats?.avg_latency_ms?.toFixed(0) || 0}ms`}
            subtitle="response time"
            icon={<Clock className="h-6 w-6" />}
          />
          <MetricCard
            title="Active Alerts"
            value={stats?.active_alerts?.toString() || '0'}
            subtitle="current issues"
            icon={<AlertCircle className="h-6 w-6" />}
          />
          <MetricCard
            title="Impact Score"
            value={`${(stats?.current_impact_score || 0) * 100}%`}
            subtitle="system impact"
            icon={<TrendingUp className="h-6 w-6" />}
          />
        </div>
      </Section>

      {/* Active Alerts */}
      <Section
        title="Active Alerts"
        subtitle="Current system alerts and notifications"
      >
        <ModernCard>
          {alerts.filter(a => !a.resolved_at).length === 0 ? (
            <div className="text-center py-8">
              <p className="text-muted-foreground">No active alerts</p>
            </div>
          ) : (
            <div className="space-y-4">
              {alerts.filter(a => !a.resolved_at).map(alert => (
                <Alert
                  key={alert.id}
                  type={alert.severity === 'Critical' ? 'error' : alert.severity === 'Warning' ? 'warning' : 'info'}
                  title={`${alert.severity}: ${alert.alert_type}`}
                  message={alert.message}
                />
              ))}
            </div>
          )}
        </ModernCard>
      </Section>

      {/* Metrics Timeline */}
      <Section
        title="Metrics Timeline"
        subtitle="Real-time chaos event stream"
      >
        <ModernCard>
          <div className="space-y-4">
            {recentMetrics.length === 0 ? (
              <div className="text-center py-8">
                <p className="text-muted-foreground">Waiting for metrics...</p>
              </div>
            ) : (
              <div className="overflow-x-auto">
                <table className="w-full">
                  <thead>
                    <tr className="border-b border-border">
                      <th className="text-left py-3 px-4">Time</th>
                      <th className="text-right py-3 px-4">Events</th>
                      <th className="text-right py-3 px-4">Latency (ms)</th>
                      <th className="text-right py-3 px-4">Faults</th>
                      <th className="text-right py-3 px-4">Rate Limits</th>
                    </tr>
                  </thead>
                  <tbody>
                    {recentMetrics.slice(-10).reverse().map((bucket, idx) => (
                      <tr key={idx} className="border-b border-border">
                        <td className="py-3 px-4 font-mono text-sm">
                          {new Date(bucket.timestamp).toLocaleTimeString()}
                        </td>
                        <td className="py-3 px-4 text-right">{bucket.total_events}</td>
                        <td className="py-3 px-4 text-right">{(bucket.avg_latency_ms ?? 0).toFixed(0)}</td>
                        <td className="py-3 px-4 text-right">
                          <ModernBadge variant={bucket.total_faults > 0 ? 'error' : 'success'} size="sm">
                            {bucket.total_faults}
                          </ModernBadge>
                        </td>
                        <td className="py-3 px-4 text-right">
                          <ModernBadge variant={bucket.rate_limit_violations > 0 ? 'warning' : 'success'} size="sm">
                            {bucket.rate_limit_violations}
                          </ModernBadge>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </div>
        </ModernCard>
      </Section>

      {/* Top Affected Endpoints */}
      {stats && Array.isArray(stats.top_endpoints) && stats.top_endpoints.length > 0 && (
        <Section
          title="Top Affected Endpoints"
          subtitle="Endpoints experiencing the most chaos events"
        >
          <ModernCard>
            <div className="space-y-3">
              {stats.top_endpoints.slice(0, 5).map(([endpoint, count]) => (
                <div key={endpoint} className="flex items-center justify-between">
                  <span className="font-mono text-sm">{endpoint}</span>
                  <ModernBadge>{count} events</ModernBadge>
                </div>
              ))}
            </div>
          </ModernCard>
        </Section>
      )}

      {/* Chaos Scenarios Status */}
      <Section
        title="Chaos Status"
        subtitle="Active chaos engineering activities"
      >
        <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
          <MetricCard
            title="Scheduled Scenarios"
            value={stats?.scheduled_scenarios?.toString() || '0'}
            subtitle="upcoming"
            icon={<Layers className="h-6 w-6" />}
          />
          <MetricCard
            title="Active Orchestrations"
            value={stats?.active_orchestrations?.toString() || '0'}
            subtitle="running"
            icon={<Activity className="h-6 w-6" />}
          />
          <MetricCard
            title="Active Replays"
            value={stats?.active_replays?.toString() || '0'}
            subtitle="in progress"
            icon={<Zap className="h-6 w-6" />}
          />
        </div>
      </Section>
    </div>
  );
}

// --- Cloud-mode view (#465) -----------------------------------------------
//
// Each saved query renders as a live tile: it executes on mount and
// re-executes every TILE_REFRESH_MS through cloudObservabilityApi
// .executeSavedQuery. Tiles are created in-page (preset metrics +
// window) so users never need to hand-craft the saved-query JSON.

const TILE_REFRESH_MS = 60_000;

type TileMetric = 'request_count' | 'request_count_by_status' | 'incident_count';

const TILE_METRICS: Array<{ value: TileMetric; label: string; description: string }> = [
  {
    value: 'request_count',
    label: 'Request volume',
    description: 'Total requests served by your hosted mocks.',
  },
  {
    value: 'request_count_by_status',
    label: 'Requests by status code',
    description: 'Request counts grouped by HTTP status code.',
  },
  {
    value: 'incident_count',
    label: 'Incidents',
    description: 'Incidents opened in your organization, by severity.',
  },
];

const TILE_WINDOWS: Array<{ value: number; label: string }> = [
  { value: 15, label: 'Last 15 minutes' },
  { value: 60, label: 'Last hour' },
  { value: 24 * 60, label: 'Last 24 hours' },
];

const STARTER_TILES: Array<{ name: string; metric: TileMetric; window: number }> = [
  { name: 'Requests (last hour)', metric: 'request_count', window: 60 },
  { name: 'Status codes (last hour)', metric: 'request_count_by_status', window: 60 },
  { name: 'Incidents (last 24 hours)', metric: 'incident_count', window: 24 * 60 },
];

function isTileMetric(kind: string | null): kind is TileMetric {
  return !!kind && TILE_METRICS.some((m) => m.value === kind);
}

function formatWindow(minutes: number): string {
  if (minutes % (24 * 60) === 0) {
    const days = minutes / (24 * 60);
    return days === 1 ? '24 hours' : `${days} days`;
  }
  if (minutes % 60 === 0) {
    const hours = minutes / 60;
    return hours === 1 ? '1 hour' : `${hours} hours`;
  }
  return `${minutes} minutes`;
}

function CloudObservabilityView() {
  const orgId = useCloudOrgId();
  const queryClient = useQueryClient();
  const [showCreate, setShowCreate] = useState(false);

  const savedQueriesKey = ['cloud', 'observability', 'saved-queries', orgId];
  const savedQueriesQuery = useQuery({
    queryKey: savedQueriesKey,
    queryFn: () => cloudObservabilityApi.listSavedQueries(orgId!),
    enabled: !!orgId,
  });

  const createTiles = useMutation({
    mutationFn: async (tiles: Array<{ name: string; metric: TileMetric; window: number }>) => {
      for (const tile of tiles) {
        await cloudObservabilityApi.createSavedQuery(orgId!, {
          name: tile.name,
          kind: 'metrics',
          filters: { kind: tile.metric, window_minutes: tile.window },
        });
      }
    },
    onSuccess: () => {
      setShowCreate(false);
      queryClient.invalidateQueries({ queryKey: savedQueriesKey });
    },
  });

  if (!orgId) {
    return (
      <div className="space-y-8">
        <PageHeader title="Observability" subtitle="Live metrics for your hosted mocks and incidents." />
        <Alert type="info" message="No active organization. Sign in or select an organization to view metrics." />
      </div>
    );
  }

  const queries = savedQueriesQuery.data ?? [];

  return (
    <div className="space-y-8">
      <PageHeader
        title="Observability"
        subtitle="Live tiles for request volume, status codes, and incidents across your hosted mocks. Tiles refresh every minute."
        action={
          queries.length > 0 && !showCreate ? (
            <Button size="sm" onClick={() => setShowCreate(true)}>
              <Plus className="h-4 w-4 mr-1" /> Add tile
            </Button>
          ) : undefined
        }
      />

      {createTiles.error && (
        <Alert type="error" message={`Could not create tile: ${(createTiles.error as Error).message}`} />
      )}

      {showCreate && (
        <CreateTileForm
          submitting={createTiles.isPending}
          onCancel={() => setShowCreate(false)}
          onSubmit={(tile) => createTiles.mutate([tile])}
        />
      )}

      {savedQueriesQuery.isLoading ? (
        <div className="flex items-center justify-center py-12 text-muted-foreground">
          <Loader2 className="h-5 w-5 animate-spin mr-2" /> Loading tiles…
        </div>
      ) : savedQueriesQuery.error ? (
        <Alert type="error" message={`Failed to load tiles: ${(savedQueriesQuery.error as Error).message}`} />
      ) : queries.length === 0 ? (
        !showCreate && (
          <ModernCard>
            <div className="flex flex-col items-center text-center py-10 px-4">
              <div className="p-4 rounded-full bg-muted text-muted-foreground mb-4">
                <BarChart3 className="h-8 w-8" />
              </div>
              <h3 className="text-lg font-semibold text-foreground mb-2">No tiles yet</h3>
              <p className="text-sm text-muted-foreground max-w-md mb-6">
                Tiles track request volume, status codes, and incidents for your hosted mocks. Start with
                the recommended set or build your own.
              </p>
              <div className="flex flex-wrap justify-center gap-2">
                <Button onClick={() => createTiles.mutate(STARTER_TILES)} disabled={createTiles.isPending}>
                  {createTiles.isPending ? (
                    <Loader2 className="h-4 w-4 animate-spin mr-1" />
                  ) : (
                    <Plus className="h-4 w-4 mr-1" />
                  )}
                  Add recommended tiles
                </Button>
                <Button variant="outline" onClick={() => setShowCreate(true)}>
                  Build a custom tile
                </Button>
              </div>
            </div>
          </ModernCard>
        )
      ) : (
        <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
          {queries.map((q) => (
            <SavedQueryTile
              key={q.id}
              query={q}
              onDeleted={() => queryClient.invalidateQueries({ queryKey: savedQueriesKey })}
            />
          ))}
        </div>
      )}
    </div>
  );
}

function CreateTileForm({
  submitting,
  onCancel,
  onSubmit,
}: {
  submitting: boolean;
  onCancel: () => void;
  onSubmit: (tile: { name: string; metric: TileMetric; window: number }) => void;
}) {
  const [metric, setMetric] = useState<TileMetric>('request_count');
  const [windowMinutes, setWindowMinutes] = useState(60);
  const [name, setName] = useState('');

  const metricMeta = TILE_METRICS.find((m) => m.value === metric)!;
  const windowLabel = TILE_WINDOWS.find((w) => w.value === windowMinutes)?.label ?? formatWindow(windowMinutes);
  const effectiveName = name.trim() || `${metricMeta.label} (${windowLabel.toLowerCase()})`;
  const fieldClass =
    'w-full px-3 py-2 text-sm bg-background border border-border rounded-md focus:outline-none focus:ring-2 focus:ring-ring';

  return (
    <ModernCard>
      <form
        className="space-y-4"
        onSubmit={(e) => {
          e.preventDefault();
          onSubmit({ name: effectiveName, metric, window: windowMinutes });
        }}
      >
        <h3 className="text-base font-semibold text-foreground">New tile</h3>
        <div className="grid gap-4 md:grid-cols-3">
          <label className="space-y-1 text-sm">
            <span className="font-medium text-foreground">Metric</span>
            <select className={fieldClass} value={metric} onChange={(e) => setMetric(e.target.value as TileMetric)}>
              {TILE_METRICS.map((m) => (
                <option key={m.value} value={m.value}>
                  {m.label}
                </option>
              ))}
            </select>
            <span className="block text-xs text-muted-foreground">{metricMeta.description}</span>
          </label>
          <label className="space-y-1 text-sm">
            <span className="font-medium text-foreground">Time window</span>
            <select className={fieldClass} value={windowMinutes} onChange={(e) => setWindowMinutes(Number(e.target.value))}>
              {TILE_WINDOWS.map((w) => (
                <option key={w.value} value={w.value}>
                  {w.label}
                </option>
              ))}
            </select>
          </label>
          <label className="space-y-1 text-sm">
            <span className="font-medium text-foreground">Name</span>
            <input
              className={fieldClass}
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder={effectiveName}
              maxLength={120}
            />
          </label>
        </div>
        <div className="flex justify-end gap-2">
          <Button type="button" variant="outline" onClick={onCancel}>
            Cancel
          </Button>
          <Button type="submit" disabled={submitting}>
            {submitting ? <Loader2 className="h-4 w-4 animate-spin" /> : 'Add tile'}
          </Button>
        </div>
      </form>
    </ModernCard>
  );
}

function SavedQueryTile({ query, onDeleted }: { query: ObservabilitySavedQuery; onDeleted: () => void }) {
  const kind = (query.filters?.kind as string | undefined) ?? null;
  const supported = isTileMetric(kind);
  const metricLabel = TILE_METRICS.find((m) => m.value === kind)?.label;
  const [confirmDelete, setConfirmDelete] = useState(false);

  const result = useQuery({
    queryKey: ['cloud', 'observability', 'execute', query.id, query.updated_at],
    queryFn: () => cloudObservabilityApi.executeSavedQuery(query.id),
    enabled: supported,
    refetchInterval: TILE_REFRESH_MS,
  });

  const remove = useMutation({
    mutationFn: () => cloudObservabilityApi.deleteSavedQuery(query.id),
    onSuccess: onDeleted,
  });

  const data: ExecuteSavedQueryResponse | undefined = result.data;
  const maxCount = data ? Math.max(1, ...data.series.map((s) => s.count)) : 1;

  return (
    <ModernCard className="flex flex-col gap-3">
      <div className="flex items-start justify-between gap-2">
        <div className="min-w-0">
          <div className="font-medium text-foreground truncate" title={query.name}>
            {query.name}
          </div>
          <div className="text-xs text-muted-foreground truncate">
            {query.description ?? metricLabel ?? 'Custom query'}
          </div>
        </div>
        <div className="flex items-center gap-1 flex-shrink-0">
          {supported && (
            <Button
              size="sm"
              variant="ghost"
              className="h-8 w-8 p-0"
              onClick={() => result.refetch()}
              disabled={result.isFetching}
              aria-label={`Refresh ${query.name}`}
              title="Refresh now"
            >
              <RefreshCw className={`h-4 w-4 ${result.isFetching ? 'animate-spin' : ''}`} />
            </Button>
          )}
          {confirmDelete ? (
            <>
              <Button size="sm" variant="destructive" onClick={() => remove.mutate()} disabled={remove.isPending}>
                Remove
              </Button>
              <Button size="sm" variant="ghost" onClick={() => setConfirmDelete(false)}>
                Keep
              </Button>
            </>
          ) : (
            <Button
              size="sm"
              variant="ghost"
              className="h-8 w-8 p-0 text-muted-foreground hover:text-destructive"
              onClick={() => setConfirmDelete(true)}
              aria-label={`Remove ${query.name}`}
              title="Remove tile"
            >
              <Trash2 className="h-4 w-4" />
            </Button>
          )}
        </div>
      </div>

      {remove.error && <Alert type="error" message={(remove.error as Error).message} />}

      {!supported ? (
        <p className="text-sm text-muted-foreground">
          This saved query uses a filter this dashboard can't chart. Tiles support request volume, status
          codes, and incidents.
        </p>
      ) : result.isLoading ? (
        <div className="flex items-center text-sm text-muted-foreground py-4">
          <Loader2 className="h-4 w-4 animate-spin mr-2" /> Loading…
        </div>
      ) : result.error ? (
        <Alert type="error" message={(result.error as Error).message} />
      ) : data ? (
        <>
          <div>
            <div className="text-3xl font-bold text-foreground">{data.total.toLocaleString()}</div>
            <div className="text-xs text-muted-foreground">in the last {formatWindow(data.window_minutes)}</div>
          </div>
          {kind === 'request_count' ? null : data.series.length === 0 ? (
            <p className="text-sm text-muted-foreground">No activity in this window.</p>
          ) : (
            <div className="space-y-1.5">
              {data.series.slice(0, 6).map((s) => (
                <div key={s.label} className="text-xs">
                  <div className="flex justify-between gap-2">
                    <code className="text-muted-foreground truncate" title={s.label}>
                      {s.label}
                    </code>
                    <span className="font-medium text-foreground">{s.count.toLocaleString()}</span>
                  </div>
                  <div className="h-1.5 rounded bg-muted overflow-hidden mt-0.5">
                    <div className="h-full rounded bg-brand-500" style={{ width: `${(s.count / maxCount) * 100}%` }} />
                  </div>
                </div>
              ))}
            </div>
          )}
        </>
      ) : null}
    </ModernCard>
  );
}
