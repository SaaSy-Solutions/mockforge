/**
 * Endpoint Coverage Component
 *
 * Displays endpoint test coverage metrics, showing which endpoints
 * are well-tested and which need more test coverage.
 */

import React, { useMemo } from 'react';
import { Card } from '../ui/Card';
import { Target, AlertCircle, CheckCircle2 } from 'lucide-react';
import { useEndpointCoverage } from '@/hooks/useCoverageMetrics';
import type { CoverageMetricsQuery } from '@/hooks/useCoverageMetrics';

interface EndpointCoverageProps {
  workspaceId?: string;
  orgId?: string;
  minCoverage?: number; // Show only endpoints below this coverage threshold
}

export const EndpointCoverage: React.FC<EndpointCoverageProps> = ({
  workspaceId,
  orgId,
  minCoverage,
}) => {
  const query: CoverageMetricsQuery = {
    workspace_id: workspaceId,
    org_id: orgId,
    min_coverage: minCoverage,
  };

  const { data, isLoading, error } = useEndpointCoverage(query);

  // Categorize endpoints by coverage level
  const categorizedData = useMemo(() => {
    if (!data || data.length === 0) return null;

    const categories = {
      excellent: [] as typeof data,
      good: [] as typeof data,
      fair: [] as typeof data,
      poor: [] as typeof data,
      untested: [] as typeof data,
    };

    data.forEach((endpoint) => {
      const coverage = endpoint.coverage_percentage ?? 0;
      if (coverage >= 90) {
        categories.excellent.push(endpoint);
      } else if (coverage >= 70) {
        categories.good.push(endpoint);
      } else if (coverage >= 50) {
        categories.fair.push(endpoint);
      } else if (coverage > 0) {
        categories.poor.push(endpoint);
      } else {
        categories.untested.push(endpoint);
      }
    });

    return categories;
  }, [data]);

  if (isLoading) {
    return (
      <Card className="p-6">
        <div className="flex items-center gap-2 mb-4">
          <Target className="h-5 w-5 text-gray-400" />
          <h3 className="text-lg font-semibold">Endpoint Coverage</h3>
        </div>
        <div className="h-64 flex items-center justify-center">
          <div className="animate-pulse text-gray-400">Loading coverage data...</div>
        </div>
      </Card>
    );
  }

  if (error || !categorizedData) {
    return (
      <Card className="p-6">
        <div className="flex items-center gap-2 mb-4">
          <Target className="h-5 w-5 text-gray-400" />
          <h3 className="text-lg font-semibold">Endpoint Coverage</h3>
        </div>
        <div className="h-64 flex items-center justify-center text-gray-400">
          {error ? `Error: ${error.message}` : 'No coverage data available'}
        </div>
      </Card>
    );
  }

  const totalEndpoints = data?.length || 0;
  const formatDate = (timestamp?: number | null) => {
    if (!timestamp) return 'Never';
    return new Date(timestamp * 1000).toLocaleDateString();
  };

  const getCoverageColor = (coverage?: number | null) => {
    if (coverage === null || coverage === undefined) return 'text-gray-400';
    if (coverage >= 90) return 'text-green-600 dark:text-green-400';
    if (coverage >= 70) return 'text-blue-600 dark:text-blue-400';
    if (coverage >= 50) return 'text-yellow-600 dark:text-yellow-400';
    if (coverage > 0) return 'text-orange-600 dark:text-orange-400';
    return 'text-red-600 dark:text-red-400';
  };

  const getCoverageIcon = (coverage?: number | null) => {
    if (coverage === null || coverage === undefined || coverage === 0) {
      return <AlertCircle className="h-4 w-4 text-red-500" />;
    }
    if (coverage >= 70) {
      return <CheckCircle2 className="h-4 w-4 text-green-500" />;
    }
    return <AlertCircle className="h-4 w-4 text-yellow-500" />;
  };

  return (
    <Card className="p-6">
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-2">
          <Target className="h-5 w-5 text-purple-600 dark:text-purple-400" />
          <h3 className="text-lg font-semibold">Endpoint Coverage</h3>
        </div>
        <div className="text-sm text-gray-500 dark:text-gray-400">
          {totalEndpoints} endpoints tracked
        </div>
      </div>

      {/* Summary Stats */}
      <div className="grid grid-cols-2 md:grid-cols-5 gap-3 mb-6">
        <div className="p-3 bg-green-50 dark:bg-green-900/20 rounded-lg">
          <div className="text-2xl font-bold text-green-600 dark:text-green-400">
            {categorizedData.excellent.length}
          </div>
          <div className="text-xs text-gray-600 dark:text-gray-400">Excellent (≥90%)</div>
        </div>
        <div className="p-3 bg-blue-50 dark:bg-blue-900/20 rounded-lg">
          <div className="text-2xl font-bold text-blue-600 dark:text-blue-400">
            {categorizedData.good.length}
          </div>
          <div className="text-xs text-gray-600 dark:text-gray-400">Good (70-89%)</div>
        </div>
        <div className="p-3 bg-yellow-50 dark:bg-yellow-900/20 rounded-lg">
          <div className="text-2xl font-bold text-yellow-600 dark:text-yellow-400">
            {categorizedData.fair.length}
          </div>
          <div className="text-xs text-gray-600 dark:text-gray-400">Fair (50-69%)</div>
        </div>
        <div className="p-3 bg-orange-50 dark:bg-orange-900/20 rounded-lg">
          <div className="text-2xl font-bold text-orange-600 dark:text-orange-400">
            {categorizedData.poor.length}
          </div>
          <div className="text-xs text-gray-600 dark:text-gray-400">Poor (&lt;50%)</div>
        </div>
        <div className="p-3 bg-red-50 dark:bg-red-900/20 rounded-lg">
          <div className="text-2xl font-bold text-red-600 dark:text-red-400">
            {categorizedData.untested.length}
          </div>
          <div className="text-xs text-gray-600 dark:text-gray-400">Untested</div>
        </div>
      </div>

      {/* Endpoint List */}
      <div className="space-y-2 max-h-96 overflow-y-auto">
        {data?.map((endpoint, index) => {
          const coverage = endpoint.coverage_percentage ?? 0;
          const method = endpoint.method || 'ANY';

          return (
            <div
              key={endpoint.id || index}
              className="p-3 border border-gray-200 dark:border-gray-700 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-800/50 transition-colors"
            >
              <div className="flex items-start justify-between">
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2 mb-1">
                    {getCoverageIcon(coverage)}
                    <div className="text-sm font-medium text-gray-900 dark:text-white truncate">
                      {method} {endpoint.endpoint}
                    </div>
                  </div>
                  <div className="text-xs text-gray-500 dark:text-gray-400">
                    {endpoint.protocol} • {endpoint.test_count} tests
                  </div>
                </div>
                <div className="flex items-center gap-3 ml-4">
                  <div className={`text-sm font-semibold ${getCoverageColor(coverage)}`}>
                    {coverage > 0 ? `${coverage.toFixed(1)}%` : '0%'}
                  </div>
                </div>
              </div>
              {endpoint.last_tested_at && (
                <div className="text-xs text-gray-400 dark:text-gray-500 mt-1">
                  Last tested: {formatDate(endpoint.last_tested_at)}
                </div>
              )}
            </div>
          );
        })}
      </div>

      {totalEndpoints === 0 && (
        <div className="text-center py-8 text-gray-400">
          No endpoint coverage data available
        </div>
      )}
    </Card>
  );
};
