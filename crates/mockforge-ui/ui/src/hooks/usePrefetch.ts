import { useQueryClient } from '@tanstack/react-query';
import { useCallback, useEffect } from 'react';
import { queryKeys } from './useApi';

/**
 * Hook for prefetching data that might be needed soon
 * Useful for improving perceived performance
 */
export function usePrefetch() {
  const queryClient = useQueryClient();

  const prefetchDashboard = useCallback(() => {
    queryClient.prefetchQuery({
      queryKey: queryKeys.dashboard,
      staleTime: 5 * 60 * 1000, // 5 minutes
    });
  }, [queryClient]);

  const prefetchMetrics = useCallback(() => {
    queryClient.prefetchQuery({
      queryKey: queryKeys.metrics,
      staleTime: 2 * 60 * 1000, // 2 minutes
    });
  }, [queryClient]);

  const prefetchLogs = useCallback(() => {
    queryClient.prefetchQuery({
      queryKey: [...queryKeys.logs],
      staleTime: 1 * 60 * 1000, // 1 minute
    });
  }, [queryClient]);

  const prefetchConfig = useCallback(() => {
    queryClient.prefetchQuery({
      queryKey: queryKeys.config,
      staleTime: 10 * 60 * 1000, // 10 minutes (config changes rarely)
    });
  }, [queryClient]);

  // Prefetch all common data
  const prefetchAll = useCallback(() => {
    prefetchDashboard();
    prefetchMetrics();
    prefetchLogs();
    prefetchConfig();
  }, [prefetchDashboard, prefetchMetrics, prefetchLogs, prefetchConfig]);

  return {
    prefetchDashboard,
    prefetchMetrics,
    prefetchLogs,
    prefetchConfig,
    prefetchAll,
  };
}

/**
 * Hook that prefetches data on app startup
 */
export function useStartupPrefetch() {
  const { prefetchAll } = usePrefetch();

  useEffect(() => {
    // Small delay to not interfere with initial page load
    const timer = setTimeout(() => {
      prefetchAll();
    }, 1000);

    return () => clearTimeout(timer);
  }, [prefetchAll]);
}

/**
 * Hook that prefetches data when user hovers over navigation items
 */
export function useHoverPrefetch(routeName: string) {
  const { prefetchDashboard, prefetchMetrics, prefetchLogs, prefetchConfig } = usePrefetch();

  const handleMouseEnter = useCallback(() => {
    switch (routeName) {
      case 'dashboard':
        prefetchDashboard();
        break;
      case 'metrics':
        prefetchMetrics();
        break;
      case 'logs':
        prefetchLogs();
        break;
      case 'config':
        prefetchConfig();
        break;
    }
  }, [routeName, prefetchDashboard, prefetchMetrics, prefetchLogs, prefetchConfig]);

  return handleMouseEnter;
}

/**
 * Hook that prefetches next page data when user scrolls near bottom
 */
export function useScrollPrefetch(threshold: number = 200) {
  const { prefetchLogs } = usePrefetch();

  useEffect(() => {
    const handleScroll = () => {
      const scrollTop = window.scrollY;
      const windowHeight = window.innerHeight;
      const documentHeight = document.documentElement.scrollHeight;

      // If user is within threshold of bottom, prefetch more data
      if (documentHeight - (scrollTop + windowHeight) < threshold) {
        prefetchLogs();
      }
    };

    window.addEventListener('scroll', handleScroll, { passive: true });
    return () => window.removeEventListener('scroll', handleScroll);
  }, [prefetchLogs, threshold]);
}
