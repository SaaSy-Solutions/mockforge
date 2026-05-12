import React, { useEffect, useState } from 'react';
import { Activity, Layers, AlertCircle, TrendingUp, Clock, Zap, Play, Loader2 } from 'lucide-react';
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
import { useQuery } from '@tanstack/react-query';

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
  const [connected, setConnected] = useState(false);

  // WebSocket connection for real-time updates
  const { lastMessage } = useWebSocket('/api/observability/ws', {
    onOpen: () => setConnected(true),
    onClose: () => setConnected(false),
  });

  // Process WebSocket messages
  useEffect(() => {
    if (!lastMessage) return;

    try {
      const data = JSON.parse(lastMessage);

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
    fetch('/api/observability/stats')
      .then(res => res.json())
      .then(data => {
        if (data && typeof data === 'object' && !Array.isArray(data)) {
          setStats(data);
        }
      })
      .catch(console.error);

    fetch('/api/observability/alerts')
      .then(res => res.json())
      .then(data => setAlerts(Array.isArray(data) ? data : []))
      .catch(console.error);
  }, []);

  return (
    <div className="space-y-8">
      <PageHeader
        title="Observability Dashboard"
        subtitle="Real-time chaos engineering and system observability"
        actions={
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
            variant={stats && (stats.active_alerts ?? 0) > 0 ? 'warning' : 'default'}
          />
          <MetricCard
            title="Impact Score"
            value={`${(stats?.current_impact_score || 0) * 100}%`}
            subtitle="system impact"
            icon={<TrendingUp className="h-6 w-6" />}
            variant={
              stats && (stats.current_impact_score ?? 0) > 0.7 ? 'error' :
              stats && (stats.current_impact_score ?? 0) > 0.3 ? 'warning' : 'default'
            }
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
// Lists the org's saved queries and lets the user run any of them
// on-demand. Phase 1 supports three `kind`s in the saved query's
// `filters` payload — `request_count`, `request_count_by_status`,
// `incident_count` — and renders the flat
// {metric,total,window_minutes,series:[{label,count}]} response as a
// small bar list. Live event-stream tiles + dashboard layouts land in a
// follow-up slice.
function CloudObservabilityView() {
  const orgId = useCloudOrgId();
  const savedQueriesQuery = useQuery({
    queryKey: ['cloud', 'observability', 'saved-queries', orgId],
    queryFn: () => cloudObservabilityApi.listSavedQueries(orgId!),
    enabled: !!orgId,
  });

  if (!orgId) {
    return (
      <div className="space-y-8">
        <PageHeader title="Observability" subtitle="Run saved queries against your cloud workspace data." />
        <Alert type="info" message="No active organization. Sign in or select an org to view saved queries." />
      </div>
    );
  }

  return (
    <div className="space-y-8">
      <PageHeader
        title="Observability"
        subtitle="Saved queries over request captures and incidents. Execute on-demand; live tiles ship in a follow-up."
      />
      <Section title="Saved queries">
        <ModernCard>
          {savedQueriesQuery.isLoading ? (
            <div className="flex items-center justify-center py-8 text-muted-foreground">
              <Loader2 className="h-5 w-5 animate-spin mr-2" /> Loading saved queries…
            </div>
          ) : savedQueriesQuery.error ? (
            <Alert type="error" message={`Failed to load saved queries: ${(savedQueriesQuery.error as Error).message}`} />
          ) : (savedQueriesQuery.data ?? []).length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              No saved queries yet. Create one with{' '}
              <code className="text-xs bg-muted px-1 py-0.5 rounded">
                POST /api/v1/organizations/{'{'}org_id{'}'}/observability/saved-queries
              </code>{' '}
              and it will appear here.
            </div>
          ) : (
            <div className="space-y-3">
              {savedQueriesQuery.data!.map((q) => (
                <SavedQueryCard key={q.id} query={q} />
              ))}
            </div>
          )}
        </ModernCard>
      </Section>
    </div>
  );
}

function SavedQueryCard({ query }: { query: ObservabilitySavedQuery }) {
  const [result, setResult] = useState<ExecuteSavedQueryResponse | null>(null);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const kind = (query.filters?.kind as string | undefined) ?? null;
  const supportedKinds = ['request_count', 'request_count_by_status', 'incident_count'];
  const isSupported = !!kind && supportedKinds.includes(kind);

  const onRun = async () => {
    setRunning(true);
    setError(null);
    try {
      const res = await cloudObservabilityApi.executeSavedQuery(query.id);
      setResult(res);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Execute failed');
    } finally {
      setRunning(false);
    }
  };

  return (
    <div className="border border-border rounded-md p-4 space-y-3">
      <div className="flex items-center justify-between gap-4">
        <div className="min-w-0">
          <div className="font-medium truncate">{query.name}</div>
          {query.description && (
            <div className="text-sm text-muted-foreground truncate">{query.description}</div>
          )}
          <div className="text-xs text-muted-foreground mt-1">
            kind: <code>{kind ?? '(missing)'}</code>
            {!isSupported && (
              <span className="ml-2 text-warning-700 dark:text-warning-400">
                (Phase 1 supports {supportedKinds.join(', ')})
              </span>
            )}
          </div>
        </div>
        <Button onClick={onRun} disabled={running || !isSupported} size="sm">
          {running ? (
            <Loader2 className="h-4 w-4 animate-spin" />
          ) : (
            <>
              <Play className="h-4 w-4 mr-1" /> Run
            </>
          )}
        </Button>
      </div>
      {error && <Alert type="error" message={error} />}
      {result && (
        <div className="text-sm space-y-2">
          <div>
            <span className="text-muted-foreground">total over {result.window_minutes}m:</span>{' '}
            <span className="font-medium">{result.total.toLocaleString()}</span>
          </div>
          <div className="space-y-1">
            {result.series.map((s) => (
              <div key={s.label} className="flex justify-between text-xs">
                <code className="text-muted-foreground">{s.label}</code>
                <span>{s.count.toLocaleString()}</span>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
