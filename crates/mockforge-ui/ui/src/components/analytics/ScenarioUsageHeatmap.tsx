/**
 * Scenario Usage Heatmap
 *
 * Displays a heatmap of scenario usage patterns, showing which scenarios
 * are most frequently used and when they're accessed.
 */

import React, { useMemo } from 'react';
import { Card } from '../ui/Card';
import { TrendingUp, Calendar } from 'lucide-react';
import { useScenarioUsage } from '@/hooks/useCoverageMetrics';
import type { CoverageMetricsQuery } from '@/hooks/useCoverageMetrics';

interface ScenarioUsageHeatmapProps {
  workspaceId?: string;
  orgId?: string;
  limit?: number;
}

export const ScenarioUsageHeatmap: React.FC<ScenarioUsageHeatmapProps> = ({
  workspaceId,
  orgId,
  limit = 20,
}) => {
  const query: CoverageMetricsQuery = {
    workspace_id: workspaceId,
    org_id: orgId,
    limit,
  };

  const { data, isLoading, error } = useScenarioUsage(query);

  // Process data for heatmap visualization
  const heatmapData = useMemo(() => {
    if (!data || data.length === 0) return null;

    // Sort by usage count
    const sorted = [...data].sort((a, b) => b.usage_count - a.usage_count);
    const maxUsage = sorted[0]?.usage_count || 1;

    return {
      scenarios: sorted,
      maxUsage,
    };
  }, [data]);

  if (isLoading) {
    return (
      <Card className="p-6">
        <div className="flex items-center gap-2 mb-4">
          <TrendingUp className="h-5 w-5 text-muted-foreground" />
          <h3 className="text-lg font-semibold">Scenario Usage Heatmap</h3>
        </div>
        <div className="h-96 flex items-center justify-center">
          <div className="animate-pulse text-muted-foreground">Loading scenario usage...</div>
        </div>
      </Card>
    );
  }

  if (error || !heatmapData) {
    return (
      <Card className="p-6">
        <div className="flex items-center gap-2 mb-4">
          <TrendingUp className="h-5 w-5 text-muted-foreground" />
          <h3 className="text-lg font-semibold">Scenario Usage Heatmap</h3>
        </div>
        <div className="h-96 flex items-center justify-center text-muted-foreground">
          {error ? `Error: ${error.message}` : 'No scenario usage data available'}
        </div>
      </Card>
    );
  }

  const getColor = (usageCount: number, maxUsage: number) => {
    if (usageCount === 0) return 'bg-muted';
    const intensity = usageCount / maxUsage;
    if (intensity < 0.2) return 'bg-success-100 dark:bg-success-900/30';
    if (intensity < 0.4) return 'bg-success/30';
    if (intensity < 0.6) return 'bg-success/50';
    if (intensity < 0.8) return 'bg-success-400 dark:bg-success-600/60';
    return 'bg-success-500 dark:bg-success-500/70';
  };

  const formatDate = (timestamp?: number | null) => {
    if (!timestamp) return 'Never';
    return new Date(timestamp * 1000).toLocaleDateString();
  };

  return (
    <Card className="p-6">
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-2">
          <TrendingUp className="h-5 w-5 text-success-600 dark:text-success-400" />
          <h3 className="text-lg font-semibold">Scenario Usage Heatmap</h3>
        </div>
        <div className="text-sm text-muted-foreground">
          Top {heatmapData.scenarios.length} scenarios by usage
        </div>
      </div>

      <div className="space-y-2">
        {heatmapData.scenarios.map((scenario, index) => {
          const color = getColor(scenario.usage_count, heatmapData.maxUsage);
          const widthPercent = (scenario.usage_count / heatmapData.maxUsage) * 100;

          return (
            <div key={scenario.scenario_id || index} className="group">
              <div className="flex items-center gap-3 mb-1">
                <div className="flex-1 min-w-0">
                  <div className="text-sm font-medium text-foreground truncate">
                    {scenario.scenario_id}
                  </div>
                  <div className="text-xs text-muted-foreground">
                    Last used: {formatDate(scenario.last_used_at)}
                  </div>
                </div>
                <div className="text-sm font-semibold text-foreground">
                  {scenario.usage_count.toLocaleString()}
                </div>
              </div>
              <div className="relative h-6 bg-muted rounded-full overflow-hidden">
                <div
                  className={`h-full ${color} transition-all duration-300 rounded-full flex items-center justify-end pr-2`}
                  style={{ width: `${Math.max(widthPercent, 5)}%` }}
                >
                  {scenario.usage_count > 0 && (
                    <span className="text-xs font-medium text-white dark:text-foreground">
                      {scenario.usage_count.toLocaleString()}
                    </span>
                  )}
                </div>
              </div>
            </div>
          );
        })}
      </div>

      {/* Legend */}
      <div className="flex items-center justify-center gap-2 mt-6 text-xs text-muted-foreground">
        <span>Less</span>
        <div className="flex gap-1">
          <div className="w-4 h-4 bg-muted rounded" />
          <div className="w-4 h-4 bg-success-100 dark:bg-success-900/30 rounded" />
          <div className="w-4 h-4 bg-success/30 rounded" />
          <div className="w-4 h-4 bg-success/50 rounded" />
          <div className="w-4 h-4 bg-success-400 dark:bg-success-600/60 rounded" />
          <div className="w-4 h-4 bg-success-500 dark:bg-success-500/70 rounded" />
        </div>
        <span>More</span>
      </div>
    </Card>
  );
};
