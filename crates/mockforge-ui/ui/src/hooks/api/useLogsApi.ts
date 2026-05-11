import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { logsApi } from '../../services/api';
import { cloudLogsApi } from '../../services/api/cloudLogs';
import { isCloudMode } from '../../utils/cloudMode';
import { useWorkspaceStore } from '../../stores/useWorkspaceStore';
import { queryKeys } from './queryKeys';

/**
 * Logs hooks. Cloud mode dispatches to `cloudLogsApi` against the active
 * workspace; local mode hits `/__mockforge/logs` on the in-process server.
 */
export function useLogs(params?: {
  method?: string;
  path?: string;
  status?: number;
  limit?: number;
  refetchInterval?: number;
}) {
  const { refetchInterval, ...apiParams } = params || {};
  const cloudMode = isCloudMode();
  const workspaceId = useWorkspaceStore((s) => s.activeWorkspace?.id);
  return useQuery({
    queryKey: [...queryKeys.logs, cloudMode ? `cloud:${workspaceId ?? ''}` : 'local', apiParams],
    queryFn: () => {
      if (cloudMode) {
        if (!workspaceId) return Promise.resolve([]);
        return cloudLogsApi.getLogs(workspaceId, {
          method: apiParams.method,
          path: apiParams.path,
          status: apiParams.status != null ? String(apiParams.status) : undefined,
          limit: apiParams.limit,
        });
      }
      return logsApi.getLogs(apiParams);
    },
    staleTime: 5000, // Logs can change frequently
    refetchInterval: refetchInterval, // Optional auto-refetch interval
    enabled: cloudMode ? !!workspaceId : true,
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
