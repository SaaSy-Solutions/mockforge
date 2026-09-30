import type { ReactNode } from 'react';
import {
  Activity,
  AlertTriangle,
  CheckCircle2,
  Clock,
  Cloud,
  Gauge,
  HardDrive,
  Route,
  Timer,
  Wifi,
  WifiOff,
  XCircle,
} from 'lucide-react';
import { ServerTable } from '../components/dashboard/ServerTable';
import { RequestLog } from '../components/dashboard/RequestLog';
import { CloudActivityFeed } from '../components/dashboard/CloudActivityFeed';
import { LatencyHistogram } from '../components/metrics/LatencyHistogram';
import { TimeTravelWidget } from '../components/time-travel/TimeTravelWidget';
import { RealitySlider } from '../components/reality/RealitySlider';
import { RealityIndicator } from '../components/reality/RealityIndicator';
import { useRealityShortcuts } from '../hooks/useRealityShortcuts';
import { useDashboardStream } from '../hooks/useDashboardStream';
import type { CloudDashboardMetrics, DashboardData, LatencyMetrics, LogEntry } from '../types';
import { useDashboard, useLogs } from '../hooks/useApi';
import { isCloudMode } from '../utils/cloudMode';
import { PageHeader, MetricCard, Section, EmptyState } from '../components/ui/DesignSystem';
import { DashboardLoading, ErrorState } from '../components/ui/LoadingStates';
import { cn } from '../utils/cn';

const isCloud = isCloudMode();

