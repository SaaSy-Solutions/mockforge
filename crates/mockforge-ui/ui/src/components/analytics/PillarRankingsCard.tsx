/**
 * Pillar usage rankings card — shows pillars ordered by usage with most/least markers
 */

import React from 'react';
import { Card } from '../ui/Card';
import { TrendingUp, TrendingDown } from 'lucide-react';
import type { PillarUsageSummary } from '@/hooks/usePillarAnalytics';

interface PillarRankingsCardProps {
  data: PillarUsageSummary | null | undefined;
  isLoading?: boolean;
}

export const PillarRankingsCard: React.FC<PillarRankingsCardProps> = ({ data, isLoading }) => {
  if (isLoading) {
    return (
      <Card className="p-6 animate-pulse">
        <div className="h-6 bg-muted rounded w-48 mb-4" />
        <div className="space-y-3">
          {[0, 1, 2, 3, 4].map((i) => (
            <div key={i} className="h-8 bg-muted rounded" />
          ))}
        </div>
      </Card>
    );
  }

  if (!data || data.rankings.length === 0) {
    return (
      <Card className="p-6">
        <h2 className="text-xl font-semibold text-foreground mb-2">
          Pillar Usage Rankings
        </h2>
        <p className="text-sm text-muted-foreground">
          No pillar usage data recorded yet.
        </p>
      </Card>
    );
  }

  return (
    <Card className="p-6">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-xl font-semibold text-foreground">
          Pillar Usage Rankings
        </h2>
        <span className="text-sm text-muted-foreground">
          Total: {data.total_usage.toLocaleString()}
        </span>
      </div>

      <div className="space-y-3">
        {data.rankings.map((ranking, idx) => (
          <div key={ranking.pillar}>
            <div className="flex items-center justify-between mb-1">
              <div className="flex items-center gap-2">
                <span className="text-sm font-mono text-muted-foreground w-6">
                  #{idx + 1}
                </span>
                <span className="text-sm font-medium text-foreground">
                  {ranking.pillar}
                </span>
                {ranking.is_most_used && (
                  <span className="inline-flex items-center gap-1 text-xs px-2 py-0.5 rounded-full bg-success-100 dark:bg-success-900/30 text-success-700 dark:text-success-300">
                    <TrendingUp className="h-3 w-3" />
                    Most used
                  </span>
                )}
                {ranking.is_least_used && (
                  <span className="inline-flex items-center gap-1 text-xs px-2 py-0.5 rounded-full bg-amber-100 dark:bg-amber-900/30 text-amber-700 dark:text-amber-300">
                    <TrendingDown className="h-3 w-3" />
                    Least used
                  </span>
                )}
              </div>
              <div className="text-sm text-muted-foreground">
                {ranking.usage.toLocaleString()}{' '}
                <span className="text-xs">({ranking.percentage.toFixed(1)}%)</span>
              </div>
            </div>
            <div className="w-full bg-muted rounded-full h-2">
              <div
                className="bg-gradient-to-r from-blue-500 to-purple-500 h-2 rounded-full transition-all"
                style={{ width: `${Math.min(100, ranking.percentage)}%` }}
              />
            </div>
          </div>
        ))}
      </div>
    </Card>
  );
};
