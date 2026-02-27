import { logger } from '@/utils/logger';
/**
 * React hooks for MockForge Admin API
 * Uses React Query for caching, background refetching, and optimistic updates
 */

import React from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  apiService,
  dashboardApi,
  serverApi,
  routesApi,
  logsApi,
  metricsApi,
  configApi,
  validationApi,
  envApi,
  filesApi,
  fixturesApi,
  smokeTestsApi,
  importApi,
  chaosApi,
  timeTravelApi,
  realityApi,
  consistencyApi,
  proxyApi,
  type ProxyRuleRequest,
} from '../services/api';
import { driftApi, type DriftIncident, type ListIncidentsRequest, type CreateDriftBudgetRequest, type UpdateIncidentRequest } from '../services/driftApi';
import type {
  CreateEnvironmentRequest,
  UpdateEnvironmentRequest,
  SetVariableRequest,
  AutocompleteRequest,
} from '../types';

// Query keys for React Query
export const queryKeys = {
  dashboard: ['dashboard'] as const,
  health: ['health'] as const,
  serverInfo: ['serverInfo'] as const,
  restartStatus: ['restartStatus'] as const,
  routes: ['routes'] as const,
  logs: ['logs'] as const,
  metrics: ['metrics'] as const,
  config: ['config'] as const,
  validation: ['validation'] as const,
  envVars: ['envVars'] as const,
  fixtures: ['fixtures'] as const,
  smokeTests: ['smokeTests'] as const,
  import: ['import'] as const,
  importHistory: ['importHistory'] as const,
  chaosConfig: ['chaosConfig'] as const,
  chaosStatus: ['chaosStatus'] as const,
  chaosLatencyMetrics: ['chaosLatencyMetrics'] as const,
  chaosLatencyStats: ['chaosLatencyStats'] as const,
  networkProfiles: ['networkProfiles'] as const,
  networkProfile: (name: string) => ['networkProfile', name] as const,
  timeTravelStatus: ['timeTravelStatus'] as const,
  cronJobs: ['cronJobs'] as const,
  mutationRules: ['mutationRules'] as const,
  proxyRules: ['proxyRules'] as const,
  proxyInspect: ['proxyInspect'] as const,
  realityLevel: ['realityLevel'] as const,
  realityPresets: ['realityPresets'] as const,
  lifecyclePresets: ['lifecyclePresets'] as const,
  lifecyclePreset: (name: string) => ['lifecyclePreset', name] as const,
  // Drift budget and incidents
  driftBudgets: ['driftBudgets'] as const,
  driftBudget: (id: string) => ['driftBudget', id] as const,
  driftIncidents: (params?: ListIncidentsRequest) => ['driftIncidents', params] as const,
  driftIncident: (id: string) => ['driftIncident', id] as const,
  driftIncidentStats: ['driftIncidentStats'] as const,
};

/**
 * Dashboard hooks
 */
export function useDashboard() {
  return useQuery({
    queryKey: queryKeys.dashboard,
    queryFn: async () => {
      if (!dashboardApi) {
        logger.error('dashboardApi is undefined!');
        throw new Error('dashboardApi service not initialized');
      }
      return dashboardApi.getDashboard();
    },
    refetchInterval: 5000, // Refetch every 5 seconds for real-time updates
    refetchIntervalInBackground: true, // Continue refetching even when tab is in background
    staleTime: 2000, // Consider data stale after 2 seconds
  });
}

export function useHealth() {
  return useQuery({
    queryKey: queryKeys.health,
    queryFn: dashboardApi.getHealth,
    refetchInterval: 60000, // Refetch every minute
  });
}

/**
 * Server management hooks
 */
export function useServerInfo() {
  return useQuery({
    queryKey: queryKeys.serverInfo,
    queryFn: serverApi.getServerInfo,
    staleTime: 30000,
  });
}

export function useRestartStatus() {
  return useQuery({
    queryKey: queryKeys.restartStatus,
    queryFn: serverApi.getRestartStatus,
    refetchInterval: 5000, // Poll frequently during restart
    enabled: false, // Only enable when restart is initiated
  });
}

