import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { logsApi } from '../../services/api';
import { queryKeys } from './queryKeys';

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
