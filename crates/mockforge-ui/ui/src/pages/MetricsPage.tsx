import React, { useState } from 'react';
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
                    style={{ width: `${maxValue > 0 ? (item.value / maxValue) * 100 : 0}%` }}
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
  const [timeRange, setTimeRange] = useState<'1h' | '6h' | '24h' | '7d'>('1h');

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

  // Process metrics data
  const endpointData = Object.entries(metrics.requests_by_endpoint).map(([endpoint, count]) => ({
    label: endpoint.split(' ')[1] || endpoint, // Extract path from "METHOD /path"
    value: count,
    color: 'bg-blue-500'
  }));

  const responseTimeData = [
    { label: 'P50', value: metrics.response_time_percentiles['p50'] || 0, color: 'bg-green-500' },
    { label: 'P95', value: metrics.response_time_percentiles['p95'] || 0, color: 'bg-yellow-500' },
    { label: 'P99', value: metrics.response_time_percentiles['p99'] || 0, color: 'bg-red-500' },
  ];

  const errorRateData = Object.entries(metrics.error_rate_by_endpoint).map(([endpoint, rate]) => ({
    label: endpoint.split(' ')[1] || endpoint,
    value: Math.round(rate * 100),
    color: rate > 0.1 ? 'bg-red-500' : rate > 0.05 ? 'bg-yellow-500' : 'bg-green-500'
  }));

  return (
    <div className="space-y-8">
      <PageHeader
        title="Performance Metrics"
        subtitle="Real-time system performance and request analytics"
        action={
          <div className="flex items-center gap-2">
            <span className="text-sm text-gray-600 dark:text-gray-400">Time Range:</span>
            <select
              value={timeRange}
              onChange={(e) => setTimeRange(e.target.value as typeof timeRange)}
              className="px-3 py-1 text-sm border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100"
            >
              <option value="1h">Last Hour</option>
              <option value="6h">Last 6 Hours</option>
              <option value="24h">Last 24 Hours</option>
              <option value="7d">Last 7 Days</option>
            </select>
          </div>
        }
      />

      {/* Key Metrics Overview */}
      <Section
        title="Key Performance Indicators"
        subtitle="Critical system metrics at a glance"
      >
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
          <MetricCard
            title="Total Requests"
            value={Object.values(metrics.requests_by_endpoint).reduce((a, b) => a + b, 0).toLocaleString()}
            subtitle="all endpoints"
            icon={<Activity className="h-6 w-6" />}
          />
          <MetricCard
            title="Avg Response Time"
            value={`${Math.round(Object.values(metrics.response_time_percentiles).reduce((a, b) => a + b, 0) / Object.keys(metrics.response_time_percentiles).length)}ms`}
            subtitle="median"
            icon={<Clock className="h-6 w-6" />}
          />
          <MetricCard
            title="Error Rate"
            value={`${(Object.values(metrics.error_rate_by_endpoint).reduce((a, b) => a + b, 0) / Object.keys(metrics.error_rate_by_endpoint).length * 100).toFixed(1)}%`}
            subtitle="average"
            trend={{ direction: 'down', value: '-2.1%' }}
            icon={<TrendingUp className="h-6 w-6" />}
          />
          <MetricCard
            title="Active Endpoints"
            value={Object.keys(metrics.requests_by_endpoint).length.toString()}
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
            {endpointData.length > 0 ? (
              <SimpleBarChart data={endpointData} title="Requests by Endpoint" />
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
            <SimpleBarChart data={responseTimeData} title="Response Time Percentiles (ms)" />
          </ModernCard>
        </Section>

        {/* Error Rate Analysis */}
        <Section title="Error Rate Analysis" subtitle="Error rates by endpoint">
          <ModernCard>
            {errorRateData.length > 0 ? (
              <SimpleBarChart data={errorRateData} title="Error Rates by Endpoint (%)" />
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
              {metrics.memory_usage_over_time.length > 0 && (
                <SimpleLineChart
                  data={metrics.memory_usage_over_time.map(([timestamp, value]) => ({
                    timestamp,
                    value
                  }))}
                  title="Memory Usage (MB)"
                />
              )}
              {metrics.cpu_usage_over_time.length > 0 && (
                <SimpleLineChart
                  data={metrics.cpu_usage_over_time.map(([timestamp, value]) => ({
                    timestamp,
                    value
                  }))}
                  title="CPU Usage (%)"
                />
              )}
              {(!metrics.memory_usage_over_time.length && !metrics.cpu_usage_over_time.length) && (
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
              <thead>
                <tr className="border-b border-gray-200 dark:border-gray-700">
                  <th className="text-left py-3 px-4 font-semibold text-gray-900 dark:text-gray-100">Endpoint</th>
                  <th className="text-right py-3 px-4 font-semibold text-gray-900 dark:text-gray-100">Requests</th>
                  <th className="text-right py-3 px-4 font-semibold text-gray-900 dark:text-gray-100">Error Rate</th>
                  <th className="text-right py-3 px-4 font-semibold text-gray-900 dark:text-gray-100">Status</th>
                </tr>
              </thead>
              <tbody>
                {Object.entries(metrics.requests_by_endpoint).map(([endpoint, requestCount]) => {
                  const errorRate = metrics.error_rate_by_endpoint[endpoint] || 0;
                  return (
                    <tr key={endpoint} className="border-b border-gray-100 dark:border-gray-800 hover:bg-gray-50 dark:hover:bg-gray-800/50">
                      <td className="py-3 px-4 font-mono text-sm text-gray-900 dark:text-gray-100">
                        {endpoint}
                      </td>
                      <td className="py-3 px-4 text-right text-gray-900 dark:text-gray-100">
                        {requestCount.toLocaleString()}
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
