/**
 * Coverage Metrics Dashboard (MockOps)
 *
 * Main dashboard component that brings together all coverage metrics:
 * - Scenario usage heatmap
 * - Persona CI hits
 * - Endpoint coverage
 * - Reality level staleness
 * - Drift percentage
 */

import React, { useState } from 'react';
import { Card } from '../ui/Card';
import { BarChart3, Filter } from 'lucide-react';
import { ScenarioUsageHeatmap } from './ScenarioUsageHeatmap';
import { PersonaCIHits } from './PersonaCIHits';
import { EndpointCoverage } from './EndpointCoverage';
import { RealityLevelStaleness } from './RealityLevelStaleness';
import { DriftPercentageDashboard } from './DriftPercentageDashboard';

export interface CoverageMetricsDashboardProps {
  workspaceId?: string;
  orgId?: string;
}

export const CoverageMetricsDashboard: React.FC<CoverageMetricsDashboardProps> = ({
  workspaceId,
  orgId,
}) => {
  const [showFilters, setShowFilters] = useState(false);
  const [minCoverage, setMinCoverage] = useState<number | undefined>(undefined);
  const [maxStalenessDays, setMaxStalenessDays] = useState<number | undefined>(undefined);

  return (
    <div className="space-y-6 p-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold text-gray-900 dark:text-white">
            Coverage Metrics Dashboard
          </h1>
          <p className="text-sm text-gray-600 dark:text-gray-400 mt-1">
            Track scenario usage, test coverage, and mock health across your workspaces
          </p>
        </div>
        <button
          onClick={() => setShowFilters(!showFilters)}
          className="flex items-center gap-2 px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors"
        >
          <Filter className="h-4 w-4" />
          <span>Filters</span>
        </button>
      </div>

      {/* Filters Panel */}
      {showFilters && (
        <Card className="p-4">
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                Minimum Coverage (%)
              </label>
              <input
                type="number"
                min="0"
                max="100"
                value={minCoverage ?? ''}
                onChange={(e) =>
                  setMinCoverage(e.target.value ? parseFloat(e.target.value) : undefined)
                }
                placeholder="Show only endpoints below this coverage"
                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-white"
              />
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                Max Staleness (days)
              </label>
              <input
                type="number"
                min="0"
                value={maxStalenessDays ?? ''}
                onChange={(e) =>
                  setMaxStalenessDays(e.target.value ? parseInt(e.target.value) : undefined)
                }
                placeholder="Show only items exceeding this threshold"
                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-white"
              />
            </div>
          </div>
        </Card>
      )}

      {/* Drift Percentage - Prominent Display */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <DriftPercentageDashboard workspaceId={workspaceId} orgId={orgId} />
        <div className="flex items-center justify-center p-6 border-2 border-dashed border-gray-300 dark:border-gray-600 rounded-lg">
          <div className="text-center text-gray-400">
            <BarChart3 className="h-12 w-12 mx-auto mb-2 opacity-50" />
            <p className="text-sm">Additional metrics placeholder</p>
          </div>
        </div>
      </div>

      {/* Scenario Usage and Persona CI Hits */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <ScenarioUsageHeatmap workspaceId={workspaceId} orgId={orgId} />
        <PersonaCIHits workspaceId={workspaceId} orgId={orgId} />
      </div>

      {/* Endpoint Coverage */}
      <EndpointCoverage workspaceId={workspaceId} orgId={orgId} minCoverage={minCoverage} />

      {/* Reality Level Staleness */}
      <RealityLevelStaleness
        workspaceId={workspaceId}
        orgId={orgId}
        maxStalenessDays={maxStalenessDays}
      />
    </div>
  );
};
