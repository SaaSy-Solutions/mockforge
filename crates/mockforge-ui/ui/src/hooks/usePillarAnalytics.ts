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

const API_BASE_URL = '/api/v1';

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
        url = `${API_BASE_URL}/workspaces/${workspaceId}/analytics/pillars`;
      } else if (orgId) {
        url = `${API_BASE_URL}/organizations/${orgId}/analytics/pillars`;
      } else {
        throw new Error('Either workspaceId or orgId must be provided');
      }

      const params = new URLSearchParams();
      if (query.time_range) {
        params.append('time_range', query.time_range);
      }

      const response = await axios.get<PillarUsageMetrics>(
        `${url}?${params.toString()}`
      );
      return response.data;
    },
    enabled: !!(workspaceId || orgId),
    staleTime: 30000, // 30 seconds
  });
};
