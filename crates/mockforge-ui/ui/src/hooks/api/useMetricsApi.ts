import { logger } from '@/utils/logger';
import { useQuery } from '@tanstack/react-query';
import { metricsApi } from '../../services/api';
import { queryKeys } from './queryKeys';

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
