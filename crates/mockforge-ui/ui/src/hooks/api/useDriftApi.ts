import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { driftApi, type DriftIncident, type ListIncidentsRequest, type CreateDriftBudgetRequest, type UpdateIncidentRequest } from '../../services/driftApi';
import { queryKeys } from './queryKeys';

// Re-export drift types for consumers
export type { DriftIncident, ListIncidentsRequest, CreateDriftBudgetRequest, UpdateIncidentRequest };

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
