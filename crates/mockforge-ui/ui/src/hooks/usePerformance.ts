/**
 * Hook for Performance Mode API
 *
 * Provides React hooks for managing performance mode:
 * - Start/stop performance simulation
 * - Configure RPS profiles
 * - Add/remove bottlenecks
 * - Get performance metrics
 */

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { authenticatedFetch } from '../utils/apiClient';
import { logger } from '@/utils/logger';

// Types for performance mode
export interface RpsStage {
  duration_secs: number;
  target_rps: number;
  name?: string;
}

export interface RpsProfile {
  name: string;
  stages: RpsStage[];
}

export interface BottleneckConfig {
  bottleneck_type: 'cpu' | 'memory' | 'network' | 'io' | 'database';
  severity: number; // 0.0-1.0
  endpoint_pattern?: string;
  duration_secs?: number;
}

export interface PerformanceMetrics {
  total_requests: number;
  successful_requests: number;
  failed_requests: number;
  current_rps: number;
  target_rps: number;
  avg_latency_ms: number;
  p95_latency_ms: number;
  p99_latency_ms: number;
  error_rate: number;
  endpoint_metrics: Record<string, EndpointMetrics>;
  timestamp: string;
}

export interface EndpointMetrics {
  request_count: number;
  avg_latency_ms: number;
  p95_latency_ms: number;
  p99_latency_ms: number;
  error_count: number;
  error_rate: number;
}

export interface PerformanceSnapshot {
  id: string;
  timestamp: string;
  metrics: PerformanceMetrics;
  active_bottlenecks: string[];
}

export interface PerformanceStatus {
  running: boolean;
  target_rps: number;
  current_rps: number;
  bottlenecks: number;
  bottleneck_types: string[];
}

export interface StartPerformanceRequest {
  initial_rps: number;
  rps_profile?: RpsProfile;
  bottlenecks?: BottleneckConfig[];
}

export interface UpdateRpsRequest {
  target_rps: number;
}

export interface AddBottleneckRequest {
  bottleneck: BottleneckConfig;
}

// Query keys
export const performanceQueryKeys = {
  status: ['performance', 'status'] as const,
  snapshot: ['performance', 'snapshot'] as const,
};

/**
 * Get performance status
 */
export function usePerformanceStatus() {
  return useQuery<PerformanceStatus>({
    queryKey: performanceQueryKeys.status,
    queryFn: async () => {
      const response = await authenticatedFetch('/api/performance/status');
      if (!response.ok) {
        throw new Error(`Failed to fetch performance status: ${response.status}`);
      }
      return response.json();
    },
    refetchInterval: 2000, // Poll every 2 seconds
    staleTime: 1000,
  });
}

/**
 * Get performance snapshot
 */
export function usePerformanceSnapshot() {
  return useQuery<PerformanceSnapshot>({
    queryKey: performanceQueryKeys.snapshot,
    queryFn: async () => {
      const response = await authenticatedFetch('/api/performance/snapshot');
      if (!response.ok) {
        if (response.status === 404) {
          // Performance mode not started
          return null;
        }
        throw new Error(`Failed to fetch performance snapshot: ${response.status}`);
      }
      return response.json();
    },
    refetchInterval: 2000, // Poll every 2 seconds
    staleTime: 1000,
  });
}

/**
 * Start performance mode
 */
export function useStartPerformance() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (request: StartPerformanceRequest) => {
      const response = await authenticatedFetch('/api/performance/start', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(request),
      });

      if (!response.ok) {
        const error = await response.json().catch(() => ({ message: 'Failed to start performance mode' }));
        throw new Error(error.message || `HTTP ${response.status}`);
      }

      return response.json();
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: performanceQueryKeys.status });
      queryClient.invalidateQueries({ queryKey: performanceQueryKeys.snapshot });
      logger.info('Performance mode started');
    },
    onError: (error) => {
      logger.error('Failed to start performance mode', error);
    },
  });
}

/**
 * Stop performance mode
 */
export function useStopPerformance() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async () => {
      const response = await authenticatedFetch('/api/performance/stop', {
        method: 'POST',
      });

      if (!response.ok) {
        const error = await response.json().catch(() => ({ message: 'Failed to stop performance mode' }));
        throw new Error(error.message || `HTTP ${response.status}`);
      }

      return response.json();
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: performanceQueryKeys.status });
      queryClient.invalidateQueries({ queryKey: performanceQueryKeys.snapshot });
      logger.info('Performance mode stopped');
    },
    onError: (error) => {
      logger.error('Failed to stop performance mode', error);
    },
  });
}

/**
 * Update target RPS
 */
export function useUpdateRps() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (request: UpdateRpsRequest) => {
      const response = await authenticatedFetch('/api/performance/rps', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(request),
      });

      if (!response.ok) {
        const error = await response.json().catch(() => ({ message: 'Failed to update RPS' }));
        throw new Error(error.message || `HTTP ${response.status}`);
      }

      return response.json();
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: performanceQueryKeys.status });
      queryClient.invalidateQueries({ queryKey: performanceQueryKeys.snapshot });
    },
  });
}

/**
 * Add bottleneck
 */
export function useAddBottleneck() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (request: AddBottleneckRequest) => {
      const response = await authenticatedFetch('/api/performance/bottlenecks', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(request),
      });

      if (!response.ok) {
        const error = await response.json().catch(() => ({ message: 'Failed to add bottleneck' }));
        throw new Error(error.message || `HTTP ${response.status}`);
      }

      return response.json();
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: performanceQueryKeys.status });
      queryClient.invalidateQueries({ queryKey: performanceQueryKeys.snapshot });
    },
  });
}

/**
 * Clear all bottlenecks
 */
export function useClearBottlenecks() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async () => {
      const response = await authenticatedFetch('/api/performance/bottlenecks', {
        method: 'DELETE',
      });

      if (!response.ok) {
        const error = await response.json().catch(() => ({ message: 'Failed to clear bottlenecks' }));
        throw new Error(error.message || `HTTP ${response.status}`);
      }

      return response.json();
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: performanceQueryKeys.status });
      queryClient.invalidateQueries({ queryKey: performanceQueryKeys.snapshot });
    },
  });
}