export function useRestartServers() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (reason?: string) => serverApi.restartServer(reason),
    onSuccess: () => {
      // Invalidate and refetch restart status
      queryClient.invalidateQueries({ queryKey: queryKeys.restartStatus });
      queryClient.invalidateQueries({ queryKey: queryKeys.dashboard });
    },
  });
}

/**
 * Routes hooks
 */
export function useRoutes() {
  return useQuery({
    queryKey: queryKeys.routes,
    queryFn: routesApi.getRoutes,
    staleTime: 60000, // Routes don't change often
  });
}

/**
 * Logs hooks
 */
export function useLogs(params?: {
  method?: string;
  path?: string;
  status?: number;
  limit?: number;
  refetchInterval?: number;
}) {
  const { refetchInterval, ...apiParams } = params || {};
  return useQuery({
    queryKey: [...queryKeys.logs, apiParams],
    queryFn: () => logsApi.getLogs(apiParams),
    staleTime: 5000, // Logs can change frequently
    refetchInterval: refetchInterval, // Optional auto-refetch interval
  });
}

export function useClearLogs() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: logsApi.clearLogs,
    onSuccess: () => {
      // Clear logs from cache
      queryClient.setQueryData(queryKeys.logs, []);
      queryClient.invalidateQueries({ queryKey: queryKeys.dashboard });
    },
  });
}

/**
 * Metrics hooks
 */
export function useMetrics() {
  return useQuery({
    queryKey: queryKeys.metrics,
    queryFn: async () => {
      if (!metricsApi) {
        logger.error('metricsApi is undefined!');
        throw new Error('metricsApi service not initialized');
      }
      return metricsApi.getMetrics();
    },
    refetchInterval: 15000, // Update metrics every 15 seconds
    staleTime: 5000,
  });
}

/**
 * Configuration hooks
 */
export function useConfig() {
  return useQuery({
    queryKey: queryKeys.config,
    queryFn: configApi.getConfig,
    staleTime: 30000,
  });
}

export function useUpdateLatency() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: configApi.updateLatency,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.config });
      queryClient.invalidateQueries({ queryKey: queryKeys.dashboard });
    },
  });
}

export function useUpdateFaults() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: configApi.updateFaults,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.config });
      queryClient.invalidateQueries({ queryKey: queryKeys.dashboard });
    },
  });
}

export function useUpdateProxy() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: configApi.updateProxy,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.config });
      queryClient.invalidateQueries({ queryKey: queryKeys.dashboard });
    },
  });
}

/**
 * Validation hooks
 */
export function useValidation() {
  return useQuery({
    queryKey: queryKeys.validation,
    queryFn: validationApi.getValidation,
    staleTime: 30000,
  });
}

export function useUpdateValidation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: validationApi.updateValidation,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.validation });
    },
  });
}

/**
 * Environment variables hooks
 */
export function useEnvVars() {
  return useQuery({
    queryKey: queryKeys.envVars,
    queryFn: envApi.getEnvVars,
    staleTime: 60000, // Env vars don't change often
  });
}

export function useUpdateEnvVar() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ key, value }: { key: string; value: string }) =>
      envApi.updateEnvVar(key, value),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.envVars });
    },
  });
}

/**
 * Files hooks
 */
export function useFileContent() {
  return useMutation({
    mutationFn: ({ path, type }: { path: string; type: string }) =>
      filesApi.getFileContent({ path, type }),
  });
}

export function useSaveFileContent() {
  const _queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ path, content }: { path: string; content: string }) =>
      filesApi.saveFileContent({ path, content }),
    onSuccess: () => {
      // Could invalidate file-related queries here if we had them
    },
  });
}

/**
 * Fixtures hooks
 */
export function useFixtures() {
  return useQuery({
    queryKey: ['fixtures-v2'],
    queryFn: async () => {
      try {
        const response = await fetch('/__mockforge/fixtures');
        if (!response.ok) {
          throw new Error(`HTTP error! status: ${response.status}`);
        }
        const data = await response.json();
        // Ensure we return an array - data.data is the array from the API response
        return Array.isArray(data.data) ? data.data : [];
      } catch (error) {
        logger.error('[FIXTURES ERROR] Failed to fetch fixtures',error);
        throw error;
      }
    },
    retry: false,
    staleTime: 30000,
  });
}

