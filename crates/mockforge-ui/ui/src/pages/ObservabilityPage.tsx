import React, { useEffect, useState } from 'react';
import { Activity, Layers, AlertCircle, TrendingUp, Clock, Zap } from 'lucide-react';
import {
  PageHeader,
  ModernCard,
  MetricCard,
  Alert,
  Section,
  ModernBadge
} from '../components/ui/DesignSystem';
import { useWebSocket } from '../hooks/useWebSocket';

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
      .then(data => setStats(data))
      .catch(console.error);

    fetch('/api/observability/alerts')
      .then(res => res.json())
      .then(data => setAlerts(data))
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
            value={stats?.events_last_hour.toLocaleString() || '0'}
            subtitle="chaos events"
            icon={<Activity className="h-6 w-6" />}
          />
          <MetricCard
            title="Avg Latency"
            value={`${stats?.avg_latency_ms.toFixed(0) || 0}ms`}
            subtitle="response time"
            icon={<Clock className="h-6 w-6" />}
          />
          <MetricCard
            title="Active Alerts"
            value={stats?.active_alerts.toString() || '0'}
            subtitle="current issues"
            icon={<AlertCircle className="h-6 w-6" />}
            variant={stats && stats.active_alerts > 0 ? 'warning' : 'default'}
          />
          <MetricCard
            title="Impact Score"
            value={`${(stats?.current_impact_score || 0) * 100}%`}
            subtitle="system impact"
            icon={<TrendingUp className="h-6 w-6" />}
            variant={
              stats && stats.current_impact_score > 0.7 ? 'error' :
              stats && stats.current_impact_score > 0.3 ? 'warning' : 'default'
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
              <p className="text-gray-500 dark:text-gray-400">No active alerts</p>
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
                <p className="text-gray-500 dark:text-gray-400">Waiting for metrics...</p>
              </div>
            ) : (
              <div className="overflow-x-auto">
                <table className="w-full">
                  <thead>
                    <tr className="border-b border-gray-200 dark:border-gray-700">
                      <th className="text-left py-3 px-4">Time</th>
                      <th className="text-right py-3 px-4">Events</th>
                      <th className="text-right py-3 px-4">Latency (ms)</th>
                      <th className="text-right py-3 px-4">Faults</th>
                      <th className="text-right py-3 px-4">Rate Limits</th>
                    </tr>
                  </thead>
                  <tbody>
                    {recentMetrics.slice(-10).reverse().map((bucket, idx) => (
                      <tr key={idx} className="border-b border-gray-100 dark:border-gray-800">
                        <td className="py-3 px-4 font-mono text-sm">
                          {new Date(bucket.timestamp).toLocaleTimeString()}
                        </td>
                        <td className="py-3 px-4 text-right">{bucket.total_events}</td>
                        <td className="py-3 px-4 text-right">{bucket.avg_latency_ms.toFixed(0)}</td>
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
      {stats && stats.top_endpoints.length > 0 && (
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
            value={stats?.scheduled_scenarios.toString() || '0'}
            subtitle="upcoming"
            icon={<Layers className="h-6 w-6" />}
          />
          <MetricCard
            title="Active Orchestrations"
            value={stats?.active_orchestrations.toString() || '0'}
            subtitle="running"
            icon={<Activity className="h-6 w-6" />}
          />
          <MetricCard
            title="Active Replays"
            value={stats?.active_replays.toString() || '0'}
            subtitle="in progress"
            icon={<Zap className="h-6 w-6" />}
          />
        </div>
      </Section>
    </div>
  );
}
