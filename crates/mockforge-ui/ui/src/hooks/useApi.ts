/**
 * React hooks for MockForge Admin API
 * Uses React Query for caching, background refetching, and optimistic updates
 */

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
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
  type DashboardData,
  type RequestLog,
  type MetricsData,
  type FixtureInfo,
  type ValidationSettings,
  type LatencyProfile,
  type FaultConfig,
  type ProxyConfig,
  type ImportRequest,
  type ImportResponse,
  type ImportHistoryResponse,
} from '../services/api';

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
};

/**
 * Dashboard hooks
 */
export function useDashboard() {
  return useQuery({
    queryKey: queryKeys.dashboard,
    queryFn: dashboardApi.getDashboard,
    refetchInterval: 30000, // Refetch every 30 seconds
    staleTime: 10000, // Consider data stale after 10 seconds
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
    mutationFn: (reason?: string) => serverApi.restartServers(reason),
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
}) {
  return useQuery({
    queryKey: [...queryKeys.logs, params],
    queryFn: () => logsApi.getLogs(params),
    staleTime: 5000, // Logs can change frequently
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
    queryFn: metricsApi.getMetrics,
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
      filesApi.getFileContent(path, type),
  });
}

export function useSaveFileContent() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ path, content }: { path: string; content: string }) =>
      filesApi.saveFileContent(path, content),
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
    queryKey: queryKeys.fixtures,
    queryFn: fixturesApi.getFixtures,
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
    mutationFn: fixturesApi.renameFixture,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.fixtures });
    },
  });
}

export function useMoveFixture() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: fixturesApi.moveFixture,
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
