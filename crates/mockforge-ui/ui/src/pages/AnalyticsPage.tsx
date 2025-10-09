import React, { useEffect } from 'react';
import { useAnalyticsStore, TimeRange } from '@/stores/useAnalyticsStore';
import { SummaryCards } from '@/components/analytics/SummaryCards';
import { RequestRateChart } from '@/components/analytics/RequestRateChart';
import { EndpointsTable } from '@/components/analytics/EndpointsTable';
import { WebSocketMetricsCard } from '@/components/analytics/WebSocketMetricsCard';
import { SystemMetricsCard } from '@/components/analytics/SystemMetricsCard';
import { exportEndpointsToCSV, exportAllAnalyticsToJSON } from '@/utils/exportData';
import { Download } from 'lucide-react';

export const AnalyticsPage: React.FC = () => {
  const store = useAnalyticsStore();
  const {
    summary,
    requests,
    endpoints,
    websocket,
    smtp,
    system,
    timeRange,
    isLoading,
    error,
    setTimeRange,
    fetchAll,
    clearError,
  } = store;

  const handleExportAll = () => {
    const exportData = {
      summary,
      requests,
      endpoints,
      websocket,
      smtp,
      system,
      timeRange,
      exportedAt: new Date().toISOString(),
    };
    exportAllAnalyticsToJSON(exportData);
  };

  useEffect(() => {
    fetchAll();
  }, [fetchAll]);

  const timeRanges: { value: TimeRange; label: string }[] = [
    { value: '5m', label: 'Last 5 minutes' },
    { value: '15m', label: 'Last 15 minutes' },
    { value: '1h', label: 'Last hour' },
    { value: '6h', label: 'Last 6 hours' },
    { value: '24h', label: 'Last 24 hours' },
  ];

  return (
    <div className="p-6 space-y-6">
      {/* Header */}
      <div className="flex justify-between items-center">
        <div>
          <h1 className="text-3xl font-bold">Analytics</h1>
          <p className="text-gray-600 dark:text-gray-400 mt-1">
            Real-time metrics and performance insights
          </p>
        </div>

        {/* Time Range Selector */}
        <div className="flex items-center gap-4">
          <label className="text-sm font-medium">Time Range:</label>
          <select
            value={timeRange}
            onChange={(e) => setTimeRange(e.target.value as TimeRange)}
            className="px-4 py-2 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 focus:ring-2 focus:ring-blue-500 focus:outline-none"
          >
            {timeRanges.map((range) => (
              <option key={range.value} value={range.value}>
                {range.label}
              </option>
            ))}
          </select>

          <button
            onClick={() => fetchAll()}
            disabled={isLoading}
            className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          >
            {isLoading ? 'Refreshing...' : 'Refresh'}
          </button>

          <button
            onClick={handleExportAll}
            disabled={isLoading || !summary}
            className="px-4 py-2 bg-green-600 text-white rounded-lg hover:bg-green-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors flex items-center gap-2"
          >
            <Download className="w-4 h-4" />
            Export All
          </button>
        </div>
      </div>

      {/* Error Message */}
      {error && (
        <div className="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg p-4 flex justify-between items-center">
          <div className="flex items-center gap-2">
            <svg
              className="w-5 h-5 text-red-600 dark:text-red-400"
              fill="currentColor"
              viewBox="0 0 20 20"
            >
              <path
                fillRule="evenodd"
                d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z"
                clipRule="evenodd"
              />
            </svg>
            <span className="text-red-800 dark:text-red-200">{error}</span>
          </div>
          <button
            onClick={clearError}
            className="text-red-600 dark:text-red-400 hover:text-red-800 dark:hover:text-red-200"
          >
            Dismiss
          </button>
        </div>
      )}

      {/* Summary Cards */}
      <SummaryCards data={summary} isLoading={isLoading} />

      {/* Request Rate Chart */}
      <RequestRateChart data={requests} isLoading={isLoading} />

      {/* Endpoints Table */}
      <EndpointsTable data={endpoints} isLoading={isLoading} />

      {/* WebSocket and System Metrics */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <WebSocketMetricsCard data={websocket} isLoading={isLoading} />
        <SystemMetricsCard data={system} isLoading={isLoading} />
      </div>

      {/* Last Updated Timestamp */}
      {!isLoading && summary && (
        <div className="text-center text-sm text-gray-500 dark:text-gray-400">
          Last updated: {new Date(summary.timestamp).toLocaleString()}
        </div>
      )}
    </div>
  );
};

export default AnalyticsPage;