export function useDeleteFixture() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: fixturesApi.deleteFixture,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.fixtures });
      queryClient.invalidateQueries({ queryKey: queryKeys.dashboard });
    },
  });
}

export function useDeleteFixturesBulk() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: fixturesApi.deleteFixturesBulk,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.fixtures });
      queryClient.invalidateQueries({ queryKey: queryKeys.dashboard });
    },
  });
}

export function useDownloadFixture() {
  return useMutation({
    mutationFn: fixturesApi.downloadFixture,
  });
}

export function useRenameFixture() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ oldPath, newPath }: { oldPath: string; newPath: string }) =>
      fixturesApi.renameFixture(oldPath, newPath),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.fixtures });
    },
  });
}

export function useMoveFixture() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ sourcePath, destinationPath }: { sourcePath: string; destinationPath: string }) =>
      fixturesApi.moveFixture(sourcePath, destinationPath),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.fixtures });
    },
  });
}

/**
 * Smoke tests hooks
 */
export function useSmokeTests() {
  return useQuery({
    queryKey: queryKeys.smokeTests,
    queryFn: smokeTestsApi.getSmokeTests,
    staleTime: 10000,
  });
}

export function useRunSmokeTests() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: smokeTestsApi.runSmokeTests,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.smokeTests });
    },
  });
}

/**
 * Import hooks
 */
export function useImportPostman() {
  return useMutation({
    mutationFn: importApi.importPostman,
  });
}

export function useImportInsomnia() {
  return useMutation({
    mutationFn: importApi.importInsomnia,
  });
}

export function useImportCurl() {
  return useMutation({
    mutationFn: importApi.importCurl,
  });
}

export function usePreviewImport() {
  return useMutation({
    mutationFn: importApi.previewImport,
  });
}

export function useImportHistory() {
  return useQuery({
    queryKey: queryKeys.importHistory,
    queryFn: importApi.getImportHistory,
    staleTime: 30000, // Import history doesn't change often
  });
}

export function useClearImportHistory() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: importApi.clearImportHistory,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.importHistory });
    },
  });
}

/**
 * Combined hooks for common use cases
 */
export function useSystemStatus() {
  const dashboard = useDashboard();
  const health = useHealth();

  return {
    dashboard,
    health,
    isLoading: dashboard.isLoading || health.isLoading,
    error: dashboard.error || health.error,
    data: {
      dashboard: dashboard.data,
      health: health.data,
    },
  };
}

export function useConfiguration() {
  const config = useConfig();
  const validation = useValidation();
  const envVars = useEnvVars();

  return {
    config,
    validation,
    envVars,
    isLoading: config.isLoading || validation.isLoading || envVars.isLoading,
    error: config.error || validation.error || envVars.error,
    data: {
      config: config.data,
      validation: validation.data,
      envVars: envVars.data,
    },
  };
}

/**
 * Environment management hooks
 */
export function useEnvironments(workspaceId: string) {
  return useQuery({
    queryKey: ['environments', workspaceId],
    queryFn: () => apiService.getEnvironments(workspaceId),
    enabled: !!workspaceId,
    staleTime: 10000, // Cache for 10 seconds
  });
}

export function useCreateEnvironment(workspaceId: string) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (request: CreateEnvironmentRequest) => apiService.createEnvironment(workspaceId, request),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['environments', workspaceId] });
    },
  });
}

export function useUpdateEnvironment(workspaceId: string, environmentId: string) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (request: UpdateEnvironmentRequest) => apiService.updateEnvironment(workspaceId, environmentId, request),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['environments', workspaceId] });
      queryClient.invalidateQueries({ queryKey: ['environment-variables', workspaceId, environmentId] });
    },
  });
}

export function useDeleteEnvironment(workspaceId: string) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (environmentId: string) => apiService.deleteEnvironment(workspaceId, environmentId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['environments', workspaceId] });
    },
  });
}

export function useSetActiveEnvironment(workspaceId: string) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (environmentId: string) => apiService.setActiveEnvironment(workspaceId, environmentId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['environments', workspaceId] });
      queryClient.invalidateQueries({ queryKey: ['workspaces'] });
    },
  });
}

