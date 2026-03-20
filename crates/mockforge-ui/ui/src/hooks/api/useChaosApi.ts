import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { chaosApi } from '../../services/api';
import { queryKeys } from './queryKeys';

/**
 * Chaos engineering hooks
 */

/**
 * Get current chaos configuration
 */
export function useChaosConfig() {
  return useQuery({
    queryKey: queryKeys.chaosConfig,
    queryFn: () => chaosApi.getChaosConfig(),
    staleTime: 10000, // Consider data stale after 10 seconds
    refetchInterval: 30000, // Refetch every 30 seconds
  });
}

/**
 * Get current chaos status
 */
export function useChaosStatus() {
  return useQuery({
    queryKey: queryKeys.chaosStatus,
    queryFn: () => chaosApi.getChaosStatus(),
    staleTime: 5000, // Consider data stale after 5 seconds
    refetchInterval: 10000, // Refetch every 10 seconds
  });
}

/**
 * Update chaos latency configuration
 */
export function useUpdateChaosLatency() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (config: any) => chaosApi.updateChaosLatency(config),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.chaosConfig });
      queryClient.invalidateQueries({ queryKey: queryKeys.chaosStatus });
    },
  });
}

/**
 * Update chaos fault injection configuration
 */
export function useUpdateChaosFaults() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (config: any) => chaosApi.updateChaosFaults(config),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.chaosConfig });
      queryClient.invalidateQueries({ queryKey: queryKeys.chaosStatus });
    },
  });
}

/**
 * Update chaos traffic shaping configuration
 */
export function useUpdateChaosTraffic() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (config: any) => chaosApi.updateChaosTraffic(config),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.chaosConfig });
      queryClient.invalidateQueries({ queryKey: queryKeys.chaosStatus });
    },
  });
}

/**
 * Enable chaos engineering
 */
export function useEnableChaos() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: () => chaosApi.enableChaos(),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.chaosConfig });
      queryClient.invalidateQueries({ queryKey: queryKeys.chaosStatus });
    },
  });
}

/**
 * Disable chaos engineering
 */
export function useDisableChaos() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: () => chaosApi.disableChaos(),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.chaosConfig });
      queryClient.invalidateQueries({ queryKey: queryKeys.chaosStatus });
    },
  });
}

/**
 * Reset chaos configuration to defaults
 */
export function useResetChaos() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: () => chaosApi.resetChaos(),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.chaosConfig });
      queryClient.invalidateQueries({ queryKey: queryKeys.chaosStatus });
    },
  });
}

/**
 * Get latency metrics (time-series data)
 */
export function useChaosLatencyMetrics() {
  return useQuery({
    queryKey: queryKeys.chaosLatencyMetrics,
    queryFn: () => chaosApi.getLatencyMetrics(),
    refetchInterval: 500, // Refetch every 500ms for real-time graph
    staleTime: 100,
  });
}

/**
 * Get latency statistics
 */
export function useChaosLatencyStats() {
  return useQuery({
    queryKey: queryKeys.chaosLatencyStats,
    queryFn: () => chaosApi.getLatencyStats(),
    refetchInterval: 2000, // Refetch every 2 seconds
    staleTime: 500,
  });
}

/**
 * Get all network profiles
 */
export function useNetworkProfiles() {
  return useQuery({
    queryKey: queryKeys.networkProfiles,
    queryFn: () => chaosApi.getNetworkProfiles(),
    staleTime: 30000, // Consider data stale after 30 seconds
    refetchInterval: 60000, // Refetch every minute
  });
}

/**
 * Get a specific network profile
 */
export function useNetworkProfile(name: string) {
  return useQuery({
    queryKey: queryKeys.networkProfile(name),
    queryFn: () => chaosApi.getNetworkProfile(name),
    enabled: !!name,
    staleTime: 30000,
  });
}

/**
 * Apply a network profile
 */
export function useApplyNetworkProfile() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (name: string) => chaosApi.applyNetworkProfile(name),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.chaosConfig });
      queryClient.invalidateQueries({ queryKey: queryKeys.chaosStatus });
      queryClient.invalidateQueries({ queryKey: queryKeys.networkProfiles });
    },
  });
}

/**
 * Create a custom network profile
 */
export function useCreateNetworkProfile() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (profile: {
      name: string;
      description: string;
      chaos_config: any;
      tags?: string[];
    }) => chaosApi.createNetworkProfile(profile),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.networkProfiles });
    },
  });
}

/**
 * Delete a custom network profile
 */
export function useDeleteNetworkProfile() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (name: string) => chaosApi.deleteNetworkProfile(name),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.networkProfiles });
    },
  });
}

/**
 * Export a network profile
 */
export function useExportNetworkProfile() {
  return useMutation({
    mutationFn: ({ name, format }: { name: string; format?: 'json' | 'yaml' }) =>
      chaosApi.exportNetworkProfile(name, format || 'json'),
  });
}

/**
 * Import a network profile
 */
export function useImportNetworkProfile() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ content, format }: { content: string; format: 'json' | 'yaml' }) =>
      chaosApi.importNetworkProfile(content, format),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.networkProfiles });
    },
  });
}

/**
 * Update error pattern configuration
 */
export function useUpdateErrorPattern() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (pattern: {
      type: 'burst' | 'random' | 'sequential';
      count?: number;
      interval_ms?: number;
      probability?: number;
      sequence?: number[];
    }) => chaosApi.updateErrorPattern(pattern),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.chaosConfig });
      queryClient.invalidateQueries({ queryKey: queryKeys.chaosStatus });
    },
  });
}
