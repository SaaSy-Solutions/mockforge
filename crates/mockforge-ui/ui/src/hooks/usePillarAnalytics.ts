/**
 * React Query hooks for pillar analytics API
 */

import { useQuery } from '@tanstack/react-query';
import axios from 'axios';
import { IS_CLOUD } from '../utils/mode';

export interface PillarMetricsQuery {
  time_range?: string;
}

export interface RealityPillarMetrics {
  blended_reality_percent: number;
  smart_personas_percent: number;
  static_fixtures_percent: number;
  avg_reality_level: number;
  chaos_enabled_count: number;
  total_scenarios: number;
}

export interface ContractsPillarMetrics {
  validation_disabled_percent: number;
  validation_warn_percent: number;
  validation_enforce_percent: number;
  drift_budget_configured_count: number;
  drift_incidents_count: number;
  contract_sync_cycles: number;
}

export interface DevXPillarMetrics {
  sdk_installations: number;
  client_generations: number;
  playground_sessions: number;
  cli_commands: number;
}

export interface CloudPillarMetrics {
  shared_scenarios_count: number;
  marketplace_downloads: number;
  org_templates_used: number;
  collaborative_workspaces: number;
}

export interface AiPillarMetrics {
  ai_generated_mocks: number;
  ai_contract_diffs: number;
  voice_commands: number;
  llm_assisted_operations: number;
}

export interface PillarUsageMetrics {
  workspace_id?: string | null;
  org_id?: string | null;
  time_range: string;
  reality?: RealityPillarMetrics | null;
  contracts?: ContractsPillarMetrics | null;
  devx?: DevXPillarMetrics | null;
  cloud?: CloudPillarMetrics | null;
  ai?: AiPillarMetrics | null;
}

const SELF_HOSTED_API_BASE = '/api/v2/analytics/pillars';

/**
 * Convert time range string to duration in seconds
 */
function timeRangeToDuration(timeRange: string): number {
  const match = timeRange.match(/^(\d+)([hdw])$/);
  if (!match) return 3600; // Default to 1 hour

  const value = parseInt(match[1], 10);
  const unit = match[2];

  switch (unit) {
    case 'h':
      return value * 3600;
    case 'd':
      return value * 86400;
    case 'w':
      return value * 604800;
    default:
      return 3600;
  }
}

// Cloud registry-server expects time_range as a discrete bucket string;
// self-hosted takes raw seconds via `duration`.
function normalizeTimeRangeForCloud(timeRange: string | undefined): string {
  if (!timeRange) return '7d';
  const seconds = timeRangeToDuration(timeRange);
  if (seconds <= 86400) return '1d';
  if (seconds <= 604800) return '7d';
  if (seconds <= 2592000) return '30d';
  return '90d';
}

/**
 * Fetch pillar metrics for a workspace
 */
export const usePillarMetrics = (
  workspaceId: string | undefined,
  orgId: string | undefined,
  query: PillarMetricsQuery
) => {
  return useQuery<PillarUsageMetrics>({
    queryKey: ['pillar-metrics', workspaceId, orgId, query.time_range, IS_CLOUD],
    queryFn: async () => {
      if (IS_CLOUD) {
        let url: string;
        if (workspaceId) {
          url = `/api/v1/workspaces/${workspaceId}/analytics/pillars`;
        } else if (orgId) {
          url = `/api/v1/organizations/${orgId}/analytics/pillars`;
        } else {
          throw new Error('Either workspaceId or orgId must be provided');
        }

        const params = new URLSearchParams();
        params.append('time_range', normalizeTimeRangeForCloud(query.time_range));

        const response = await axios.get<{
          workspace_id: string | null;
          org_id: string | null;
          time_range: string;
          metrics: PillarUsageMetrics;
        }>(`${url}?${params.toString()}`);

        return response.data.metrics;
      }

      let url: string;
      if (workspaceId) {
        url = `${SELF_HOSTED_API_BASE}/workspace/${workspaceId}`;
      } else if (orgId) {
        url = `${SELF_HOSTED_API_BASE}/org/${orgId}`;
      } else {
        throw new Error('Either workspaceId or orgId must be provided');
      }

      const params = new URLSearchParams();
      const duration = query.time_range ? timeRangeToDuration(query.time_range) : 3600;
      params.append('duration', duration.toString());

      const response = await axios.get<{ success: boolean; data: PillarUsageMetrics }>(
        `${url}?${params.toString()}`
      );

      if (response.data.success && response.data.data) {
        return response.data.data;
      }

      throw new Error('Failed to fetch pillar metrics');
    },
    enabled: !!(workspaceId || orgId),
    staleTime: 30000, // 30 seconds
  });
};

