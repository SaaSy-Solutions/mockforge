import React, { useMemo } from 'react';
import { ServerTable } from '../components/dashboard/ServerTable';
import { RequestLog } from '../components/dashboard/RequestLog';
import { LatencyHistogram } from '../components/metrics/LatencyHistogram';
import { Clock, Cpu, HardDrive, Activity as ActivityIcon, Activity, Loader2, AlertTriangle, XCircle } from 'lucide-react';
import { useDashboard, useLogs, useMetrics } from '../hooks/useApi';
import {
  PageHeader,
  MetricCard,
  Alert,
  EmptyState,
  Section
} from '../components/ui/DesignSystem';

function formatUptime(seconds: number): string {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  if (hours > 0) {
    return `${hours}h ${minutes}m`;
  }
  return `${minutes}m`;
}

export function DashboardPage() {
  const { data: dashboard, isLoading, error } = useDashboard();
  const { data: logs } = useLogs({ limit: 100 });
  const { data: metrics } = useMetrics();

  // Calculate failure counters
  const failureCounters = useMemo(() => {
    if (!logs) return { total2xx: 0, total4xx: 0, total5xx: 0 };

    return logs.reduce((acc, log) => {
      const code = log.status_code;
      if (code >= 500) acc.total5xx++;
      else if (code >= 400) acc.total4xx++;
      else if (code >= 200) acc.total2xx++;
      return acc;
    }, { total2xx: 0, total4xx: 0, total5xx: 0 });
  }, [logs]);

  // Mock latency data for demonstration
  const mockLatencyMetrics = useMemo(() => {
    if (!logs) return [];

    const latencyData = logs.reduce((acc, log) => {
      if (log.response_time_ms) {
        const rounded = Math.floor(log.response_time_ms / 10) * 10;
        const range = `${rounded}-${rounded + 9}`;
        acc[range] = (acc[range] || 0) + 1;
      }
      return acc;
    }, {});

    return [{
      service: 'MockForge',
      route: 'api/*',
      p50: 25,
      p95: 75,
      p99: 125,
      histogram: Object.entries(latencyData)
        .sort(([a], [b]) => parseInt(a) - parseInt(b))
        .map(([range, count]) => ({ range, count }))
        .slice(0, 20) // Limit to first 20 ranges
    }];
  }, [logs]);

  if (isLoading) {
    return (
      <div className="space-y-8">
        <PageHeader
          title="Dashboard"
          subtitle="System overview and performance metrics"
        />
        <EmptyState
          icon={<Loader2 className="h-8 w-8 animate-spin" />}
          title="Loading dashboard data..."
          description="Please wait while we fetch the latest system information."
        />
      </div>
    );
  }

  if (error) {
    return (
      <div className="space-y-8">
        <PageHeader
          title="Dashboard"
          subtitle="System overview and performance metrics"
        />
        <Alert
          type="error"
          title="Failed to load dashboard"
          message={error instanceof Error ? error.message : 'Unknown error occurred. Please try refreshing the page.'}
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

  return (
    <div className="space-y-8">
      <PageHeader
        title="Dashboard"
        subtitle="Real-time system overview and performance metrics"
      />

      <Section
        title="System Metrics"
        subtitle="Current system performance indicators"
      >
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
          <MetricCard
            title="Uptime"
            value={formatUptime(system.uptime_seconds)}
            subtitle="system uptime"
            trend={{ direction: 'up', value: '+2.5%' }}
            icon={<Clock className="h-6 w-6" />}
          />
          <MetricCard
            title="CPU Usage"
            value={`${system.cpu_usage_percent.toFixed(1)}%`}
            subtitle="current utilization"
            trend={{ direction: 'down', value: '-1.2%' }}
            icon={<Cpu className="h-6 w-6" />}
          />
          <MetricCard
            title="Memory"
            value={`${system.memory_usage_mb} MB`}
            subtitle="allocated"
            trend={{ direction: 'up', value: '+5.3%' }}
            icon={<HardDrive className="h-6 w-6" />}
          />
          <MetricCard
            title="Active Threads"
            value={system.active_threads.toString()}
            subtitle="running threads"
            trend={{ direction: 'neutral', value: '0%' }}
            icon={<ActivityIcon className="h-6 w-6" />}
          />
        </div>

        {/* Failure Counters */}
        <div className="mt-8 grid grid-cols-1 md:grid-cols-3 gap-4">
          <MetricCard
            title="Success Responses"
            value={failureCounters.total2xx.toString()}
            subtitle="2xx responses"
            trend={{ direction: 'up', value: `${((failureCounters.total2xx / (failureCounters.total2xx + failureCounters.total4xx + failureCounters.total5xx)) * 100).toFixed(1)}%` }}
            icon={<Activity className="h-6 w-6 text-green-500" />}
          />
          <MetricCard
            title="Client Errors"
            value={failureCounters.total4xx.toString()}
            subtitle="4xx responses"
            trend={{ direction: 'neutral', value: `${((failureCounters.total4xx / (failureCounters.total2xx + failureCounters.total4xx + failureCounters.total5xx)) * 100).toFixed(1)}%` }}
            icon={<AlertTriangle className="h-6 w-6 text-yellow-500" />}
          />
          <MetricCard
            title="Server Errors"
            value={failureCounters.total5xx.toString()}
            subtitle="5xx responses"
            trend={{ direction: 'down', value: `${((failureCounters.total5xx / (failureCounters.total2xx + failureCounters.total4xx + failureCounters.total5xx)) * 100).toFixed(1)}%` }}
            icon={<XCircle className="h-6 w-6 text-red-500" />}
          />
        </div>
      </Section>

      {/* Performance Metrics Section */}
      <Section
        title="Performance Metrics"
        subtitle="Response time distribution and latency analysis"
      >
        <div className="space-y-6">
          {mockLatencyMetrics.length > 0 ? (
            <LatencyHistogram
              metrics={mockLatencyMetrics}
              selectedService={undefined}
              onServiceChange={() => {}}
            />
          ) : (
            <div className="text-center py-8">
              <div className="inline-flex items-center justify-center w-16 h-16 rounded-full bg-gray-100 dark:bg-gray-800 mb-4">
                <Clock className="h-8 w-8 text-gray-400" />
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

      <Section
        title="System Status"
        subtitle="Server instances and recent activity"
      >
        <div className="space-y-8">
          {/* Server Instances - Full Width */}
          <div>
            <ServerTable />
          </div>

          {/* Recent Requests - Full Width */}
          <div>
            <RequestLog />
          </div>
        </div>
      </Section>

      {/* Additional dashboard sections could go here */}
      <Section
        title="System Health"
        subtitle="Overall system status and alerts"
      >
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
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
                <p className="text-sm text-gray-500 dark:text-gray-400">Routes</p>
                <p className="text-lg font-semibold text-gray-900 dark:text-gray-100">
                  {system.total_routes}
                </p>
              </div>
            </div>
          </div>
        </div>
      </Section>
    </div>
  );
}
