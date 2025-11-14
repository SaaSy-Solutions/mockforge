import React, { useMemo } from 'react';
import { Clock, TrendingUp, Activity, AlertCircle, BarChart3 } from 'lucide-react';
import { useMetrics } from '../../hooks/useApi';
import { ModernCard, MetricCard, Section, EmptyState } from '../ui/DesignSystem';

/**
 * Performance Dashboard Component
 *
 * Displays comprehensive performance metrics including:
 * - Latency percentiles (p50, p75, p90, p95, p99, p99.9)
 * - Time-series latency charts
 * - Per-endpoint performance breakdown
 * - Error rates and request counts
 */
export function PerformanceDashboard() {
  const { data: metrics, isLoading, error } = useMetrics();

  // Process metrics data
  const processedData = useMemo(() => {
    if (!metrics?.data) return null;

    const percentiles = metrics.data.response_time_percentiles || {};
    const endpointPercentiles = metrics.data.endpoint_percentiles || {};
    const latencyOverTime = metrics.data.latency_over_time || [];
    const requestsByEndpoint = metrics.data.requests_by_endpoint || {};
    const errorRates = metrics.data.error_rate_by_endpoint || {};

    // Calculate average latency
    const avgLatency = latencyOverTime.length > 0
      ? Math.round(latencyOverTime.reduce((sum, [, latency]) => sum + latency, 0) / latencyOverTime.length)
      : 0;

    // Get top endpoints by request count
    const topEndpoints = Object.entries(requestsByEndpoint)
      .sort(([, a], [, b]) => (b as number) - (a as number))
      .slice(0, 10)
      .map(([endpoint]) => endpoint);

    return {
      percentiles,
      endpointPercentiles,
      latencyOverTime,
      requestsByEndpoint,
      errorRates,
      avgLatency,
      topEndpoints,
    };
  }, [metrics]);

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-12">
        <div className="text-center">
          <div className="inline-block animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600 mb-4"></div>
          <p className="text-gray-600 dark:text-gray-400">Loading performance metrics...</p>
        </div>
      </div>
    );
  }

  if (error || !processedData) {
    return (
      <EmptyState
        icon={<AlertCircle className="h-12 w-12" />}
        title="Unable to Load Metrics"
        message={error?.message || "Failed to load performance metrics. Please try again."}
      />
    );
  }

  const { percentiles, endpointPercentiles, latencyOverTime, avgLatency, topEndpoints } = processedData;

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold text-gray-900 dark:text-gray-100">Performance Dashboard</h2>
          <p className="text-sm text-gray-600 dark:text-gray-400 mt-1">
            Real-time latency analysis and performance metrics
          </p>
        </div>
      </div>

      {/* Key Metrics Overview */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <MetricCard
          title="Average Latency"
          value={`${avgLatency}ms`}
          subtitle="mean response time"
          icon={<Clock className="h-5 w-5" />}
        />
        <MetricCard
          title="P50 (Median)"
          value={`${percentiles.p50 || 0}ms`}
          subtitle="50th percentile"
          icon={<Activity className="h-5 w-5" />}
        />
        <MetricCard
          title="P95"
          value={`${percentiles.p95 || 0}ms`}
          subtitle="95th percentile"
          icon={<TrendingUp className="h-5 w-5" />}
        />
        <MetricCard
          title="P99"
          value={`${percentiles.p99 || 0}ms`}
          subtitle="99th percentile"
          icon={<BarChart3 className="h-5 w-5" />}
        />
      </div>

      {/* Latency Percentiles Chart */}
      <Section title="Latency Percentiles" subtitle="Response time distribution across percentiles">
        <ModernCard>
          <PercentileChart percentiles={percentiles} />
        </ModernCard>
      </Section>

      {/* Time-Series Latency Chart */}
      {latencyOverTime.length > 0 && (
        <Section title="Latency Over Time" subtitle="Response time trends">
          <ModernCard>
            <LatencyTimeSeriesChart data={latencyOverTime} />
          </ModernCard>
        </Section>
      )}

      {/* Per-Endpoint Performance */}
      {topEndpoints.length > 0 && (
        <Section title="Endpoint Performance" subtitle="Top endpoints by request volume">
          <ModernCard>
            <EndpointPerformanceTable
              endpoints={topEndpoints}
              endpointPercentiles={endpointPercentiles}
              requestsByEndpoint={processedData.requestsByEndpoint}
              errorRates={processedData.errorRates}
            />
          </ModernCard>
        </Section>
      )}
    </div>
  );
}

/**
 * Percentile Chart Component
 * Displays latency percentiles as a horizontal bar chart
 */
function PercentileChart({ percentiles }: { percentiles: Record<string, number> }) {
  const percentileLabels: Record<string, string> = {
    p50: 'P50 (Median)',
    p75: 'P75',
    p90: 'P90',
    p95: 'P95',
    p99: 'P99',
    p999: 'P99.9',
  };

  const percentileOrder = ['p50', 'p75', 'p90', 'p95', 'p99', 'p999'];
  const maxValue = Math.max(...Object.values(percentiles), 1);

  return (
    <div className="space-y-4">
      {percentileOrder.map((key) => {
        const value = percentiles[key] || 0;
        const percentage = maxValue > 0 ? (value / maxValue) * 100 : 0;
        const color = getPercentileColor(key);

        return (
          <div key={key} className="space-y-1">
            <div className="flex items-center justify-between text-sm">
              <span className="font-medium text-gray-700 dark:text-gray-300">
                {percentileLabels[key]}
              </span>
              <span className="text-gray-600 dark:text-gray-400 font-mono">
                {value}ms
              </span>
            </div>
            <div className="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-3">
              <div
                className={`h-3 rounded-full transition-all duration-500 ${color}`}
                style={{ width: `${percentage}%` }}
              />
            </div>
          </div>
        );
      })}
    </div>
  );
}

