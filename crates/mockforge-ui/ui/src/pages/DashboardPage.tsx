import { ServerTable } from '../components/dashboard/ServerTable';
import { RequestLog } from '../components/dashboard/RequestLog';
import { CloudActivityFeed } from '../components/dashboard/CloudActivityFeed';
import { LatencyHistogram } from '../components/metrics/LatencyHistogram';
import { TimeTravelWidget } from '../components/time-travel/TimeTravelWidget';
import { RealitySlider } from '../components/reality/RealitySlider';
import { RealityIndicator } from '../components/reality/RealityIndicator';
import { useRealityShortcuts } from '../hooks/useRealityShortcuts';
import { useDashboardStream } from '../hooks/useDashboardStream';
import type { CloudDashboardMetrics, LatencyMetrics, LogEntry } from '../types';
import { useDashboard, useLogs } from '../hooks/useApi';
import { isCloudMode } from '../utils/cloudMode';
import {
  PageHeader,
  MetricCard,
  Alert,
  Section
} from '../components/ui/DesignSystem';
import { MetricIcon, StatusIcon } from '../components/ui/IconSystem';
import { DashboardLoading, ErrorState } from '../components/ui/LoadingStates';
import { Wifi, WifiOff } from 'lucide-react';

const isCloud = isCloudMode();

function formatUptime(seconds: number): string {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  if (hours > 0) {
    return `${hours}h ${minutes}m`;
  }
  return `${minutes}m`;
}