export function useEnvironmentVariables(workspaceId: string, environmentId: string) {
  return useQuery({
    queryKey: ['environment-variables', workspaceId, environmentId],
    queryFn: () => apiService.getEnvironmentVariables(workspaceId, environmentId),
    enabled: !!workspaceId && !!environmentId,
    staleTime: 5000, // Cache for 5 seconds
  });
}

export function useSetEnvironmentVariable(workspaceId: string, environmentId: string) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (request: SetVariableRequest) => apiService.setEnvironmentVariable(workspaceId, environmentId, request),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['environment-variables', workspaceId, environmentId] });
      queryClient.invalidateQueries({ queryKey: ['environments', workspaceId] });
    },
  });
}

export function useRemoveEnvironmentVariable(workspaceId: string, environmentId: string) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (variableName: string) => apiService.removeEnvironmentVariable(workspaceId, environmentId, variableName),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['environment-variables', workspaceId, environmentId] });
      queryClient.invalidateQueries({ queryKey: ['environments', workspaceId] });
    },
  });
}

export function useAutocomplete(workspaceId: string) {
  return useMutation({
    mutationFn: (request: AutocompleteRequest) => apiService.getAutocompleteSuggestions(workspaceId, request),
  });
}

/**
 * Ordering hooks
 */
export function useUpdateWorkspacesOrder() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (workspaceIds: string[]) => apiService.updateWorkspacesOrder(workspaceIds),
    onSuccess: () => {
      // Invalidate workspace queries to trigger refetch
      queryClient.invalidateQueries({ queryKey: ['workspaces'] });
    },
  });
}

export function useUpdateEnvironmentsOrder(workspaceId: string) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (environmentIds: string[]) => apiService.updateEnvironmentsOrder(workspaceId, environmentIds),
    onSuccess: () => {
      // Invalidate environment queries for this workspace
      queryClient.invalidateQueries({ queryKey: ['environments', workspaceId] });
    },
  });
}

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

/**
 * Drift Budget and Incident Management hooks
 */

/**
 * List drift budgets
 */
export function useDriftBudgets(params?: {
  endpoint?: string;
  method?: string;
  workspace_id?: string;
}) {
  return useQuery({
    queryKey: queryKeys.driftBudgets,
    queryFn: () => driftApi.listBudgets(params),
    staleTime: 30000,
  });
}

/**
 * Get a specific drift budget
 */
export function useDriftBudget(id: string) {
  return useQuery({
    queryKey: queryKeys.driftBudget(id),
    queryFn: () => driftApi.getBudget(id),
    enabled: !!id,
  });
}

/**
 * Create or update a drift budget
 */
export function useCreateOrUpdateDriftBudget() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (request: CreateDriftBudgetRequest) => driftApi.createOrUpdateBudget(request),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.driftBudgets });
    },
  });
}

/**
 * List incidents with optional filters
 */
export function useDriftIncidents(params?: ListIncidentsRequest, options?: { refetchInterval?: number }) {
  return useQuery({
    queryKey: queryKeys.driftIncidents(params),
    queryFn: () => driftApi.listIncidents(params),
    refetchInterval: options?.refetchInterval || 5000, // Auto-refresh every 5 seconds by default
    staleTime: 2000,
  });
}

/**
 * Get a specific incident
 */
export function useDriftIncident(id: string) {
  return useQuery({
    queryKey: queryKeys.driftIncident(id),
    queryFn: () => driftApi.getIncident(id),
    enabled: !!id,
    refetchInterval: 5000, // Auto-refresh for real-time updates
  });
}

/**
 * Update an incident
 */
export function useUpdateDriftIncident() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, request }: { id: string; request: UpdateIncidentRequest }) =>
      driftApi.updateIncident(id, request),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({ queryKey: queryKeys.driftIncident(variables.id) });
      queryClient.invalidateQueries({ queryKey: queryKeys.driftIncidents() });
      queryClient.invalidateQueries({ queryKey: queryKeys.driftIncidentStats });
    },
  });
}

/**
 * Resolve an incident
 */
export function useResolveDriftIncident() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (id: string) => driftApi.resolveIncident(id),
    onSuccess: (_, id) => {
      queryClient.invalidateQueries({ queryKey: queryKeys.driftIncident(id) });
      queryClient.invalidateQueries({ queryKey: queryKeys.driftIncidents() });
      queryClient.invalidateQueries({ queryKey: queryKeys.driftIncidentStats });
    },
  });
}

