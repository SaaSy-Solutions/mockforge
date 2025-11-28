/**
 * Reality Level Staleness Component
 *
 * Displays which mocks have stale reality levels, helping identify
 * endpoints that may need reality level updates.
 */

import React, { useMemo } from 'react';
import { Card } from '../ui/Card';
import { Clock, AlertTriangle, RefreshCw } from 'lucide-react';
import { useRealityLevelStaleness } from '@/hooks/useCoverageMetrics';
import type { CoverageMetricsQuery } from '@/hooks/useCoverageMetrics';

interface RealityLevelStalenessProps {
  workspaceId?: string;
  orgId?: string;
  maxStalenessDays?: number; // Show only items exceeding this threshold
}

export const RealityLevelStaleness: React.FC<RealityLevelStalenessProps> = ({
  workspaceId,
  orgId,
  maxStalenessDays,
}) => {
  const query: CoverageMetricsQuery = {
    workspace_id: workspaceId,
    org_id: orgId,
    max_staleness_days: maxStalenessDays,
  };

  const { data, isLoading, error } = useRealityLevelStaleness(query);

  // Categorize by staleness severity
  const categorizedData = useMemo(() => {
    if (!data || data.length === 0) return null;

    const categories = {
      critical: [] as typeof data, // > 90 days
      high: [] as typeof data, // 30-90 days
      medium: [] as typeof data, // 7-30 days
      low: [] as typeof data, // < 7 days
      unknown: [] as typeof data, // no staleness data
    };

    data.forEach((item) => {
      const days = item.staleness_days;
      if (days === null || days === undefined) {
        categories.unknown.push(item);
      } else if (days > 90) {
        categories.critical.push(item);
      } else if (days > 30) {
        categories.high.push(item);
      } else if (days > 7) {
        categories.medium.push(item);
      } else {
        categories.low.push(item);
      }
    });

    return categories;
  }, [data]);

  if (isLoading) {
    return (
      <Card className="p-6">
        <div className="flex items-center gap-2 mb-4">
          <Clock className="h-5 w-5 text-gray-400" />
          <h3 className="text-lg font-semibold">Reality Level Staleness</h3>
        </div>
        <div className="h-64 flex items-center justify-center">
          <div className="animate-pulse text-gray-400">Loading staleness data...</div>
        </div>
      </Card>
    );
  }

  if (error || !categorizedData) {
    return (
      <Card className="p-6">
        <div className="flex items-center gap-2 mb-4">
          <Clock className="h-5 w-5 text-gray-400" />
          <h3 className="text-lg font-semibold">Reality Level Staleness</h3>
        </div>
        <div className="h-64 flex items-center justify-center text-gray-400">
          {error ? `Error: ${error.message}` : 'No staleness data available'}
        </div>
      </Card>
    );
  }

  const totalItems = data?.length || 0;
  const formatDate = (timestamp?: number | null) => {
    if (!timestamp) return 'Unknown';
    return new Date(timestamp * 1000).toLocaleDateString();
  };

  const getStalenessColor = (days?: number | null) => {
    if (days === null || days === undefined) return 'text-gray-400';
    if (days > 90) return 'text-red-600 dark:text-red-400';
    if (days > 30) return 'text-orange-600 dark:text-orange-400';
    if (days > 7) return 'text-yellow-600 dark:text-yellow-400';
    return 'text-green-600 dark:text-green-400';
  };

  const getStalenessIcon = (days?: number | null) => {
    if (days === null || days === undefined) {
      return <AlertTriangle className="h-4 w-4 text-gray-500" />;
    }
    if (days > 30) {
      return <AlertTriangle className="h-4 w-4 text-red-500" />;
    }
    if (days > 7) {
      return <Clock className="h-4 w-4 text-yellow-500" />;
    }
    return <RefreshCw className="h-4 w-4 text-green-500" />;
  };

  return (
    <Card className="p-6">
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-2">
          <Clock className="h-5 w-5 text-orange-600 dark:text-orange-400" />
          <h3 className="text-lg font-semibold">Reality Level Staleness</h3>
        </div>
        <div className="text-sm text-gray-500 dark:text-gray-400">
          {totalItems} items tracked
        </div>
      </div>

      {/* Summary Stats */}
      <div className="grid grid-cols-2 md:grid-cols-5 gap-3 mb-6">
        <div className="p-3 bg-red-50 dark:bg-red-900/20 rounded-lg">
          <div className="text-2xl font-bold text-red-600 dark:text-red-400">
            {categorizedData.critical.length}
          </div>
          <div className="text-xs text-gray-600 dark:text-gray-400">Critical (&gt;90d)</div>
        </div>
        <div className="p-3 bg-orange-50 dark:bg-orange-900/20 rounded-lg">
          <div className="text-2xl font-bold text-orange-600 dark:text-orange-400">
            {categorizedData.high.length}
          </div>
          <div className="text-xs text-gray-600 dark:text-gray-400">High (30-90d)</div>
        </div>
        <div className="p-3 bg-yellow-50 dark:bg-yellow-900/20 rounded-lg">
          <div className="text-2xl font-bold text-yellow-600 dark:text-yellow-400">
            {categorizedData.medium.length}
          </div>
          <div className="text-xs text-gray-600 dark:text-gray-400">Medium (7-30d)</div>
        </div>
        <div className="p-3 bg-green-50 dark:bg-green-900/20 rounded-lg">
          <div className="text-2xl font-bold text-green-600 dark:text-green-400">
            {categorizedData.low.length}
          </div>
          <div className="text-xs text-gray-600 dark:text-gray-400">Low (&lt;7d)</div>
        </div>
        <div className="p-3 bg-gray-50 dark:bg-gray-800 rounded-lg">
          <div className="text-2xl font-bold text-gray-600 dark:text-gray-400">
            {categorizedData.unknown.length}
          </div>
          <div className="text-xs text-gray-600 dark:text-gray-400">Unknown</div>
        </div>
      </div>

      {/* Item List */}
      <div className="space-y-2 max-h-96 overflow-y-auto">
        {data?.map((item, index) => {
          const days = item.staleness_days;
          const method = item.method || 'ANY';
          const endpoint = item.endpoint || 'N/A';

          return (
            <div
              key={item.id || index}
              className="p-3 border border-gray-200 dark:border-gray-700 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-800/50 transition-colors"
            >
              <div className="flex items-start justify-between">
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2 mb-1">
                    {getStalenessIcon(days)}
                    <div className="text-sm font-medium text-gray-900 dark:text-white truncate">
                      {method} {endpoint}
                    </div>
                  </div>
                  <div className="text-xs text-gray-500 dark:text-gray-400">
                    {item.protocol || 'N/A'} • Level: {item.current_reality_level || 'Unknown'}
                  </div>
                </div>
                <div className="flex items-center gap-3 ml-4">
                  {days !== null && days !== undefined ? (
                    <div className={`text-sm font-semibold ${getStalenessColor(days)}`}>
                      {days} days
                    </div>
                  ) : (
                    <div className="text-sm font-semibold text-gray-400">Unknown</div>
                  )}
                </div>
              </div>
              {item.last_updated_at && (
                <div className="text-xs text-gray-400 dark:text-gray-500 mt-1">
                  Last updated: {formatDate(item.last_updated_at)}
                </div>
              )}
            </div>
          );
        })}
      </div>

      {totalItems === 0 && (
        <div className="text-center py-8 text-gray-400">
          No staleness data available
        </div>
      )}
    </Card>
  );
};
