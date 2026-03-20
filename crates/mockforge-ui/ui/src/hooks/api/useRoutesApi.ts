import { useQuery } from '@tanstack/react-query';
import { routesApi } from '../../services/api';
import { queryKeys } from './queryKeys';

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
