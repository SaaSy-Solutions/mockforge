import React, { useMemo } from 'react';
import { BarChart3, TrendingUp, Clock, Activity, Zap } from 'lucide-react';
import { useMetrics } from '../hooks/useApi';
import {
  PageHeader,
  ModernCard,
  MetricCard,
  Alert,
  EmptyState,
  Section,
  ModernBadge
} from '../components/ui/DesignSystem';

// Simple chart component using CSS
function SimpleBarChart({ data, title }: { data: Array<{ label: string; value: number; color: string }>; title: string }) {
  const maxValue = Math.max(...data.map(d => d.value));

  if (data.length === 0 || maxValue === 0) {
    return (
      <div className="space-y-4">
        <h3 className="text-lg font-semibold text-gray-900 dark:text-gray-100">{title}</h3>
        <p className="text-sm text-gray-500 dark:text-gray-400">No data available</p>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <h3 className="text-lg font-semibold text-gray-900 dark:text-gray-100">{title}</h3>
      <div className="space-y-3">
        {data.map((item, index) => (
          <div key={index} className="flex items-center gap-4">
            <div className="w-24 text-sm text-gray-600 dark:text-gray-400 truncate">
              {item.label}
            </div>
            <div className="flex-1">
              <div className="flex items-center gap-2">
                <div className="flex-1 bg-gray-200 dark:bg-gray-700 rounded-full h-3">
                  <div
                    className={`h-3 rounded-full transition-all duration-500 ${item.color}`}
                    style={{ width: `${(item.value / maxValue) * 100}%` }}
                  />
                </div>
                <span className="text-sm font-medium text-gray-900 dark:text-gray-100 min-w-[3rem] text-right">
                  {item.value}
                </span>
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

function SimpleLineChart({ data, title }: { data: Array<{ timestamp: string; value: number }>; title: string }) {
  if (data.length === 0) return null;

  const values = data.map(d => d.value);
  const min = Math.min(...values);
  const max = Math.max(...values);
  const range = max - min || 1;

  return (
    <div className="space-y-4">
      <h3 className="text-lg font-semibold text-gray-900 dark:text-gray-100">{title}</h3>
      <div className="h-32 flex items-end gap-1">
        {data.map((point, index) => (
          <div
            key={index}
            className="flex-1 bg-blue-500 rounded-t-sm transition-all duration-300"
            style={{
              height: `${((point.value - min) / range) * 100}%`,
              minHeight: '4px'
            }}
            title={`${new Date(point.timestamp).toLocaleTimeString()}: ${point.value}`}
          />
        ))}
      </div>
      <div className="flex justify-between text-xs text-gray-500 dark:text-gray-400">
        <span>{new Date(data[0]?.timestamp).toLocaleTimeString()}</span>
        <span>{new Date(data[data.length - 1]?.timestamp).toLocaleTimeString()}</span>
      </div>
    </div>
  );
}

export function MetricsPage() {
  const { data: metrics, isLoading, error } = useMetrics();

  // Helper function to safely convert to number
  const toNumber = (value: unknown): number => {
    if (typeof value === 'number') return value;
    if (typeof value === 'string') return parseFloat(value) || 0;
    return 0;
  };

  // Process metrics data with memoization
  const processedMetrics = useMemo(() => {
    if (!metrics) return null;

    const endpointData = Object.entries(metrics.requests_by_endpoint || {}).map(([endpoint, count]) => ({
      label: endpoint.split(' ')[1] || endpoint, // Extract path from "METHOD /path"
      value: toNumber(count),
      color: 'bg-blue-500'
    }));

    const responseTimeData = [
      { label: 'P50', value: toNumber(metrics.response_time_percentiles?.['p50']), color: 'bg-green-500' },
      { label: 'P95', value: toNumber(metrics.response_time_percentiles?.['p95']), color: 'bg-yellow-500' },
      { label: 'P99', value: toNumber(metrics.response_time_percentiles?.['p99']), color: 'bg-red-500' },
    ];

    const errorRateData = Object.entries(metrics.error_rate_by_endpoint || {}).map(([endpoint, rate]) => {
      const errorRate = toNumber(rate);
      return {
        label: endpoint.split(' ')[1] || endpoint,
        value: Math.round(errorRate * 100),
        color: errorRate > 0.1 ? 'bg-red-500' : errorRate > 0.05 ? 'bg-yellow-500' : 'bg-green-500'
      };
    });

    // Calculate KPI metrics safely
    const requestsByEndpoint = Object.values(metrics.requests_by_endpoint || {}).map(toNumber);
    const totalRequests = requestsByEndpoint.reduce((a, b) => a + b, 0);

    const responseTimeValues = Object.values(metrics.response_time_percentiles || {}).map(toNumber);
    const avgResponseTime = responseTimeValues.length > 0
      ? Math.round(responseTimeValues.reduce((a, b) => a + b, 0) / responseTimeValues.length)
      : 0;

    const errorRates = Object.values(metrics.error_rate_by_endpoint || {}).map(toNumber);
    const avgErrorRate = errorRates.length > 0
      ? (errorRates.reduce((a, b) => a + b, 0) / errorRates.length * 100)
      : 0;

    const activeEndpoints = Object.keys(metrics.requests_by_endpoint || {}).length;

    // Process time series data safely
    const memoryData = (metrics.memory_usage_over_time || [])
      .filter((item): item is [string, number] => Array.isArray(item) && item.length === 2)
      .map(([timestamp, value]: [string, number]) => ({
        timestamp: String(timestamp),
        value: toNumber(value)
      }));

    const cpuData = (metrics.cpu_usage_over_time || [])
      .filter((item): item is [string, number] => Array.isArray(item) && item.length === 2)
      .map(([timestamp, value]: [string, number]) => ({
        timestamp: String(timestamp),
        value: toNumber(value)
      }));

    return {
      endpointData,
      responseTimeData,
      errorRateData,
      totalRequests,
      avgResponseTime,
      avgErrorRate,
      activeEndpoints,
      memoryData,
      cpuData
    };
  }, [metrics]);

  if (isLoading) {
    return (
      <div className="space-y-8">
        <PageHeader
          title="Performance Metrics"
          subtitle="Monitor system performance and request analytics"
        />
        <EmptyState
          icon={<BarChart3 className="h-12 w-12" />}
          title="Loading metrics..."
          description="Fetching performance data and analytics."
        />
      </div>
    );
  }

  if (error) {
    return (
      <div className="space-y-8">
        <PageHeader
          title="Performance Metrics"
          subtitle="Monitor system performance and request analytics"
        />
        <Alert
          type="error"
          title="Failed to load metrics"
          message={error instanceof Error ? error.message : 'Unable to fetch performance metrics. Please try again.'}
        />
      </div>
    );
  }

  if (!metrics) {
    return (
      <div className="space-y-8">
        <PageHeader
          title="Performance Metrics"
          subtitle="Monitor system performance and request analytics"
        />
        <Alert
          type="warning"
          title="No metrics available"
          message="Metrics data is not available. Ensure the MockForge server is running and collecting metrics."
        />
      </div>
    );
  }

  if (!processedMetrics) {
    return (
      <div className="space-y-8">
        <PageHeader
          title="Performance Metrics"
          subtitle="Monitor system performance and request analytics"
        />
        <Alert
          type="warning"
          title="Processing metrics..."
          message="Processing metrics data."
        />
      </div>
    );
  }

  return (
    <div className="space-y-8">
      <PageHeader
        title="Performance Metrics"
        subtitle="Real-time system performance and request analytics"
      />

      {/* Key Metrics Overview */}
      <Section
        title="Key Performance Indicators"
        subtitle="Critical system metrics at a glance"
      >
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
          <MetricCard
            title="Total Requests"
            value={processedMetrics.totalRequests.toLocaleString()}
            subtitle="all endpoints"
            icon={<Activity className="h-6 w-6" />}
          />
          <MetricCard
            title="Avg Response Time"
            value={`${processedMetrics.avgResponseTime}ms`}
            subtitle="median"
            icon={<Clock className="h-6 w-6" />}
          />
          <MetricCard
            title="Error Rate"
            value={`${processedMetrics.avgErrorRate.toFixed(1)}%`}
            subtitle="average"
            icon={<TrendingUp className="h-6 w-6" />}
          />
          <MetricCard
            title="Active Endpoints"
            value={processedMetrics.activeEndpoints.toString()}
            subtitle="with traffic"
            icon={<Zap className="h-6 w-6" />}
          />
        </div>
      </Section>

      {/* Charts and Analytics */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-8">
        {/* Request Distribution */}
        <Section title="Request Distribution" subtitle="Traffic breakdown by endpoint">
          <ModernCard>
            {processedMetrics.endpointData.length > 0 ? (
              <SimpleBarChart data={processedMetrics.endpointData} title="Requests by Endpoint" />
            ) : (
              <EmptyState
                icon={<BarChart3 className="h-8 w-8" />}
                title="No request data"
                description="Start making API calls to see request distribution."
              />
            )}
          </ModernCard>
        </Section>

        {/* Response Time Percentiles */}
        <Section title="Response Time Analysis" subtitle="Latency percentiles across all requests">
          <ModernCard>
            <SimpleBarChart data={processedMetrics.responseTimeData} title="Response Time Percentiles (ms)" />
          </ModernCard>
        </Section>

        {/* Error Rate Analysis */}
        <Section title="Error Rate Analysis" subtitle="Error rates by endpoint">
          <ModernCard>
            {processedMetrics.errorRateData.length > 0 ? (
              <SimpleBarChart data={processedMetrics.errorRateData} title="Error Rates by Endpoint (%)" />
            ) : (
              <EmptyState
                icon={<TrendingUp className="h-8 w-8" />}
                title="No error data"
                description="Error rates will appear here when requests fail."
              />
            )}
          </ModernCard>
        </Section>

        {/* Time Series Charts */}
        <Section title="System Resource Usage" subtitle="Memory and CPU usage over time">
          <ModernCard>
            <div className="space-y-6">
              {processedMetrics.memoryData.length > 0 && (
                <SimpleLineChart
                  data={processedMetrics.memoryData}
                  title="Memory Usage (MB)"
                />
              )}
              {processedMetrics.cpuData.length > 0 && (
                <SimpleLineChart
                  data={processedMetrics.cpuData}
                  title="CPU Usage (%)"
                />
              )}
              {(!processedMetrics.memoryData.length && !processedMetrics.cpuData.length) && (
                <EmptyState
                  icon={<Activity className="h-8 w-8" />}
                  title="No time series data"
                  description="System metrics will appear here over time."
                />
              )}
            </div>
          </ModernCard>
        </Section>
      </div>

      {/* Detailed Metrics Table */}
      <Section title="Endpoint Performance" subtitle="Detailed performance metrics for each endpoint">
        <ModernCard>
          <div className="overflow-x-auto">
            <table className="w-full">
              <caption className="sr-only">Endpoint performance metrics showing requests, error rates, and health status</caption>
              <thead>
                <tr className="border-b border-gray-200 dark:border-gray-700">
                  <th scope="col" className="text-left py-3 px-4 font-semibold text-gray-900 dark:text-gray-100">Endpoint</th>
                  <th scope="col" className="text-right py-3 px-4 font-semibold text-gray-900 dark:text-gray-100">Requests</th>
                  <th scope="col" className="text-right py-3 px-4 font-semibold text-gray-900 dark:text-gray-100">Error Rate</th>
                  <th scope="col" className="text-right py-3 px-4 font-semibold text-gray-900 dark:text-gray-100">Status</th>
                </tr>
              </thead>
              <tbody>
                {Object.entries(metrics.requests_by_endpoint || {}).map(([endpoint, requestCount]) => {
                  const errorRate = toNumber(metrics.error_rate_by_endpoint?.[endpoint]);
                  const requests = toNumber(requestCount);
                  return (
                    <tr key={endpoint} className="border-b border-gray-100 dark:border-gray-800 hover:bg-gray-50 dark:hover:bg-gray-800/50">
                      <td className="py-3 px-4 font-mono text-sm text-gray-900 dark:text-gray-100">
                        {endpoint}
                      </td>
                      <td className="py-3 px-4 text-right text-gray-900 dark:text-gray-100">
                        {requests.toLocaleString()}
                      </td>
                      <td className="py-3 px-4 text-right">
                        <ModernBadge
                          variant={errorRate > 0.1 ? 'error' : errorRate > 0.05 ? 'warning' : 'success'}
                          size="sm"
                        >
                          {(errorRate * 100).toFixed(1)}%
                        </ModernBadge>
                      </td>
                      <td className="py-3 px-4 text-right">
                        <ModernBadge
                          variant={errorRate === 0 ? 'success' : 'warning'}
                          size="sm"
                        >
                          {errorRate === 0 ? 'Healthy' : 'Issues'}
                        </ModernBadge>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        </ModernCard>
      </Section>
    </div>
  );
}
