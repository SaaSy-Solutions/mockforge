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
import { usePillarMetrics, usePillarUsageSummary } from '@/hooks/usePillarAnalytics';
import { PillarOverviewCards } from './PillarOverviewCards';
import { PillarUsageChart } from './PillarUsageChart';
import { PillarRankingsCard } from './PillarRankingsCard';
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

  const { data: rankings, isLoading: rankingsLoading } = usePillarUsageSummary(
    workspaceId,
    orgId,
    query
  );

  if (error) {
    return (
      <div className="p-6">
        <Card className="p-6 border-danger-200 bg-danger-50 dark:bg-danger-900/20">
          <h2 className="text-lg font-semibold text-danger-700 dark:text-danger-200 mb-2">
            Error Loading Pillar Analytics
          </h2>
          <p className="text-danger-600 dark:text-danger-300">
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
          <h1 className="text-3xl font-bold text-foreground">
            Pillar Usage Analytics
          </h1>
          <p className="text-sm text-muted-foreground mt-1">
            Track adoption of MockForge's foundational pillars across your workspaces
          </p>
        </div>

        <TimeRangeSelector value={timeRange} onChange={setTimeRange} />
      </div>

      {/* Overview Cards */}
      <PillarOverviewCards data={metrics} isLoading={isLoading} />

      {/* Rankings and Distribution */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <PillarRankingsCard data={rankings} isLoading={rankingsLoading} />

        <Card className="p-6">
          <h2 className="text-xl font-semibold text-foreground mb-4">
            Pillar Usage Distribution
          </h2>
          <PillarUsageChart data={metrics} isLoading={isLoading} />
        </Card>
      </div>

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
            <h3 className="text-lg font-semibold text-foreground mb-4">
              DevX Pillar
            </h3>
            <p className="text-sm text-muted-foreground">
              SDK installations: {metrics.devx.sdk_installations}
            </p>
            <p className="text-sm text-muted-foreground">
              Client generations: {metrics.devx.client_generations}
            </p>
            <p className="text-sm text-muted-foreground">
              Playground sessions: {metrics.devx.playground_sessions}
            </p>
            <p className="text-sm text-muted-foreground">
              CLI commands: {metrics.devx.cli_commands}
            </p>
          </Card>
        )}

        {metrics?.cloud && (
          <Card className="p-6">
            <h3 className="text-lg font-semibold text-foreground mb-4">
              Cloud Pillar
            </h3>
            <p className="text-sm text-muted-foreground">
              Shared scenarios: {metrics.cloud.shared_scenarios_count}
            </p>
            <p className="text-sm text-muted-foreground">
              Marketplace downloads: {metrics.cloud.marketplace_downloads}
            </p>
            <p className="text-sm text-muted-foreground">
              Org templates used: {metrics.cloud.org_templates_used}
            </p>
            <p className="text-sm text-muted-foreground">
              Collaborative workspaces: {metrics.cloud.collaborative_workspaces}
            </p>
          </Card>
        )}

        {metrics?.ai && (
          <Card className="p-6">
            <h3 className="text-lg font-semibold text-foreground mb-4">
              AI Pillar
            </h3>
            <p className="text-sm text-muted-foreground">
              AI-generated mocks: {metrics.ai.ai_generated_mocks}
            </p>
            <p className="text-sm text-muted-foreground">
              AI contract diffs: {metrics.ai.ai_contract_diffs}
            </p>
            <p className="text-sm text-muted-foreground">
              Voice commands: {metrics.ai.voice_commands}
            </p>
            <p className="text-sm text-muted-foreground">
              LLM-assisted operations: {metrics.ai.llm_assisted_operations}
            </p>
          </Card>
        )}
      </div>
    </div>
  );
};
