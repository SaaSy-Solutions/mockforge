import React, { Suspense } from 'react';
import type { ComponentType } from 'react';
import { Skeleton } from './Skeleton';

interface LazyWrapperProps {
  children: React.ReactNode;
  fallback?: React.ReactNode;
}

/**
 * Wrapper component that provides consistent loading and error states
 * for lazy-loaded components
 */
export function LazyWrapper({
  children,
  fallback,
}: LazyWrapperProps) {
  const defaultFallback = (
    <div className="p-8 space-y-4 animate-fade-in">
      <div className="space-y-2">
        <Skeleton height={28} width="40%" />
        <Skeleton height={16} width="60%" />
      </div>
      <div className="space-y-3">
        <Skeleton height={20} />
        <Skeleton height={20} />
        <Skeleton height={20} width="80%" />
      </div>
    </div>
  );

  return (
    <Suspense fallback={fallback || defaultFallback}>
      {children}
    </Suspense>
  );
}

/**
 * Higher-order component for lazy loading React components
 * with automatic loading states
 */
export function withLazyLoading<P extends object>(
  importFn: () => Promise<{ default: ComponentType<P> }>,
  fallback?: React.ReactNode
) {
  const LazyComponent = React.lazy(importFn);

  return function LazyWrapperComponent(props: P) {
    return (
      <LazyWrapper fallback={fallback}>
        <LazyComponent {...props} />
      </LazyWrapper>
    );
  };
}

/**
 * Hook for lazy loading data with loading states
 */
export function useLazyData<T>(
  loadFn: () => Promise<T>,
  dependencies: React.DependencyList = []
): {
  data: T | null;
  loading: boolean;
  error: Error | null;
  refetch: () => Promise<void>;
} {
  const [data, setData] = React.useState<T | null>(null);
  const [loading, setLoading] = React.useState(false);
  const [error, setError] = React.useState<Error | null>(null);

  const loadData = React.useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const result = await loadFn();
      setData(result);
    } catch (err) {
      setError(err instanceof Error ? err : new Error(String(err)));
    } finally {
      setLoading(false);
    }
  }, []);

  React.useEffect(() => {
    loadData();
  }, dependencies);

  return {
    data,
    loading,
    error,
    refetch: loadData,
  };
}
