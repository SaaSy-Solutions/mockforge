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
          <Target className="h-5 w-5 text-muted-foreground" />
          <h3 className="text-lg font-semibold">Endpoint Coverage</h3>
        </div>
        <div className="h-64 flex items-center justify-center">
          <div className="animate-pulse text-muted-foreground">Loading coverage data...</div>
        </div>
      </Card>
    );
  }

  if (error || !categorizedData) {
    return (
      <Card className="p-6">
        <div className="flex items-center gap-2 mb-4">
          <Target className="h-5 w-5 text-muted-foreground" />
          <h3 className="text-lg font-semibold">Endpoint Coverage</h3>
        </div>
        <div className="h-64 flex items-center justify-center text-muted-foreground">
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
    if (coverage >= 90) return 'text-success-600 dark:text-success-400';
    if (coverage >= 70) return 'text-info-600 dark:text-info-400';
    if (coverage >= 50) return 'text-warning-600 dark:text-warning-400';
    if (coverage > 0) return 'text-orange-600 dark:text-orange-400';
    return 'text-danger-600 dark:text-danger-400';
  };

  const getCoverageIcon = (coverage?: number | null) => {
    if (coverage === null || coverage === undefined || coverage === 0) {
      return <AlertCircle className="h-4 w-4 text-danger-500" />;
    }
    if (coverage >= 70) {
      return <CheckCircle2 className="h-4 w-4 text-success-500" />;
    }
    return <AlertCircle className="h-4 w-4 text-warning-500" />;
  };

  return (
    <Card className="p-6">
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-2">
          <Target className="h-5 w-5 text-purple-600 dark:text-purple-400" />
          <h3 className="text-lg font-semibold">Endpoint Coverage</h3>
        </div>
        <div className="text-sm text-muted-foreground">
          {totalEndpoints} endpoints tracked
        </div>
      </div>

      {/* Summary Stats */}
      <div className="grid grid-cols-2 md:grid-cols-5 gap-3 mb-6">
        <div className="p-3 bg-success-50 dark:bg-success-900/20 rounded-lg">
          <div className="text-2xl font-bold text-success-600 dark:text-success-400">
            {categorizedData.excellent.length}
          </div>
          <div className="text-xs text-muted-foreground">Excellent (≥90%)</div>
        </div>
        <div className="p-3 bg-info-50 dark:bg-info-900/20 rounded-lg">
          <div className="text-2xl font-bold text-info-600 dark:text-info-400">
            {categorizedData.good.length}
          </div>
          <div className="text-xs text-muted-foreground">Good (70-89%)</div>
        </div>
        <div className="p-3 bg-warning-50 dark:bg-warning-900/20 rounded-lg">
          <div className="text-2xl font-bold text-warning-600 dark:text-warning-400">
            {categorizedData.fair.length}
          </div>
          <div className="text-xs text-muted-foreground">Fair (50-69%)</div>
        </div>
        <div className="p-3 bg-orange-50 dark:bg-orange-900/20 rounded-lg">
          <div className="text-2xl font-bold text-orange-600 dark:text-orange-400">
            {categorizedData.poor.length}
          </div>
          <div className="text-xs text-muted-foreground">Poor (&lt;50%)</div>
        </div>
        <div className="p-3 bg-danger-50 dark:bg-danger-900/20 rounded-lg">
          <div className="text-2xl font-bold text-danger-600 dark:text-danger-400">
            {categorizedData.untested.length}
          </div>
          <div className="text-xs text-muted-foreground">Untested</div>
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
              className="p-3 border border-border rounded-lg hover:bg-accent hover:text-accent-foreground/50 transition-colors"
            >
              <div className="flex items-start justify-between">
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2 mb-1">
                    {getCoverageIcon(coverage)}
                    <div className="text-sm font-medium text-foreground truncate">
                      {method} {endpoint.endpoint}
                    </div>
                  </div>
                  <div className="text-xs text-muted-foreground">
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
                <div className="text-xs text-muted-foreground mt-1">
                  Last tested: {formatDate(endpoint.last_tested_at)}
                </div>
              )}
            </div>
          );
        })}
      </div>

      {totalEndpoints === 0 && (
        <div className="text-center py-8 text-muted-foreground">
          No endpoint coverage data available
        </div>
      )}
    </Card>
  );
};
