import { logger } from '@/utils/logger';
import { useQuery } from '@tanstack/react-query';
import { dashboardApi } from '../../services/api';
import { queryKeys } from './queryKeys';

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
