import React, { useMemo } from 'react';
import { ServerTable } from '../components/dashboard/ServerTable';
import { RequestLog } from '../components/dashboard/RequestLog';
import { LatencyHistogram } from '../components/metrics/LatencyHistogram';
import type { LatencyMetrics, LogEntry } from '../types';
import { useDashboard, useLogs } from '../hooks/useApi';
import {
  PageHeader,
  MetricCard,
  Alert,
  Section
} from '../components/ui/DesignSystem';
import { MetricIcon, StatusIcon } from '../components/ui/IconSystem';
import { DashboardLoading, ErrorState } from '../components/ui/LoadingStates';

function formatUptime(seconds: number): string {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  if (hours > 0) {
    return `${hours}h ${minutes}m`;
  }
  return `${minutes}m`;
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

export function DashboardPage() {
  const { data: dashboard, isLoading, error } = useDashboard();
  // Refetch logs every 3 seconds for dashboard metrics to stay in sync with SSE updates
  const { data: logs } = useLogs({ limit: 100, refetchInterval: 3000 });

  // Calculate failure counters
  const failureCounters = useMemo(() => {
    if (!logs || !Array.isArray(logs)) return { total2xx: 0, total4xx: 0, total5xx: 0 };

    const validLogs = logs.filter(isLogEntry);
    return validLogs.reduce((acc: { total2xx: number; total4xx: number; total5xx: number }, log) => {
      const code = log.status_code;
      if (code >= 500) acc.total5xx++;
      else if (code >= 400) acc.total4xx++;
      else if (code >= 200) acc.total2xx++;
      return acc;
    }, { total2xx: 0, total4xx: 0, total5xx: 0 });
  }, [logs]);

  // Calculate latency metrics from logs
  const latencyMetrics = useMemo(() => {
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
  }, [logs]);

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

  return (
    <div className="content-width">
      <PageHeader
        title="Dashboard"
        subtitle="Real-time system overview and performance metrics"
        className="space-section"
      />

      <Section
        title="System Metrics"
        subtitle="Current system performance indicators"
        className="space-section section-breathing"
      >
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 grid-gap-lg">
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
        </div>

        {/* Failure Counters */}
        <div className="divider-soft my-8"></div>
        <div className="visual-group">
          <h3 className="text-lg font-bold text-gray-900 dark:text-gray-100 mb-4">Response Status Distribution</h3>
          <div className="grid grid-cols-1 md:grid-cols-3 grid-gap-md">
          <MetricCard
            title="Success Responses"
            value={failureCounters.total2xx.toString()}
            subtitle={`${safePercentage(failureCounters.total2xx, failureCounters.total2xx + failureCounters.total4xx + failureCounters.total5xx)}% of total`}
            icon={<StatusIcon status="success" size="lg" />}
            className="animate-fade-in-up animate-delay-100"
          />
          <MetricCard
            title="Client Errors"
            value={failureCounters.total4xx.toString()}
            subtitle={`${safePercentage(failureCounters.total4xx, failureCounters.total2xx + failureCounters.total4xx + failureCounters.total5xx)}% of total`}
            icon={<StatusIcon status="warning" size="lg" />}
            className="animate-fade-in-up animate-delay-200"
          />
          <MetricCard
            title="Server Errors"
            value={failureCounters.total5xx.toString()}
            subtitle={`${safePercentage(failureCounters.total5xx, failureCounters.total2xx + failureCounters.total4xx + failureCounters.total5xx)}% of total`}
            icon={<StatusIcon status="error" size="lg" />}
            className="animate-fade-in-up animate-delay-300"
          />
          </div>
        </div>
      </Section>

      <div className="divider-accent my-12"></div>

      {/* Performance Metrics Section */}
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

      <Section
        title="System Status"
        subtitle="Server instances and recent activity"
        className="space-section"
      >
        <div className="space-component">
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
