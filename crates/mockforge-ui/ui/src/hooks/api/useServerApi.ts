import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { serverApi } from '../../services/api';
import { queryKeys } from './queryKeys';

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
