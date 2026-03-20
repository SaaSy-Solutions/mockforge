import { logger } from '@/utils/logger';
import React from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { timeTravelApi } from '../../services/api';
import { queryKeys } from './queryKeys';

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