export interface PillarRanking {
  pillar: string;
  usage: number;
  percentage: number;
  is_most_used: boolean;
  is_least_used: boolean;
}

export interface PillarUsageSummary {
  time_range: string;
  rankings: PillarRanking[];
  total_usage: number;
}

/**
 * Fetch the ranked pillar usage summary (most/least used pillars)
 */
export const usePillarUsageSummary = (
  workspaceId: string | undefined,
  orgId: string | undefined,
  query: PillarMetricsQuery
) => {
  return useQuery<PillarUsageSummary>({
    queryKey: ['pillar-usage-summary', workspaceId, orgId, query.time_range, IS_CLOUD],
    queryFn: async () => {
      // The registry-server has no /summary endpoint yet, so in cloud mode
      // we synthesize a ranking from the metrics response.
      if (IS_CLOUD) {
        let url: string;
        if (workspaceId) {
          url = `/api/v1/workspaces/${workspaceId}/analytics/pillars`;
        } else if (orgId) {
          url = `/api/v1/organizations/${orgId}/analytics/pillars`;
        } else {
          throw new Error('Either workspaceId or orgId must be provided');
        }

        const params = new URLSearchParams();
        params.append('time_range', normalizeTimeRangeForCloud(query.time_range));

        const response = await axios.get<{ time_range: string; metrics: PillarUsageMetrics }>(
          `${url}?${params.toString()}`
        );

        return synthesizePillarSummary(response.data.metrics, response.data.time_range);
      }

      let url: string;
      if (workspaceId) {
        url = `${SELF_HOSTED_API_BASE}/workspace/${workspaceId}/summary`;
      } else if (orgId) {
        url = `${SELF_HOSTED_API_BASE}/org/${orgId}/summary`;
      } else {
        throw new Error('Either workspaceId or orgId must be provided');
      }

      const params = new URLSearchParams();
      const duration = query.time_range
        ? timeRangeToDuration(query.time_range)
        : 3600;
      params.append('duration', duration.toString());

      const response = await axios.get<{ success: boolean; data: PillarUsageSummary }>(
        `${url}?${params.toString()}`
      );

      if (response.data.success && response.data.data) {
        return response.data.data;
      }

      throw new Error('Failed to fetch pillar usage summary');
    },
    enabled: !!(workspaceId || orgId),
    staleTime: 30000,
  });
};

function synthesizePillarSummary(
  metrics: PillarUsageMetrics,
  timeRange: string
): PillarUsageSummary {
  const pillarTotals: Array<{ pillar: string; usage: number }> = [
    {
      pillar: 'reality',
      usage: metrics.reality
        ? metrics.reality.chaos_enabled_count + metrics.reality.total_scenarios
        : 0,
    },
    {
      pillar: 'contracts',
      usage: metrics.contracts
        ? metrics.contracts.drift_budget_configured_count +
          metrics.contracts.drift_incidents_count +
          metrics.contracts.contract_sync_cycles
        : 0,
    },
    {
      pillar: 'devx',
      usage: metrics.devx
        ? metrics.devx.sdk_installations +
          metrics.devx.client_generations +
          metrics.devx.playground_sessions +
          metrics.devx.cli_commands
        : 0,
    },
    {
      pillar: 'cloud',
      usage: metrics.cloud
        ? metrics.cloud.shared_scenarios_count +
          metrics.cloud.marketplace_downloads +
          metrics.cloud.org_templates_used +
          metrics.cloud.collaborative_workspaces
        : 0,
    },
    {
      pillar: 'ai',
      usage: metrics.ai
        ? metrics.ai.ai_generated_mocks +
          metrics.ai.ai_contract_diffs +
          metrics.ai.voice_commands +
          metrics.ai.llm_assisted_operations
        : 0,
    },
  ];

  const total = pillarTotals.reduce((sum, p) => sum + p.usage, 0);
  const max = Math.max(...pillarTotals.map((p) => p.usage));
  const min = Math.min(...pillarTotals.map((p) => p.usage));

  return {
    time_range: timeRange,
    total_usage: total,
    rankings: pillarTotals.map((p) => ({
      pillar: p.pillar,
      usage: p.usage,
      percentage: total > 0 ? (p.usage / total) * 100 : 0,
      is_most_used: total > 0 && p.usage === max,
      is_least_used: total > 0 && p.usage === min && p.usage !== max,
    })),
  };
}
