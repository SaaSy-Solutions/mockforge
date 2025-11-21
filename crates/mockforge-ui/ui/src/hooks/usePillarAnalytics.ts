/**
 * React Query hooks for pillar analytics API
 */

import { useQuery } from '@tanstack/react-query';
import axios from 'axios';

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

const API_BASE_URL = '/api/v2/analytics/pillars';

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

/**
 * Fetch pillar metrics for a workspace
 */
export const usePillarMetrics = (
  workspaceId: string | undefined,
  orgId: string | undefined,
  query: PillarMetricsQuery
) => {
  return useQuery<PillarUsageMetrics>({
    queryKey: ['pillar-metrics', workspaceId, orgId, query.time_range],
    queryFn: async () => {
      let url: string;
      if (workspaceId) {
        url = `${API_BASE_URL}/workspace/${workspaceId}`;
      } else if (orgId) {
        url = `${API_BASE_URL}/org/${orgId}`;
      } else {
        throw new Error('Either workspaceId or orgId must be provided');
      }

      const params = new URLSearchParams();
      if (query.time_range) {
        // Convert time range to duration
        const duration = timeRangeToDuration(query.time_range);
        params.append('duration', duration.toString());
      } else {
        params.append('duration', '3600'); // Default 1 hour
      }

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
