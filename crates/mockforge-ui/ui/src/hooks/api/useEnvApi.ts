import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { apiService } from '../../services/api';
import type {
  CreateEnvironmentRequest,
  UpdateEnvironmentRequest,
  SetVariableRequest,
  AutocompleteRequest,
} from '../../types';

/**
 * Environment management hooks (workspace-scoped environments)
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