/**
 * Latency Time-Series Chart Component
 * Displays latency over time as a line chart
 */
function LatencyTimeSeriesChart({ data }: { data: Array<[string, number]> }) {
  if (data.length === 0) {
    return <p className="text-sm text-gray-500 dark:text-gray-400">No latency data available</p>;
  }

  // Convert timestamps to Date objects and extract values
  const points = data.map(([timestamp, latency]) => ({
    time: new Date(timestamp),
    latency,
  }));

  const values = points.map(p => p.latency);
  const min = Math.min(...values);
  const max = Math.max(...values);
  const range = max - min || 1;

  // Sample data points if too many (max 100 points for display)
  const displayPoints = points.length > 100
    ? points.filter((_, i) => i % Math.ceil(points.length / 100) === 0)
    : points;

  return (
    <div className="space-y-4">
      <div className="h-48 flex items-end gap-1">
        {displayPoints.map((point, index) => {
          const height = ((point.latency - min) / range) * 100;
          return (
            <div
              key={index}
              className="flex-1 bg-blue-500 hover:bg-blue-600 rounded-t-sm transition-all duration-300 cursor-pointer"
              style={{
                height: `${height}%`,
                minHeight: '2px',
              }}
              title={`${point.time.toLocaleTimeString()}: ${point.latency}ms`}
            />
          );
        })}
      </div>
      <div className="flex justify-between text-xs text-gray-500 dark:text-gray-400">
        <span>{displayPoints[0]?.time.toLocaleTimeString() || ''}</span>
        <span>{displayPoints[displayPoints.length - 1]?.time.toLocaleTimeString() || ''}</span>
      </div>
      <div className="flex justify-between text-xs text-gray-500 dark:text-gray-400">
        <span>Min: {min}ms</span>
        <span>Max: {max}ms</span>
        <span>Avg: {Math.round(values.reduce((a, b) => a + b, 0) / values.length)}ms</span>
      </div>
    </div>
  );
}

/**
 * Endpoint Performance Table Component
 * Displays per-endpoint latency percentiles and request counts
 */
function EndpointPerformanceTable({
  endpoints,
  endpointPercentiles,
  requestsByEndpoint,
  errorRates,
}: {
  endpoints: string[];
  endpointPercentiles: Record<string, Record<string, number>>;
  requestsByEndpoint: Record<string, number>;
  errorRates: Record<string, number>;
}) {
  return (
    <div className="overflow-x-auto">
      <table className="w-full text-sm">
        <thead>
          <tr className="border-b border-gray-200 dark:border-gray-700">
            <th className="text-left py-3 px-4 font-semibold text-gray-700 dark:text-gray-300">Endpoint</th>
            <th className="text-right py-3 px-4 font-semibold text-gray-700 dark:text-gray-300">Requests</th>
            <th className="text-right py-3 px-4 font-semibold text-gray-700 dark:text-gray-300">P50</th>
            <th className="text-right py-3 px-4 font-semibold text-gray-700 dark:text-gray-300">P95</th>
            <th className="text-right py-3 px-4 font-semibold text-gray-700 dark:text-gray-300">P99</th>
            <th className="text-right py-3 px-4 font-semibold text-gray-700 dark:text-gray-300">Error Rate</th>
          </tr>
        </thead>
        <tbody>
          {endpoints.map((endpoint) => {
            const percentiles = endpointPercentiles[endpoint] || {};
            const requestCount = requestsByEndpoint[endpoint] || 0;
            const errorRate = (errorRates[endpoint] || 0) * 100;

            return (
              <tr
                key={endpoint}
                className="border-b border-gray-100 dark:border-gray-800 hover:bg-gray-50 dark:hover:bg-gray-800/50"
              >
                <td className="py-3 px-4 font-mono text-xs text-gray-900 dark:text-gray-100">
                  {endpoint}
                </td>
                <td className="py-3 px-4 text-right text-gray-700 dark:text-gray-300">
                  {requestCount.toLocaleString()}
                </td>
                <td className="py-3 px-4 text-right font-mono text-gray-700 dark:text-gray-300">
                  {percentiles.p50 || 0}ms
                </td>
                <td className="py-3 px-4 text-right font-mono text-gray-700 dark:text-gray-300">
                  {percentiles.p95 || 0}ms
                </td>
                <td className="py-3 px-4 text-right font-mono text-gray-700 dark:text-gray-300">
                  {percentiles.p99 || 0}ms
                </td>
                <td className="py-3 px-4 text-right">
                  <span
                    className={`font-medium ${
                      errorRate < 1
                        ? 'text-green-600 dark:text-green-400'
                        : errorRate < 5
                        ? 'text-yellow-600 dark:text-yellow-400'
                        : 'text-red-600 dark:text-red-400'
                    }`}
                  >
                    {errorRate.toFixed(2)}%
                  </span>
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}

/**
 * Get color class for percentile based on value
 */
function getPercentileColor(percentile: string): string {
  const colors: Record<string, string> = {
    p50: 'bg-green-500',
    p75: 'bg-blue-500',
    p90: 'bg-yellow-500',
    p95: 'bg-orange-500',
    p99: 'bg-red-500',
    p999: 'bg-purple-500',
  };
  return colors[percentile] || 'bg-gray-500';
}