/**
 * Get incident statistics
 */
export function useDriftIncidentStatistics() {
  return useQuery({
    queryKey: queryKeys.driftIncidentStats,
    queryFn: () => driftApi.getIncidentStatistics(),
    refetchInterval: 10000, // Refetch every 10 seconds
    staleTime: 5000,
  });
}

/**
 * Time Travel hooks
 */
export function useTimeTravelStatus() {
  return useQuery({
    queryKey: queryKeys.timeTravelStatus,
    queryFn: () => timeTravelApi.getStatus(),
    refetchInterval: 2000, // Refetch every 2 seconds for real-time updates
    staleTime: 1000,
  });
}

/**
 * Hook to update persona lifecycle states based on virtual time
 * This should be called when time changes to update lifecycle states
 */
export function useUpdatePersonaLifecycles() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (workspace: string = 'default') => {
      const response = await fetch(`/api/v1/consistency/persona/update-lifecycles?workspace=${workspace}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
      });
      // Handle 405 (Method Not Allowed) gracefully - endpoint may not be implemented
      if (response.status === 405) {
        logger.debug('[TimeTravel] Persona lifecycle update endpoint not available (405)');
        return null; // Return null instead of throwing to prevent error UI
      }
      if (!response.ok) {
        throw new Error(`Failed to update persona lifecycles: ${response.status}`);
      }
      return response.json();
    },
    onSuccess: () => {
      // Invalidate relevant queries to refresh responses
      queryClient.invalidateQueries({ queryKey: ['consistency', 'state'] });
      queryClient.invalidateQueries({ queryKey: ['consistency', 'persona'] });
    },
    onError: (error) => {
      // Only log errors that aren't 405 (which we handle gracefully)
      if (!error.message?.includes('405')) {
        logger.warn('[TimeTravel] Failed to update persona lifecycles', error);
      }
    },
  });
}

/**
 * Hook that watches time changes and automatically updates persona lifecycle states
 * This provides live preview of persona/lifecycle state changes when virtual time is adjusted
 */
export function useLivePreviewLifecycleUpdates(workspace: string = 'default', enabled: boolean = true) {
  const { data: timeStatus } = useTimeTravelStatus();
  const updateLifecycles = useUpdatePersonaLifecycles();
  const previousTimeRef = React.useRef<string | undefined>();

  React.useEffect(() => {
    if (!enabled || !timeStatus?.enabled) {
      return;
    }

    const currentTime = timeStatus.current_time;

    // Check if time has changed
    if (currentTime && currentTime !== previousTimeRef.current) {
      previousTimeRef.current = currentTime;

      // Update persona lifecycle states based on new virtual time
      updateLifecycles.mutate(workspace, {
        onSuccess: () => {
          // Lifecycle states have been updated, responses will be refreshed automatically
          // via query invalidation in the mutation
        },
        onError: () => {
          // Silently handle errors (405 is expected if endpoint doesn't exist)
          // This prevents error UI from showing for missing endpoints
        },
      });
    }
  }, [timeStatus?.current_time, timeStatus?.enabled, enabled, workspace, updateLifecycles]);
}

export function useEnableTimeTravel() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ time, scale }: { time?: string; scale?: number }) =>
      timeTravelApi.enable(time, scale),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.timeTravelStatus });
    },
  });
}

export function useDisableTimeTravel() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: () => timeTravelApi.disable(),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.timeTravelStatus });
    },
  });
}

export function useAdvanceTime() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (duration: string) => timeTravelApi.advance(duration),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.timeTravelStatus });
    },
  });
}

export function useSetTime() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (time: string) => timeTravelApi.setTime(time),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.timeTravelStatus });
    },
  });
}

export function useSetTimeScale() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (scale: number) => timeTravelApi.setScale(scale),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.timeTravelStatus });
    },
  });
}

export function useResetTimeTravel() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: () => timeTravelApi.reset(),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.timeTravelStatus });
    },
  });
}

export function useCronJobs() {
  return useQuery({
    queryKey: queryKeys.cronJobs,
    queryFn: () => timeTravelApi.listCronJobs(),
    refetchInterval: 5000, // Refetch every 5 seconds
    staleTime: 2000,
  });
}

export function useMutationRules() {
  return useQuery({
    queryKey: queryKeys.mutationRules,
    queryFn: () => timeTravelApi.listMutationRules(),
    refetchInterval: 5000, // Refetch every 5 seconds
    staleTime: 2000,
  });
}

/**
 * Proxy replacement rules hooks
 */
export function useProxyRules() {
  return useQuery({
    queryKey: queryKeys.proxyRules,
    queryFn: () => proxyApi.getProxyRules(),
    staleTime: 10000, // Cache for 10 seconds
    refetchInterval: 5000, // Auto-refresh every 5 seconds
  });
}

export function useProxyRule(id: number) {
  return useQuery({
    queryKey: [...queryKeys.proxyRules, id],
    queryFn: () => proxyApi.getProxyRule(id),
    enabled: id !== undefined && id !== null,
    staleTime: 10000,
  });
}

export function useCreateProxyRule() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (rule: ProxyRuleRequest) => proxyApi.createProxyRule(rule),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.proxyRules });
    },
  });
}

export function useUpdateProxyRule() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, rule }: { id: number; rule: ProxyRuleRequest }) =>
      proxyApi.updateProxyRule(id, rule),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({ queryKey: queryKeys.proxyRules });
      queryClient.invalidateQueries({ queryKey: [...queryKeys.proxyRules, variables.id] });
    },
  });
}

export function useDeleteProxyRule() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (id: number) => proxyApi.deleteProxyRule(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.proxyRules });
    },
  });
}

export function useProxyInspect(limit?: number) {
  return useQuery({
    queryKey: [...queryKeys.proxyInspect, limit],
    queryFn: () => proxyApi.getProxyInspect(limit),
    staleTime: 2000, // Very short cache for real-time inspection
    refetchInterval: 2000, // Auto-refresh every 2 seconds
  });
}

/**
 * Reality Slider hooks
 */
export function useRealityLevel() {
  return useQuery({
    queryKey: queryKeys.realityLevel,
    queryFn: () => realityApi.getRealityLevel(),
    staleTime: 10000, // Consider data stale after 10 seconds
    refetchInterval: 30000, // Refetch every 30 seconds
  });
}

export function useSetRealityLevel() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (level: number) => realityApi.setRealityLevel(level),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.realityLevel });
      queryClient.invalidateQueries({ queryKey: queryKeys.dashboard });
    },
  });
}

export function useRealityPresets() {
  return useQuery({
    queryKey: queryKeys.realityPresets,
    queryFn: () => realityApi.listPresets(),
    staleTime: 60000, // Presets don't change often
  });
}

export function useImportRealityPreset() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (path: string) => realityApi.importPreset(path),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.realityLevel });
      queryClient.invalidateQueries({ queryKey: queryKeys.realityPresets });
      queryClient.invalidateQueries({ queryKey: queryKeys.dashboard });
    },
  });
}

export function useExportRealityPreset() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ name, description }: { name: string; description?: string }) =>
      realityApi.exportPreset(name, description),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.realityPresets });
    },
  });
}

/**
 * Lifecycle preset hooks
 */
export function useLifecyclePresets() {
  return useQuery({
    queryKey: queryKeys.lifecyclePresets,
    queryFn: () => consistencyApi.listLifecyclePresets(),
    staleTime: 60000, // Presets don't change often
  });
}

export function useLifecyclePresetDetails(presetName: string) {
  return useQuery({
    queryKey: queryKeys.lifecyclePreset(presetName),
    queryFn: () => consistencyApi.getLifecyclePresetDetails(presetName),
    enabled: !!presetName,
    staleTime: 60000,
  });
}

export function useApplyLifecyclePreset() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ workspace, personaId, preset }: { workspace: string; personaId: string; preset: string }) =>
      consistencyApi.applyLifecyclePreset(workspace, personaId, preset),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['consistency', 'state'] });
      queryClient.invalidateQueries({ queryKey: ['consistency', 'persona'] });
      queryClient.invalidateQueries({ queryKey: queryKeys.lifecyclePresets });
    },
  });
}