function formatBytes(bytes: number): string {
  if (bytes <= 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  const exp = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  const value = bytes / Math.pow(1024, exp);
  return `${value.toFixed(value >= 100 || exp === 0 ? 0 : 1)} ${units[exp]}`;
}

function safePercentage(value: number, total: number): string {
  return total === 0 ? '0.0' : ((value / total) * 100).toFixed(1);
}

// Type guard to validate LogEntry objects
function isLogEntry(obj: unknown): obj is LogEntry {
  if (typeof obj !== 'object' || obj === null) return false;
  const entry = obj as Record<string, unknown>;

  return (
    typeof entry.timestamp === 'string' &&
    (typeof entry.status === 'number' || typeof entry.status_code === 'number') &&
    typeof entry.method === 'string' &&
    (typeof entry.url === 'string' || typeof entry.path === 'string')
  );
}

function computeFailureCounters(logs: unknown): { total2xx: number; total4xx: number; total5xx: number } {
  if (!logs || !Array.isArray(logs)) return { total2xx: 0, total4xx: 0, total5xx: 0 };

  const validLogs = logs.filter(isLogEntry);
  return validLogs.reduce((acc: { total2xx: number; total4xx: number; total5xx: number }, log) => {
    const code = log.status_code;
    if (code >= 500) acc.total5xx++;
    else if (code >= 400) acc.total4xx++;
    else if (code >= 200) acc.total2xx++;
    return acc;
  }, { total2xx: 0, total4xx: 0, total5xx: 0 });
}

function computeLatencyMetrics(logs: unknown) {
  if (!logs || !Array.isArray(logs)) return [];

  const logEntries = logs.filter(isLogEntry);
  const responseTimes = logEntries
    .map(log => log.response_time_ms)
    .filter((time): time is number => time !== undefined);

  if (responseTimes.length === 0) return [];

  // Calculate statistics
  const sorted = [...responseTimes].sort((a, b) => a - b);
  const sum = responseTimes.reduce((acc, time) => acc + time, 0);
  const avg = sum / responseTimes.length;
  const min = sorted[0];
  const max = sorted[sorted.length - 1];
  const p50 = sorted[Math.floor(sorted.length * 0.5)];
  const p95 = sorted[Math.floor(sorted.length * 0.95)];
  const p99 = sorted[Math.floor(sorted.length * 0.99)];

  // Build histogram
  const latencyData = responseTimes.reduce((acc: Record<string, number>, time) => {
    const rounded = Math.floor(time / 10) * 10;
    const range = `${rounded}-${rounded + 9}`;
    acc[range] = (acc[range] || 0) + 1;
    return acc;
  }, {});

  return [{
    service: 'MockForge',
    route: 'api/*',
    avg_response_time: avg,
    min_response_time: min,
    max_response_time: max,
    p50_response_time: p50,
    p95_response_time: p95,
    p99_response_time: p99,
    total_requests: logEntries.length,
    histogram: Object.entries(latencyData)
      .sort(([a], [b]) => parseInt(a) - parseInt(b))
      .map(([range, count]) => ({ range, count: count as number }))
      .slice(0, 20) // Limit to first 20 ranges
  }];
}

export function DashboardPage() {
  // All hooks must be called unconditionally and in the same order every render
  const { data: dashboard, isLoading, error, isFetching } = useDashboard();
  // Refetch logs every 3 seconds for dashboard metrics to stay in sync with SSE updates.
  // Skipped in cloud mode — cloud has no per-request log stream; CloudActivityFeed
  // covers the audit/activity surface instead.
  const { data: logs } = useLogs({ limit: 100, refetchInterval: isCloud ? 0 : 3000 });

  // Wire real-time WebSocket updates — patches React Query cache on incoming events.
  // Polling continues as fallback when the WebSocket is unavailable. The /__mockforge/ws
  // endpoint doesn't exist in cloud, so disable the connect attempt entirely there.
  const { connected: wsConnected } = useDashboardStream({ enabled: !isCloud });

  // Enable keyboard shortcuts for reality level changes
  // This hook must be called unconditionally before any early returns
  useRealityShortcuts();

  const cloudMetrics: CloudDashboardMetrics | undefined = dashboard?.cloud_metrics;

  const failureCounters = isCloud && cloudMetrics
    ? {
        total2xx: cloudMetrics.requests_2xx,
        total4xx: cloudMetrics.requests_4xx,
        total5xx: cloudMetrics.requests_5xx,
      }
    : computeFailureCounters(logs);
  const latencyMetrics = computeLatencyMetrics(logs);

  if (isLoading) {
    return (
      <div className="content-width space-y-8">
        <PageHeader
          title="Dashboard"
          subtitle="System overview and performance metrics"
          className="space-section"
        />
        <DashboardLoading />
      </div>
    );
  }

  if (error) {
    return (
      <div className="content-width space-y-8">
        <PageHeader
          title="Dashboard"
          subtitle="System overview and performance metrics"
          className="space-section"
        />
        <ErrorState
          title="Failed to load dashboard"
          description="Unable to retrieve dashboard data. Please try refreshing the page."
          error={error}
          retry={() => window.location.reload()}
        />
      </div>
    );
  }

  const system = dashboard?.system;

  if (!dashboard || !system) {
    return (
      <div className="space-y-8">
        <PageHeader
          title="Dashboard"
          subtitle="System overview and performance metrics"
        />
        <Alert
          type="warning"
          title="No data available"
          message="Unable to retrieve dashboard data. The system might be initializing."
        />
      </div>
    );
  }

  const totalRequests = dashboard.metrics?.total_requests ?? 0;
  const errorRate = dashboard.metrics?.error_rate ?? 0;
  const avgResponseTime = dashboard.metrics?.average_response_time ?? 0;

  return (
    <div className="content-width">
      <PageHeader
        title="Dashboard"
        subtitle="Real-time system overview and performance metrics"
        className="space-section"
        action={isCloud ? null : <RealityIndicator />}
      />

      {/* Reality Slider and Time Travel Widget — local mode only.
          In cloud, /__mockforge/reality and /__mockforge/time-travel are stubbed
          no-ops, so showing the controls just misleads users. */}
      {!isCloud && (
        <>
          <Section
            title="Environment Control"
            subtitle="Adjust realism and temporal simulation for testing"
            className="space-section section-breathing"
          >
            <div className="grid grid-cols-1 lg:grid-cols-2 grid-gap-lg">
              <RealitySlider />
              <TimeTravelWidget />
            </div>
          </Section>

          <div className="divider-accent my-12"></div>
        </>
      )}

      <Section
        title="System Metrics"
        subtitle={isCloud ? 'Hosted-deployment activity for this organization' : 'Current system performance indicators'}
        className="space-section section-breathing"
        action={
          isCloud ? (
            <div className="flex items-center gap-2 text-sm text-green-600 dark:text-green-400">
              <div className="relative">
                <div className="w-2 h-2 rounded-full bg-green-500 animate-pulse" />
                <div className="absolute inset-0 w-2 h-2 rounded-full bg-green-500 animate-ping opacity-75" />
              </div>
              <span className="font-medium">Live</span>
              {isFetching && (
                <span className="text-xs text-gray-500 dark:text-gray-400 ml-1">(updating...)</span>
              )}
            </div>
          ) : (
            <div className="flex items-center gap-3 text-sm">
              <div className={`flex items-center gap-1.5 ${wsConnected ? 'text-green-600 dark:text-green-400' : 'text-gray-400 dark:text-gray-500'}`}>
                {wsConnected ? (
                  <Wifi className="h-3.5 w-3.5" />
                ) : (
                  <WifiOff className="h-3.5 w-3.5" />
                )}
                <span className="text-xs font-medium">
                  {wsConnected ? 'Streaming' : 'Polling'}
                </span>
              </div>
              <div className="relative flex items-center gap-2 text-green-600 dark:text-green-400">
                <div className="relative">
                  <div className="w-2 h-2 rounded-full bg-green-500 animate-pulse" />
                  <div className="absolute inset-0 w-2 h-2 rounded-full bg-green-500 animate-ping opacity-75" />
                </div>
                <span className="font-medium">Live</span>
                {isFetching && !wsConnected && (
                  <span className="text-xs text-gray-500 dark:text-gray-400 ml-1">(updating...)</span>
                )}
              </div>
            </div>
          )
        }
      >
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 grid-gap-lg">
          {isCloud ? (
            <>
              <MetricCard
                title="Active Deployments"
                value={(cloudMetrics?.active_deployments ?? 0).toString()}
                subtitle={`${cloudMetrics?.total_deployments ?? 0} total`}
                icon={<MetricIcon metric="activity" size="lg" />}
                className="animate-stagger-in animate-delay-75"
              />
              <MetricCard
                title="Requests (period)"
                value={totalRequests.toLocaleString()}
                subtitle={
                  cloudMetrics?.period_start
                    ? `since ${cloudMetrics.period_start}`
                    : 'this billing period'
                }
                icon={<MetricIcon metric="performance" size="lg" />}
                className="animate-stagger-in animate-delay-150"
              />
              <MetricCard
                title="Avg Response Time"
                value={`${Math.round(avgResponseTime)}ms`}
                subtitle="weighted across deployments"
                icon={<MetricIcon metric="performance" size="lg" />}
                className="animate-stagger-in animate-delay-200"
              />
              <MetricCard
                title="Egress"
                value={formatBytes(cloudMetrics?.egress_bytes ?? 0)}
                subtitle={`${errorRate.toFixed(1)}% error rate`}
                icon={<MetricIcon metric="memory" size="lg" />}
                className="animate-stagger-in animate-delay-300"
              />
            </>
          ) : (
            <>
              <MetricCard
                title="Uptime"
                value={formatUptime(system.uptime_seconds)}
                subtitle="system uptime"
                icon={<MetricIcon metric="uptime" size="lg" />}
                className="animate-stagger-in animate-delay-75"
              />
              <MetricCard
                title="CPU Usage"
                value={`${system.cpu_usage_percent.toFixed(1)}%`}
                subtitle="current utilization"
                icon={<MetricIcon metric="cpu" size="lg" />}
                className="animate-stagger-in animate-delay-150"
              />
              <MetricCard
                title="Memory"
                value={`${system.memory_usage_mb} MB`}
                subtitle="allocated"
                icon={<MetricIcon metric="memory" size="lg" />}
                className="animate-stagger-in animate-delay-200"
              />
              <MetricCard
                title="Active Threads"
                value={system.active_threads.toString()}
                subtitle="running threads"
                icon={<MetricIcon metric="activity" size="lg" />}
                className="animate-stagger-in animate-delay-300"
              />
            </>
          )}
        </div>

        {/* Failure Counters */}
        <div className="divider-soft my-8"></div>
        <div className="visual-group">
          <h3 className="text-lg font-bold text-gray-900 dark:text-gray-100 mb-4">Response Status Distribution</h3>
          <div className="grid grid-cols-1 md:grid-cols-3 grid-gap-md">
          <MetricCard
            title="Success Responses"
            value={failureCounters.total2xx.toLocaleString()}
            subtitle={`${safePercentage(failureCounters.total2xx, failureCounters.total2xx + failureCounters.total4xx + failureCounters.total5xx)}% of total`}
            icon={<StatusIcon status="success" size="lg" />}
            className="animate-fade-in-up animate-delay-100"
          />
          <MetricCard
            title="Client Errors"
            value={failureCounters.total4xx.toLocaleString()}
            subtitle={`${safePercentage(failureCounters.total4xx, failureCounters.total2xx + failureCounters.total4xx + failureCounters.total5xx)}% of total`}
            icon={<StatusIcon status="warning" size="lg" />}
            className="animate-fade-in-up animate-delay-200"
          />
          <MetricCard
            title="Server Errors"
            value={failureCounters.total5xx.toLocaleString()}
            subtitle={`${safePercentage(failureCounters.total5xx, failureCounters.total2xx + failureCounters.total4xx + failureCounters.total5xx)}% of total`}
            icon={<StatusIcon status="error" size="lg" />}
            className="animate-fade-in-up animate-delay-300"
          />
          </div>
        </div>
      </Section>

      <div className="divider-accent my-12"></div>

      {/* Performance Metrics — local only. Cloud doesn't expose per-request response
          times yet (only weighted averages), so the histogram has nothing useful to show. */}
      {!isCloud && (
        <>
          <Section
            title="Performance Metrics"
            subtitle="Response time distribution and latency analysis"
            className="space-section section-breathing"
          >
            <div className="space-component">
              {latencyMetrics.length > 0 ? (
                 <LatencyHistogram
                   metrics={latencyMetrics as LatencyMetrics[]}
                   selectedService={undefined}
                   onServiceChange={() => {}}
                 />
              ) : (
                <div className="text-center py-8">
                  <div className="inline-flex items-center justify-center w-16 h-16 rounded-full bg-gray-100 dark:bg-gray-800 mb-4">
                    <MetricIcon metric="performance" size="2xl" />
                  </div>
                  <div className="text-lg font-medium text-gray-900 dark:text-gray-100 mb-2">
                    No Latency Data Available
                  </div>
                  <p className="text-gray-600 dark:text-gray-400">
                    Latency metrics will appear here once requests have been processed.
                  </p>
                </div>
              )}
            </div>
          </Section>

          <div className="divider-soft my-8"></div>
        </>
      )}

      <Section
        title="System Status"
        subtitle={isCloud ? 'Active deployments and recent organization activity' : 'Server instances and recent activity'}
        className="space-section"
      >
        <div className="space-component">
          <div>
            <ServerTable />
          </div>

          {/* Cloud uses an audit-event activity feed; local uses the per-request log stream. */}
          <div>
            {isCloud ? <CloudActivityFeed /> : <RequestLog />}
          </div>
        </div>
      </Section>

      {/* Additional dashboard sections could go here */}
      <Section
        title="System Health"
        subtitle="Overall system status and alerts"
        className="space-section"
      >
        <div className="grid grid-cols-1 md:grid-cols-2 grid-gap-lg">
          <Alert
            type="success"
            title="All Systems Operational"
            message="MockForge is running normally with all services active."
          />
          <div className="bg-white dark:bg-gray-900 border border-gray-200 dark:border-gray-800 rounded-xl p-6">
            <div className="flex items-center justify-between">
              <div>
                <h3 className="font-semibold text-gray-900 dark:text-gray-100">Version</h3>
                <p className="text-sm text-gray-600 dark:text-gray-400 mt-1">
                  {system.version}
                </p>
              </div>
              <div className="text-right">
                <p className="text-sm text-gray-500 dark:text-gray-400">
                  {isCloud ? 'Workspaces' : 'Routes'}
                </p>
                <p className="text-lg font-semibold text-gray-900 dark:text-gray-100">
                  {isCloud
                    ? cloudMetrics?.workspaces ?? 0
                    : system.total_routes}
                </p>
              </div>
            </div>
          </div>
        </div>
      </Section>
    </div>
  );
}
