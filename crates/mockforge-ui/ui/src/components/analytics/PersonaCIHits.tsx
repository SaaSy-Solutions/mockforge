/**
 * Persona CI Hits Component
 *
 * Displays which personas are being hit by CI runs, providing visibility
 * into test coverage and persona usage in automated testing.
 */

import React, { useMemo } from 'react';
import { Card } from '../ui/Card';
import { GitBranch, Clock, Activity } from 'lucide-react';
import { usePersonaCIHits } from '@/hooks/useCoverageMetrics';
import type { CoverageMetricsQuery } from '@/hooks/useCoverageMetrics';

interface PersonaCIHitsProps {
  workspaceId?: string;
  orgId?: string;
  limit?: number;
}

export const PersonaCIHits: React.FC<PersonaCIHitsProps> = ({
  workspaceId,
  orgId,
  limit = 50,
}) => {
  const query: CoverageMetricsQuery = {
    workspace_id: workspaceId,
    org_id: orgId,
    limit,
  };

  const { data, isLoading, error } = usePersonaCIHits(query);

  // Group by persona_id and aggregate hits
  const aggregatedData = useMemo(() => {
    if (!data || data.length === 0) return null;

    const personaMap = new Map<string, {
      persona_id: string;
      total_hits: number;
      ci_runs: Set<string>;
      last_hit: number;
    }>();

    data.forEach((hit) => {
      const existing = personaMap.get(hit.persona_id) || {
        persona_id: hit.persona_id,
        total_hits: 0,
        ci_runs: new Set<string>(),
        last_hit: 0,
      };

      existing.total_hits += hit.hit_count;
      if (hit.ci_run_id) {
        existing.ci_runs.add(hit.ci_run_id);
      }
      if (hit.hit_at > existing.last_hit) {
        existing.last_hit = hit.hit_at;
      }

      personaMap.set(hit.persona_id, existing);
    });

    return Array.from(personaMap.values())
      .sort((a, b) => b.total_hits - a.total_hits)
      .map((p) => ({
        ...p,
        unique_ci_runs: p.ci_runs.size,
      }));
  }, [data]);

  if (isLoading) {
    return (
      <Card className="p-6">
        <div className="flex items-center gap-2 mb-4">
          <GitBranch className="h-5 w-5 text-gray-400" />
          <h3 className="text-lg font-semibold">Persona CI Hits</h3>
        </div>
        <div className="h-64 flex items-center justify-center">
          <div className="animate-pulse text-gray-400">Loading CI hit data...</div>
        </div>
      </Card>
    );
  }

  if (error || !aggregatedData) {
    return (
      <Card className="p-6">
        <div className="flex items-center gap-2 mb-4">
          <GitBranch className="h-5 w-5 text-gray-400" />
          <h3 className="text-lg font-semibold">Persona CI Hits</h3>
        </div>
        <div className="h-64 flex items-center justify-center text-gray-400">
          {error ? `Error: ${error.message}` : 'No CI hit data available'}
        </div>
      </Card>
    );
  }

  const formatDate = (timestamp: number) => {
    return new Date(timestamp * 1000).toLocaleString();
  };

  return (
    <Card className="p-6">
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-2">
          <GitBranch className="h-5 w-5 text-blue-600 dark:text-blue-400" />
          <h3 className="text-lg font-semibold">Persona CI Hits</h3>
        </div>
        <div className="text-sm text-gray-500 dark:text-gray-400">
          {aggregatedData.length} personas hit by CI
        </div>
      </div>

      <div className="space-y-3">
        {aggregatedData.map((persona, index) => (
          <div
            key={persona.persona_id}
            className="p-4 border border-gray-200 dark:border-gray-700 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-800/50 transition-colors"
          >
            <div className="flex items-start justify-between mb-2">
              <div className="flex-1 min-w-0">
                <div className="text-sm font-semibold text-gray-900 dark:text-white truncate">
                  {persona.persona_id}
                </div>
                <div className="flex items-center gap-4 mt-2 text-xs text-gray-500 dark:text-gray-400">
                  <div className="flex items-center gap-1">
                    <Activity className="h-3 w-3" />
                    <span>{persona.total_hits.toLocaleString()} total hits</span>
                  </div>
                  <div className="flex items-center gap-1">
                    <GitBranch className="h-3 w-3" />
                    <span>{persona.unique_ci_runs} CI runs</span>
                  </div>
                </div>
              </div>
            </div>
            <div className="flex items-center gap-1 text-xs text-gray-400 dark:text-gray-500 mt-2">
              <Clock className="h-3 w-3" />
              <span>Last hit: {formatDate(persona.last_hit)}</span>
            </div>
          </div>
        ))}
      </div>

      {aggregatedData.length === 0 && (
        <div className="text-center py-8 text-gray-400">
          No personas have been hit by CI runs yet
        </div>
      )}
    </Card>
  );
};
