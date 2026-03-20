import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { configApi, envApi } from '../../services/api';
import { queryKeys } from './queryKeys';
import { useValidation } from './useValidationApi';

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
 * Environment variables hooks (global env vars, not workspace-scoped)
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
 * Combined configuration hook
 */
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