function formatUptime(seconds: number): string {
  const days = Math.floor(seconds / 86400);
  const hours = Math.floor((seconds % 86400) / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  if (days > 0) return `${days}d ${hours}h`;
  if (hours > 0) return `${hours}h ${minutes}m`;
  return `${minutes}m`;
}

function formatBytes(bytes: number): string {
  if (bytes <= 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  const exp = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  const value = bytes / Math.pow(1024, exp);
  return `${value.toFixed(value >= 100 || exp === 0 ? 0 : 1)} ${units[exp]}`;
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

interface StatusCounts {
  total2xx: number;
  total4xx: number;
  total5xx: number;
}

function computeFailureCounters(logs: unknown): StatusCounts {
  if (!logs || !Array.isArray(logs)) return { total2xx: 0, total4xx: 0, total5xx: 0 };

  const validLogs = logs.filter(isLogEntry);
  return validLogs.reduce((acc: StatusCounts, log) => {
    const code = log.status_code;
    if (code === undefined) return acc;
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

  const sorted = [...responseTimes].sort((a, b) => a - b);
  const sum = responseTimes.reduce((acc, time) => acc + time, 0);
  const at = (q: number) => sorted[Math.min(sorted.length - 1, Math.floor(sorted.length * q))];

  const latencyData = responseTimes.reduce((acc: Record<string, number>, time) => {
    const rounded = Math.floor(time / 10) * 10;
    const range = `${rounded}-${rounded + 9}`;
    acc[range] = (acc[range] || 0) + 1;
    return acc;
  }, {});

  return [{
    service: 'MockForge',
    route: 'last 100 requests',
    avg_response_time: sum / responseTimes.length,
    min_response_time: sorted[0],
    max_response_time: sorted[sorted.length - 1],
    p50_response_time: at(0.5),
    p95_response_time: at(0.95),
    p99_response_time: at(0.99),
    total_requests: logEntries.length,
    histogram: Object.entries(latencyData)
      .sort(([a], [b]) => parseInt(a) - parseInt(b))
      .map(([range, count]) => ({ range, count }))
      .slice(0, 20),
  }];
}

/* -------------------------------------------------------------------------- */
/* Building blocks                                                            */
/* -------------------------------------------------------------------------- */

function Panel({
  title,
  description,
  action,
  children,
  className,
}: {
  title: string;
  description?: string;
  action?: ReactNode;
  children: ReactNode;
  className?: string;
}) {
  return (
    <section className={cn('rounded-xl border border-border bg-card shadow-xs', className)}>
      <header className="flex items-center justify-between gap-3 border-b border-border px-5 py-3.5">
        <div className="min-w-0">
          <h2 className="text-sm font-semibold text-foreground">{title}</h2>
          {description && <p className="text-xs text-muted-foreground">{description}</p>}
        </div>
        {action}
      </header>
      <div className="p-5">{children}</div>
    </section>
  );
}

function LiveBadge({ streaming, fetching }: { streaming: boolean; fetching: boolean }) {
  const Icon = isCloud ? Cloud : streaming ? Wifi : WifiOff;
  const label = isCloud ? 'Live' : streaming ? 'Streaming' : 'Polling';
  return (
    <span
      className="inline-flex h-7 items-center gap-1.5 rounded-full border border-border bg-bg-primary px-2.5 text-xs font-medium text-muted-foreground"
      title={streaming ? 'Receiving real-time updates' : 'Refreshing every few seconds'}
    >
      <span
        aria-hidden
        className={cn(
          'h-2 w-2 rounded-full',
          streaming || isCloud ? 'bg-success-500' : 'bg-warning-500',
          fetching && 'animate-pulse',
        )}
      />
      <Icon className="h-3 w-3" aria-hidden />
      {label}
    </span>
  );
}

const statusRows = [
  { key: 'total2xx', label: '2xx Success', icon: CheckCircle2, bar: 'bg-success-500', text: 'text-success-600 dark:text-success-400' },
  { key: 'total4xx', label: '4xx Client errors', icon: AlertTriangle, bar: 'bg-warning-500', text: 'text-warning-600 dark:text-warning-400' },
  { key: 'total5xx', label: '5xx Server errors', icon: XCircle, bar: 'bg-danger-500', text: 'text-danger-600 dark:text-danger-400' },
] as const;

function StatusDistribution({ counts, scope }: { counts: StatusCounts; scope: string }) {
  const total = counts.total2xx + counts.total4xx + counts.total5xx;
  const pct = (n: number) => (total === 0 ? 0 : (n / total) * 100);

  return (
    <Panel title="Response status" description={scope}>
      {total === 0 ? (
        <p className="py-6 text-center text-sm text-muted-foreground">No responses recorded yet.</p>
      ) : (
        <>
          <div
            className="flex h-2 w-full overflow-hidden rounded-full bg-muted"
            role="img"
            aria-label={statusRows.map((r) => `${r.label} ${pct(counts[r.key]).toFixed(1)}%`).join(', ')}
          >
            {statusRows.map((r) => (
              <div key={r.key} className={r.bar} style={{ width: `${pct(counts[r.key])}%` }} />
            ))}
          </div>
          <ul className="mt-4 divide-y divide-border">
            {statusRows.map((r) => {
              const Icon = r.icon;
              return (
                <li key={r.key} className="flex items-center gap-2.5 py-2 text-sm">
                  <Icon className={cn('h-4 w-4', r.text)} aria-hidden />
                  <span className="flex-1 text-foreground">{r.label}</span>
                  <span className="font-mono tabular-nums text-foreground">
                    {counts[r.key].toLocaleString()}
                  </span>
                  <span className="w-14 text-right font-mono text-xs tabular-nums text-muted-foreground">
                    {pct(counts[r.key]).toFixed(1)}%
                  </span>
                </li>
              );
            })}
          </ul>
        </>
      )}
    </Panel>
  );
}

type Health = { tone: 'ok' | 'warn' | 'bad'; title: string; detail: string };

/** Derive health from what the API actually reports — never a hardcoded "all good". */
function deriveHealth(dashboard: DashboardData, errorRate: number): Health {
  const servers = dashboard.servers ?? [];
  const stopped = servers.filter((s) => !s.running);
  if (servers.length > 0 && stopped.length === servers.length) {
    return { tone: 'bad', title: 'No servers running', detail: 'Every configured protocol server is stopped.' };
  }
  if (stopped.length > 0) {
    return {
      tone: 'warn',
      title: `${stopped.length} of ${servers.length} servers stopped`,
      detail: stopped.map((s) => s.server_type).join(', '),
    };
  }
  if (errorRate >= 5) {
    return { tone: 'warn', title: 'Elevated error rate', detail: `${errorRate.toFixed(1)}% of requests failed.` };
  }
  return {
    tone: 'ok',
    title: 'Healthy',
    detail: servers.length > 0 ? `${servers.length} servers running` : 'No issues reported',
  };
}

const healthStyles: Record<Health['tone'], { icon: typeof CheckCircle2; className: string }> = {
  ok: { icon: CheckCircle2, className: 'text-success-600 dark:text-success-400' },
  warn: { icon: AlertTriangle, className: 'text-warning-600 dark:text-warning-400' },
  bad: { icon: XCircle, className: 'text-danger-600 dark:text-danger-400' },
};

function SystemPanel({ rows, health }: { rows: { label: string; value: ReactNode }[]; health: Health }) {
  const HealthIcon = healthStyles[health.tone].icon;
  return (
    <Panel title="System">
      <div className="mb-4 flex items-start gap-2.5 rounded-lg border border-border bg-bg-secondary px-3 py-2.5">
        <HealthIcon className={cn('mt-0.5 h-4 w-4 shrink-0', healthStyles[health.tone].className)} aria-hidden />
        <div className="min-w-0">
          <p className="text-sm font-medium text-foreground">{health.title}</p>
          <p className="truncate text-xs text-muted-foreground">{health.detail}</p>
        </div>
      </div>
      <dl className="divide-y divide-border text-sm">
        {rows.map((row) => (
          <div key={row.label} className="flex items-center justify-between py-2">
            <dt className="text-muted-foreground">{row.label}</dt>
            <dd className="font-mono tabular-nums text-foreground">{row.value}</dd>
          </div>
        ))}
      </dl>
    </Panel>
  );
}

/* -------------------------------------------------------------------------- */
/* Page                                                                       */
/* -------------------------------------------------------------------------- */

const pageTitle = 'Dashboard';
const pageSubtitle = isCloud
  ? 'Traffic and health across your hosted deployments.'
  : 'Traffic, latency and health for this MockForge instance.';

export function DashboardPage() {
  // All hooks must be called unconditionally and in the same order every render
  const { data: dashboard, isLoading, error, isFetching, refetch } = useDashboard();
  // Refetch logs every 3 seconds for dashboard metrics to stay in sync with SSE updates.
  // Skipped in cloud mode — cloud has no per-request log stream; CloudActivityFeed
  // covers the audit/activity surface instead.
  const { data: logs } = useLogs({ limit: 100, refetchInterval: isCloud ? 0 : 3000 });

  // Real-time WebSocket updates patch the React Query cache; polling is the
  // fallback. /__mockforge/ws doesn't exist in cloud, so skip it there.
  const { connected: wsConnected } = useDashboardStream({ enabled: !isCloud });

  // Keyboard shortcuts for reality level changes (must run before early returns).
  useRealityShortcuts();

  if (isLoading) {
    return (
      <div>
        <PageHeader title={pageTitle} subtitle={pageSubtitle} />
        <DashboardLoading />
      </div>
    );
  }

  if (error) {
    return (
      <div>
        <PageHeader title={pageTitle} subtitle={pageSubtitle} />
        <ErrorState
          title="Failed to load dashboard"
          description="The dashboard API did not respond. Check that the admin server is running, then retry."
          error={error}
          retry={() => void refetch()}
        />
      </div>
    );
  }

  const system = dashboard?.system;

  if (!dashboard || !system) {
    return (
      <div>
        <PageHeader title={pageTitle} subtitle={pageSubtitle} />
        <div className="rounded-xl border border-border bg-card">
          <EmptyState
            icon={<Activity />}
            title="No dashboard data yet"
            description="The server is still starting up. Data appears here as soon as it reports in."
          />
        </div>
      </div>
    );
  }

  const cloudMetrics: CloudDashboardMetrics | undefined = dashboard.cloud_metrics;
  const failureCounters: StatusCounts = isCloud && cloudMetrics
    ? {
        total2xx: cloudMetrics.requests_2xx,
        total4xx: cloudMetrics.requests_4xx,
        total5xx: cloudMetrics.requests_5xx,
      }
    : computeFailureCounters(logs);
  // Buckets carry `range` labels rather than Prometheus `le` bounds; the
  // histogram only reads range/count.
  const latencyMetrics = computeLatencyMetrics(logs) as LatencyMetrics[];

  const totalRequests = dashboard.metrics?.total_requests ?? 0;
  const activeRequests = dashboard.metrics?.active_requests ?? 0;
  const errorRate = dashboard.metrics?.error_rate ?? 0;
  const avgResponseTime = dashboard.metrics?.average_response_time ?? 0;
  const health = deriveHealth(dashboard, errorRate);

  const kpis = isCloud
    ? [
        {
          title: 'Active deployments',
          value: (cloudMetrics?.active_deployments ?? 0).toLocaleString(),
          subtitle: `${cloudMetrics?.total_deployments ?? 0} total`,
          icon: <Cloud />,
        },
        {
          title: 'Requests',
          value: totalRequests.toLocaleString(),
          subtitle: cloudMetrics?.period_start ? `since ${cloudMetrics.period_start}` : 'this billing period',
          icon: <Activity />,
        },
        {
          title: 'Avg response time',
          value: `${Math.round(avgResponseTime)} ms`,
          subtitle: 'weighted across deployments',
          icon: <Timer />,
        },
        {
          title: 'Egress',
          value: formatBytes(cloudMetrics?.egress_bytes ?? 0),
          subtitle: `${errorRate.toFixed(1)}% error rate`,
          icon: <HardDrive />,
        },
      ]
    : [
        {
          title: 'Requests',
          value: totalRequests.toLocaleString(),
          subtitle: `${activeRequests} in flight`,
          icon: <Activity />,
        },
        {
          title: 'Error rate',
          value: `${errorRate.toFixed(1)}%`,
          subtitle: 'of all requests',
          icon: <AlertTriangle />,
        },
        {
          title: 'Avg response time',
          value: `${Math.round(avgResponseTime)} ms`,
          subtitle: 'across all routes',
          icon: <Timer />,
        },
        {
          title: 'Routes',
          value: system.total_routes.toLocaleString(),
          subtitle: `${system.total_fixtures} fixtures`,
          icon: <Route />,
        },
      ];

  const systemRows = isCloud
    ? [
        { label: 'Workspaces', value: cloudMetrics?.workspaces ?? 0 },
        { label: 'Services', value: cloudMetrics?.services ?? 0 },
        { label: 'Fixtures', value: cloudMetrics?.fixtures ?? 0 },
        { label: 'Federations', value: cloudMetrics?.federations ?? 0 },
        { label: 'Version', value: system.version },
      ]
    : [
        { label: 'Uptime', value: formatUptime(system.uptime_seconds) },
        { label: 'CPU', value: `${system.cpu_usage_percent.toFixed(1)}%` },
        { label: 'Memory', value: `${system.memory_usage_mb} MB` },
        { label: 'Threads', value: system.active_threads },
        { label: 'Version', value: system.version },
      ];

  return (
    <div className="flex flex-col gap-6">
      <PageHeader
        title={pageTitle}
        subtitle={pageSubtitle}
        className="mb-0"
        action={
          <>
            <LiveBadge streaming={wsConnected} fetching={isFetching} />
            {!isCloud && <RealityIndicator />}
          </>
        }
      />

      {/* KPIs */}
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-4">
        {kpis.map((kpi) => (
          <MetricCard key={kpi.title} {...kpi} />
        ))}
      </div>

      {/* Status + system */}
      <div className="grid grid-cols-1 gap-3 lg:grid-cols-3">
        <div className="lg:col-span-2">
          {isCloud ? (
            <StatusDistribution counts={failureCounters} scope="This billing period, all deployments" />
          ) : latencyMetrics.length > 0 ? (
            <LatencyHistogram metrics={latencyMetrics} selectedService={undefined} onServiceChange={() => {}} />
          ) : (
            <Panel title="Response time distribution">
              <EmptyState
                icon={<Gauge />}
                title="No latency data yet"
                description="Send a few requests to your mock and the distribution will appear here."
                className="py-8"
              />
            </Panel>
          )}
        </div>
        <SystemPanel rows={systemRows} health={health} />
      </div>

      {!isCloud && <StatusDistribution counts={failureCounters} scope="Last 100 requests" />}

      <Section
        title={isCloud ? 'Deployments & activity' : 'Servers & traffic'}
        subtitle={isCloud ? 'Active deployments and recent organization activity' : 'Protocol listeners and the live request stream'}
        className="py-0"
      >
        <div className="space-y-3">
          <ServerTable />
          {isCloud ? <CloudActivityFeed /> : <RequestLog />}
        </div>
      </Section>

      {/* Simulation controls — local only. In cloud, /__mockforge/reality and
          /__mockforge/time-travel are stubbed no-ops, so the controls would mislead. */}
      {!isCloud && (
        <Section
          title="Simulation"
          subtitle="Tune realism and control the virtual clock for this instance"
          className="py-0"
          action={
            <span className="hidden items-center gap-1 text-xs text-muted-foreground sm:inline-flex">
              <Clock className="h-3.5 w-3.5" aria-hidden />
              Changes apply immediately
            </span>
          }
        >
          <div className="grid grid-cols-1 gap-3 lg:grid-cols-2">
            <RealitySlider />
            <TimeTravelWidget />
          </div>
        </Section>
      )}
    </div>
  );
}
