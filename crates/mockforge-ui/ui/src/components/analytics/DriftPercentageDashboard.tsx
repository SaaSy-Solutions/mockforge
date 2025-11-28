/**
 * Drift Percentage Dashboard
 *
 * Displays the percentage of mocks that are drifting from real data,
 * providing a high-level view of mock health across workspaces.
 */

import React, { useMemo } from 'react';
import { Card } from '../ui/Card';
import { TrendingDown, TrendingUp, AlertCircle, CheckCircle2 } from 'lucide-react';
import { useDriftPercentage } from '@/hooks/useCoverageMetrics';
import type { CoverageMetricsQuery } from '@/hooks/useCoverageMetrics';

interface DriftPercentageDashboardProps {
  workspaceId?: string;
  orgId?: string;
  limit?: number;
}

export const DriftPercentageDashboard: React.FC<DriftPercentageDashboardProps> = ({
  workspaceId,
  orgId,
  limit = 10,
}) => {
  const query: CoverageMetricsQuery = {
    workspace_id: workspaceId,
    org_id: orgId,
    limit,
  };

  const { data, isLoading, error } = useDriftPercentage(query);

  // Get the most recent measurement
  const latestData = useMemo(() => {
    if (!data || data.length === 0) return null;
    return data[0]; // Data is sorted by measured_at DESC
  }, [data]);

  // Calculate aggregate statistics
  const aggregateStats = useMemo(() => {
    if (!data || data.length === 0) return null;

    const totalMocks = data.reduce((sum, item) => sum + item.total_mocks, 0);
    const totalDrifting = data.reduce((sum, item) => sum + item.drifting_mocks, 0);
    const avgDriftPercentage = totalMocks > 0 ? (totalDrifting / totalMocks) * 100 : 0;

    return {
      totalMocks,
      totalDrifting,
      avgDriftPercentage,
      workspaceCount: data.length,
    };
  }, [data]);

  if (isLoading) {
    return (
      <Card className="p-6">
        <div className="flex items-center gap-2 mb-4">
          <TrendingDown className="h-5 w-5 text-gray-400" />
          <h3 className="text-lg font-semibold">Drift Percentage</h3>
        </div>
        <div className="h-64 flex items-center justify-center">
          <div className="animate-pulse text-gray-400">Loading drift metrics...</div>
        </div>
      </Card>
    );
  }

  if (error || !latestData) {
    return (
      <Card className="p-6">
        <div className="flex items-center gap-2 mb-4">
          <TrendingDown className="h-5 w-5 text-gray-400" />
          <h3 className="text-lg font-semibold">Drift Percentage</h3>
        </div>
        <div className="h-64 flex items-center justify-center text-gray-400">
          {error ? `Error: ${error.message}` : 'No drift data available'}
        </div>
      </Card>
    );
  }

  const formatDate = (timestamp: number) => {
    return new Date(timestamp * 1000).toLocaleString();
  };

  const getDriftColor = (percentage: number) => {
    if (percentage < 5) return 'text-green-600 dark:text-green-400';
    if (percentage < 15) return 'text-yellow-600 dark:text-yellow-400';
    if (percentage < 30) return 'text-orange-600 dark:text-orange-400';
    return 'text-red-600 dark:text-red-400';
  };

  const getDriftIcon = (percentage: number) => {
    if (percentage < 5) {
      return <CheckCircle2 className="h-5 w-5 text-green-500" />;
    }
    if (percentage < 15) {
      return <AlertCircle className="h-5 w-5 text-yellow-500" />;
    }
    return <TrendingDown className="h-5 w-5 text-red-500" />;
  };

  const getDriftStatus = (percentage: number) => {
    if (percentage < 5) return 'Excellent';
    if (percentage < 15) return 'Good';
    if (percentage < 30) return 'Warning';
    return 'Critical';
  };

  return (
    <Card className="p-6">
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-2">
          <TrendingDown className="h-5 w-5 text-red-600 dark:text-red-400" />
          <h3 className="text-lg font-semibold">Drift Percentage</h3>
        </div>
        <div className="text-sm text-gray-500 dark:text-gray-400">
          Last updated: {formatDate(latestData.measured_at)}
        </div>
      </div>

      {/* Main Metric */}
      <div className="mb-6">
        <div className="flex items-center justify-between mb-2">
          <div className="flex items-center gap-2">
            {getDriftIcon(latestData.drift_percentage)}
            <span className="text-sm font-medium text-gray-700 dark:text-gray-300">
              Current Drift
            </span>
          </div>
          <div className={`text-3xl font-bold ${getDriftColor(latestData.drift_percentage)}`}>
            {latestData.drift_percentage.toFixed(1)}%
          </div>
        </div>
        <div className="text-xs text-gray-500 dark:text-gray-400 mb-4">
          Status: <span className="font-semibold">{getDriftStatus(latestData.drift_percentage)}</span>
        </div>
        <div className="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-3 overflow-hidden">
          <div
            className={`h-full transition-all duration-500 ${
              latestData.drift_percentage < 5
                ? 'bg-green-500'
                : latestData.drift_percentage < 15
                  ? 'bg-yellow-500'
                  : latestData.drift_percentage < 30
                    ? 'bg-orange-500'
                    : 'bg-red-500'
            }`}
            style={{ width: `${Math.min(latestData.drift_percentage, 100)}%` }}
          />
        </div>
        <div className="flex justify-between text-xs text-gray-500 dark:text-gray-400 mt-1">
          <span>{latestData.drifting_mocks.toLocaleString()} drifting</span>
          <span>{latestData.total_mocks.toLocaleString()} total mocks</span>
        </div>
      </div>

      {/* Aggregate Stats */}
      {aggregateStats && aggregateStats.workspaceCount > 1 && (
        <div className="p-4 bg-gray-50 dark:bg-gray-800/50 rounded-lg mb-6">
          <div className="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-2">
            Aggregate Statistics ({aggregateStats.workspaceCount} workspaces)
          </div>
          <div className="grid grid-cols-3 gap-4 text-sm">
            <div>
              <div className="text-gray-500 dark:text-gray-400">Total Mocks</div>
              <div className="text-lg font-bold text-gray-900 dark:text-white">
                {aggregateStats.totalMocks.toLocaleString()}
              </div>
            </div>
            <div>
              <div className="text-gray-500 dark:text-gray-400">Drifting</div>
              <div className="text-lg font-bold text-red-600 dark:text-red-400">
                {aggregateStats.totalDrifting.toLocaleString()}
              </div>
            </div>
            <div>
              <div className="text-gray-500 dark:text-gray-400">Avg Drift</div>
              <div className={`text-lg font-bold ${getDriftColor(aggregateStats.avgDriftPercentage)}`}>
                {aggregateStats.avgDriftPercentage.toFixed(1)}%
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Historical Trend (if multiple data points) */}
      {data && data.length > 1 && (
        <div className="mt-6">
          <div className="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-3">
            Recent Trend
          </div>
          <div className="space-y-2">
            {data.slice(0, 5).map((item, index) => (
              <div
                key={item.id || index}
                className="flex items-center justify-between p-2 border border-gray-200 dark:border-gray-700 rounded"
              >
                <div className="text-xs text-gray-500 dark:text-gray-400">
                  {formatDate(item.measured_at)}
                </div>
                <div className="flex items-center gap-2">
                  <div className={`text-sm font-semibold ${getDriftColor(item.drift_percentage)}`}>
                    {item.drift_percentage.toFixed(1)}%
                  </div>
                  <div className="text-xs text-gray-400 dark:text-gray-500">
                    ({item.drifting_mocks}/{item.total_mocks})
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </Card>
  );
};
