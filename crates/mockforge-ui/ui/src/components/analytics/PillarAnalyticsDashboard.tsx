/**
 * Pillar Usage Analytics Dashboard
 *
 * Displays usage metrics for MockForge's five foundational pillars:
 * - Reality: Blended reality %, Smart Personas vs static fixtures
 * - Contracts: Validation modes, drift budgets
 * - DevX: SDK usage, playground sessions
 * - Cloud: Shared scenarios, marketplace usage
 * - AI: AI-generated mocks, voice commands
 */

import React, { useState } from 'react';
import { Card } from '../ui/Card';
import { usePillarMetrics } from '@/hooks/usePillarAnalytics';
import { PillarOverviewCards } from './PillarOverviewCards';
import { PillarUsageChart } from './PillarUsageChart';
import { RealityPillarDetails } from './RealityPillarDetails';
import { ContractsPillarDetails } from './ContractsPillarDetails';
import { TimeRangeSelector } from './TimeRangeSelector';
import type { PillarMetricsQuery } from '@/hooks/usePillarAnalytics';

export interface PillarAnalyticsDashboardProps {
  workspaceId?: string;
  orgId?: string;
}

export const PillarAnalyticsDashboard: React.FC<PillarAnalyticsDashboardProps> = ({
  workspaceId,
  orgId,
}) => {
  const [timeRange, setTimeRange] = useState<string>('7d');
  const [selectedPillar, setSelectedPillar] = useState<string | null>(null);

  const query: PillarMetricsQuery = {
    time_range: timeRange,
  };

  const { data: metrics, isLoading, error } = usePillarMetrics(
    workspaceId,
    orgId,
    query
  );

  if (error) {
    return (
      <div className="p-6">
        <Card className="p-6 border-red-200 bg-red-50 dark:bg-red-900/20">
          <h2 className="text-lg font-semibold text-red-800 dark:text-red-200 mb-2">
            Error Loading Pillar Analytics
          </h2>
          <p className="text-red-600 dark:text-red-300">
            {error instanceof Error ? error.message : 'Failed to load pillar usage metrics'}
          </p>
        </Card>
      </div>
    );
  }

  return (
    <div className="space-y-6 p-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold text-gray-900 dark:text-white">
            Pillar Usage Analytics
          </h1>
          <p className="text-sm text-gray-600 dark:text-gray-400 mt-1">
            Track adoption of MockForge's foundational pillars across your workspaces
          </p>
        </div>

        <TimeRangeSelector value={timeRange} onChange={setTimeRange} />
      </div>

      {/* Overview Cards */}
      <PillarOverviewCards data={metrics} isLoading={isLoading} />

      {/* Pillar Usage Chart */}
      <Card className="p-6">
        <h2 className="text-xl font-semibold text-gray-900 dark:text-white mb-4">
          Pillar Usage Distribution
        </h2>
        <PillarUsageChart data={metrics} isLoading={isLoading} />
      </Card>

      {/* Detailed Pillar Views */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {metrics?.reality && (
          <RealityPillarDetails
            data={metrics.reality}
            isLoading={isLoading}
            onSelect={() => setSelectedPillar('reality')}
            isSelected={selectedPillar === 'reality'}
          />
        )}

        {metrics?.contracts && (
          <ContractsPillarDetails
            data={metrics.contracts}
            isLoading={isLoading}
            onSelect={() => setSelectedPillar('contracts')}
            isSelected={selectedPillar === 'contracts'}
          />
        )}

        {metrics?.devx && (
          <Card className="p-6">
            <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
              DevX Pillar
            </h3>
            <p className="text-sm text-gray-600 dark:text-gray-400">
              SDK installations: {metrics.devx.sdk_installations}
            </p>
            <p className="text-sm text-gray-600 dark:text-gray-400">
              Client generations: {metrics.devx.client_generations}
            </p>
            <p className="text-sm text-gray-600 dark:text-gray-400">
              Playground sessions: {metrics.devx.playground_sessions}
            </p>
          </Card>
        )}

        {metrics?.cloud && (
          <Card className="p-6">
            <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
              Cloud Pillar
            </h3>
            <p className="text-sm text-gray-600 dark:text-gray-400">
              Shared scenarios: {metrics.cloud.shared_scenarios_count}
            </p>
            <p className="text-sm text-gray-600 dark:text-gray-400">
              Marketplace downloads: {metrics.cloud.marketplace_downloads}
            </p>
            <p className="text-sm text-gray-600 dark:text-gray-400">
              Org templates used: {metrics.cloud.org_templates_used}
            </p>
          </Card>
        )}

        {metrics?.ai && (
          <Card className="p-6">
            <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
              AI Pillar
            </h3>
            <p className="text-sm text-gray-600 dark:text-gray-400">
              AI-generated mocks: {metrics.ai.ai_generated_mocks}
            </p>
            <p className="text-sm text-gray-600 dark:text-gray-400">
              Voice commands: {metrics.ai.voice_commands}
            </p>
            <p className="text-sm text-gray-600 dark:text-gray-400">
              LLM-assisted operations: {metrics.ai.llm_assisted_operations}
            </p>
          </Card>
        )}
      </div>
    </div>
  );
};
