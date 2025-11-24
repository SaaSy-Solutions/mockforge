/**
 * Hook for accessing Coverage Metrics API (MockOps)
 * Provides integration with scenario usage, persona CI hits, endpoint coverage, etc.
 */

import { useQuery, UseQueryResult } from '@tanstack/react-query';

// API Base URL
const API_BASE = '/api/v2/analytics';

// ============================================================================
// Types
// ============================================================================

export interface ScenarioUsageMetrics {
  id?: number;
  scenario_id: string;
  workspace_id?: string | null;
  org_id?: string | null;
  usage_count: number;
  last_used_at?: number | null;
  usage_pattern?: string | null;
  created_at?: number | null;
  updated_at?: number | null;
}

export interface PersonaCIHit {
  id?: number;
  persona_id: string;
  workspace_id?: string | null;
  org_id?: string | null;
  ci_run_id?: string | null;
  hit_count: number;
  hit_at: number;
  created_at?: number | null;
}

export interface EndpointCoverage {
  id?: number;
  endpoint: string;
  method?: string | null;
  protocol: string;
  workspace_id?: string | null;
  org_id?: string | null;
  test_count: number;
  last_tested_at?: number | null;
  coverage_percentage?: number | null;
  created_at?: number | null;
  updated_at?: number | null;
}

export interface RealityLevelStaleness {
  id?: number;
  workspace_id: string;
  org_id?: string | null;
  endpoint?: string | null;
  method?: string | null;
  protocol?: string | null;
  current_reality_level?: string | null;
  last_updated_at?: number | null;
  staleness_days?: number | null;
  created_at?: number | null;
  updated_at?: number | null;
}

export interface DriftPercentageMetrics {
  id?: number;
  workspace_id: string;
  org_id?: string | null;
  total_mocks: number;
  drifting_mocks: number;
  drift_percentage: number;
  measured_at: number;
  created_at?: number | null;
}

export interface CoverageMetricsQuery {
  workspace_id?: string;
  org_id?: string;
  limit?: number;
  min_coverage?: number;
  max_staleness_days?: number;
}

// ============================================================================
// API Response Types
// ============================================================================

interface ApiResponse<T> {
  success: boolean;
  data: T;
  message?: string;
}

// ============================================================================
// Hooks
// ============================================================================

/**
 * Fetch scenario usage metrics
 */
export function useScenarioUsage(
  query?: CoverageMetricsQuery,
  options?: { enabled?: boolean }
): UseQueryResult<ScenarioUsageMetrics[], Error> {
  return useQuery({
    queryKey: ['coverage-metrics', 'scenarios', 'usage', query],
    queryFn: async () => {
      const params = new URLSearchParams();
      if (query?.workspace_id) params.append('workspace_id', query.workspace_id);
      if (query?.org_id) params.append('org_id', query.org_id);
      if (query?.limit) params.append('limit', query.limit.toString());

      const response = await fetch(`${API_BASE}/scenarios/usage?${params.toString()}`);
      if (!response.ok) {
        throw new Error(`Failed to fetch scenario usage: ${response.statusText}`);
      }

      const result: ApiResponse<ScenarioUsageMetrics[]> = await response.json();
      return result.data;
    },
    enabled: options?.enabled !== false,
    refetchInterval: 30000, // Refresh every 30 seconds
  });
}

/**
 * Fetch persona CI hits
 */
export function usePersonaCIHits(
  query?: CoverageMetricsQuery,
  options?: { enabled?: boolean }
): UseQueryResult<PersonaCIHit[], Error> {
  return useQuery({
    queryKey: ['coverage-metrics', 'personas', 'ci-hits', query],
    queryFn: async () => {
      const params = new URLSearchParams();
      if (query?.workspace_id) params.append('workspace_id', query.workspace_id);
      if (query?.org_id) params.append('org_id', query.org_id);
      if (query?.limit) params.append('limit', query.limit.toString());

      const response = await fetch(`${API_BASE}/personas/ci-hits?${params.toString()}`);
      if (!response.ok) {
        throw new Error(`Failed to fetch persona CI hits: ${response.statusText}`);
      }

      const result: ApiResponse<PersonaCIHit[]> = await response.json();
      return result.data;
    },
    enabled: options?.enabled !== false,
    refetchInterval: 30000,
  });
}

/**
 * Fetch endpoint coverage metrics
 */
export function useEndpointCoverage(
  query?: CoverageMetricsQuery,
  options?: { enabled?: boolean }
): UseQueryResult<EndpointCoverage[], Error> {
  return useQuery({
    queryKey: ['coverage-metrics', 'endpoints', 'coverage', query],
    queryFn: async () => {
      const params = new URLSearchParams();
      if (query?.workspace_id) params.append('workspace_id', query.workspace_id);
      if (query?.org_id) params.append('org_id', query.org_id);
      if (query?.min_coverage !== undefined) {
        params.append('min_coverage', query.min_coverage.toString());
      }

      const response = await fetch(`${API_BASE}/endpoints/coverage?${params.toString()}`);
      if (!response.ok) {
        throw new Error(`Failed to fetch endpoint coverage: ${response.statusText}`);
      }

      const result: ApiResponse<EndpointCoverage[]> = await response.json();
      return result.data;
    },
    enabled: options?.enabled !== false,
    refetchInterval: 30000,
  });
}

/**
 * Fetch reality level staleness metrics
 */
export function useRealityLevelStaleness(
  query?: CoverageMetricsQuery,
  options?: { enabled?: boolean }
): UseQueryResult<RealityLevelStaleness[], Error> {
  return useQuery({
    queryKey: ['coverage-metrics', 'reality-levels', 'staleness', query],
    queryFn: async () => {
      const params = new URLSearchParams();
      if (query?.workspace_id) params.append('workspace_id', query.workspace_id);
      if (query?.org_id) params.append('org_id', query.org_id);
      if (query?.max_staleness_days !== undefined) {
        params.append('max_staleness_days', query.max_staleness_days.toString());
      }

      const response = await fetch(
        `${API_BASE}/reality-levels/staleness?${params.toString()}`
      );
      if (!response.ok) {
        throw new Error(`Failed to fetch reality level staleness: ${response.statusText}`);
      }

      const result: ApiResponse<RealityLevelStaleness[]> = await response.json();
      return result.data;
    },
    enabled: options?.enabled !== false,
    refetchInterval: 30000,
  });
}

/**
 * Fetch drift percentage metrics
 */
export function useDriftPercentage(
  query?: CoverageMetricsQuery,
  options?: { enabled?: boolean }
): UseQueryResult<DriftPercentageMetrics[], Error> {
  return useQuery({
    queryKey: ['coverage-metrics', 'drift', 'percentage', query],
    queryFn: async () => {
      const params = new URLSearchParams();
      if (query?.workspace_id) params.append('workspace_id', query.workspace_id);
      if (query?.org_id) params.append('org_id', query.org_id);
      if (query?.limit) params.append('limit', query.limit.toString());

      const response = await fetch(`${API_BASE}/drift/percentage?${params.toString()}`);
      if (!response.ok) {
        throw new Error(`Failed to fetch drift percentage: ${response.statusText}`);
      }

      const result: ApiResponse<DriftPercentageMetrics[]> = await response.json();
      return result.data;
    },
    enabled: options?.enabled !== false,
    refetchInterval: 30000,
  });
}
