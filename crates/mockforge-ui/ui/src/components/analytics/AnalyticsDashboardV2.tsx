/**
 * Enhanced Analytics Dashboard (V2)
 * Uses the new persistent analytics database with real-time updates
 */

import React, { useState } from 'react';
import { Card } from '../ui/Card';
import { useOverviewMetrics } from '@/hooks/useAnalyticsV2';
import { useAnalyticsStream } from '@/hooks/useAnalyticsStream';
import { OverviewCards } from './OverviewCards';
import { LatencyTrendChart } from './LatencyTrendChart';
import { RequestTimeSeriesChart } from './RequestTimeSeriesChart';
import { ErrorDashboard } from './ErrorDashboard';
import { TrafficHeatmap } from './TrafficHeatmap';
import { FilterPanel } from './FilterPanel';
import { ExportButton } from './ExportButton';
import type { AnalyticsFilter } from '@/hooks/useAnalyticsV2';

export const AnalyticsDashboardV2: React.FC = () => {
  const [filter, setFilter] = useState<AnalyticsFilter>({
    duration: 3600, // Last hour by default
    granularity: 'minute',
  });

  const [liveUpdatesEnabled, setLiveUpdatesEnabled] = useState(true);

  // Fetch overview metrics (with auto-refresh if live updates disabled)
  const { data: overview, isLoading, error } = useOverviewMetrics(filter, {
    refetchInterval: liveUpdatesEnabled ? false : 30000, // 30s when WS disabled
  });

  // WebSocket for real-time updates
  const { isConnected, lastUpdate } = useAnalyticsStream({
    enabled: liveUpdatesEnabled,
    config: {
      interval_seconds: 5,
      duration_seconds: filter.duration || 3600,
      protocol: filter.protocol,
      endpoint: filter.endpoint,
      workspace_id: filter.workspace_id,
    },
  });

  // Use WebSocket data if available, otherwise use REST API data
  // Note: MetricsUpdate from WebSocket has fewer fields than OverviewMetrics,
  // so we prefer OverviewMetrics when available, but allow MetricsUpdate for live updates
  const currentMetrics = overview || (lastUpdate ? {
    total_requests: lastUpdate.total_requests,
    total_errors: lastUpdate.total_errors,
    error_rate: lastUpdate.error_rate,
    avg_latency_ms: lastUpdate.avg_latency_ms,
    p95_latency_ms: lastUpdate.p95_latency_ms,
    p99_latency_ms: lastUpdate.p99_latency_ms,
    active_connections: lastUpdate.active_connections,
    total_bytes_sent: 0, // Not in MetricsUpdate
    total_bytes_received: 0, // Not in MetricsUpdate
    requests_per_second: lastUpdate.requests_per_second,
    top_protocols: [], // Not in MetricsUpdate
    top_endpoints: [], // Not in MetricsUpdate
  } : null) as OverviewMetrics | null;

  return (
    <div className="space-y-6 p-6">
      {/* Header with title and controls */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold text-gray-900 dark:text-white">
            Analytics Dashboard
          </h1>
          <p className="text-sm text-gray-600 dark:text-gray-400 mt-1">
            Comprehensive traffic analytics and metrics visualization
          </p>
        </div>

        <div className="flex items-center gap-4">
          {/* Live updates toggle */}
          <div className="flex items-center gap-2">
            <label className="text-sm font-medium text-gray-700 dark:text-gray-300">
              Live Updates
            </label>
            <button
              onClick={() => setLiveUpdatesEnabled(!liveUpdatesEnabled)}
              className={`
                relative inline-flex h-6 w-11 items-center rounded-full
                transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2
                ${liveUpdatesEnabled ? 'bg-blue-600' : 'bg-gray-200 dark:bg-gray-700'}
              `}
            >
              <span
                className={`
                  inline-block h-4 w-4 transform rounded-full bg-white transition-transform
                  ${liveUpdatesEnabled ? 'translate-x-6' : 'translate-x-1'}
                `}
              />
            </button>
            {isConnected && (
              <span className="flex items-center gap-1 text-xs text-green-600 dark:text-green-400">
                <span className="h-2 w-2 rounded-full bg-green-600 animate-pulse" />
                Connected
              </span>
            )}
          </div>

          {/* Export button */}
          <ExportButton filter={filter} />
        </div>
      </div>

      {/* Filter panel */}
      <FilterPanel filter={filter} onChange={setFilter} />

      {/* Error display */}
      {error && (
        <Card className="p-4 bg-red-50 dark:bg-red-900/20 border-red-200 dark:border-red-800">
          <p className="text-sm text-red-600 dark:text-red-400">
            Error loading analytics: {error.message}
          </p>
        </Card>
      )}

      {/* Overview cards */}
      <OverviewCards data={currentMetrics} isLoading={isLoading} />

      {/* Charts grid */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Request time series */}
        <RequestTimeSeriesChart filter={filter} />

        {/* Latency trends */}
        <LatencyTrendChart filter={filter} />
      </div>

      {/* Error dashboard */}
      <ErrorDashboard filter={filter} />

      {/* Traffic heatmap */}
      <TrafficHeatmap days={7} workspace_id={filter.workspace_id} />
    </div>
  );
};
