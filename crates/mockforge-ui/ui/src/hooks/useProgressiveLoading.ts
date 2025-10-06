import { logger } from '@/utils/logger';
import { useState, useEffect, useCallback, useRef } from 'react';

interface UseProgressiveLoadingOptions<T> {
  loadData: (offset: number, limit: number) => Promise<T[]>;
  pageSize?: number;
  initialLoad?: boolean;
  dependencies?: unknown[];
}

interface UseProgressiveLoadingResult<T> {
  data: T[];
  isLoading: boolean;
  isLoadingMore: boolean;
  error: Error | null;
  hasMore: boolean;
  loadMore: () => Promise<void>;
  refresh: () => Promise<void>;
  setScrollRef: (element: HTMLElement | null) => void;
}

export function useProgressiveLoading<T>({
  loadData,
  pageSize = 50,
  initialLoad = true,
  dependencies = []
}: UseProgressiveLoadingOptions<T>): UseProgressiveLoadingResult<T> {
  const [data, setData] = useState<T[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [isLoadingMore, setIsLoadingMore] = useState(false);
  const [error, setError] = useState<Error | null>(null);
  const [hasMore, setHasMore] = useState(true);
  const [offset, setOffset] = useState(0);
  
  const scrollElementRef = useRef<HTMLElement | null>(null);
  const observerRef = useRef<IntersectionObserver | null>(null);
  const sentinelRef = useRef<HTMLDivElement | null>(null);

  const loadMore = useCallback(async () => {
    if (isLoadingMore || !hasMore) return;

    try {
      setIsLoadingMore(true);
      setError(null);
      
      const newData = await loadData(offset, pageSize);
      
      if (newData.length < pageSize) {
        setHasMore(false);
      }
      
      setData(prev => [...prev, ...newData]);
      setOffset(prev => prev + newData.length);
    } catch (err) {
      setError(err instanceof Error ? err : new Error('Failed to load more data'));
    } finally {
      setIsLoadingMore(false);
    }
  }, [loadData, offset, pageSize, isLoadingMore, hasMore]);

  const refresh = useCallback(async () => {
    try {
      setIsLoading(true);
      setError(null);
      setData([]);
      setOffset(0);
      setHasMore(true);
      
      const newData = await loadData(0, pageSize);
      
      if (newData.length < pageSize) {
        setHasMore(false);
      }
      
      setData(newData);
      setOffset(newData.length);
    } catch (err) {
      setError(err instanceof Error ? err : new Error('Failed to load data'));
    } finally {
      setIsLoading(false);
    }
  }, [loadData, pageSize]);

  // Initial load effect
  useEffect(() => {
    if (initialLoad) {
      refresh();
    }
  }, dependencies);

  // Set up intersection observer for infinite scroll
  const setScrollRef = useCallback((element: HTMLElement | null) => {
    scrollElementRef.current = element;

    // Clean up previous observer
    if (observerRef.current) {
      observerRef.current.disconnect();
    }

    if (!element) return;

    // Create sentinel element if it doesn't exist
    if (!sentinelRef.current) {
      sentinelRef.current = document.createElement('div');
      sentinelRef.current.style.height = '1px';
      sentinelRef.current.style.marginTop = '20px';
    }

    // Add sentinel to the scroll container
    element.appendChild(sentinelRef.current);

    // Set up intersection observer
    observerRef.current = new IntersectionObserver(
      (entries) => {
        const [entry] = entries;
        if (entry.isIntersecting && hasMore && !isLoadingMore) {
          loadMore();
        }
      },
      {
        root: element,
        rootMargin: '100px', // Load when sentinel is 100px away from viewport
        threshold: 0.1
      }
    );

    observerRef.current.observe(sentinelRef.current);
  }, [hasMore, isLoadingMore, loadMore]);

  // Cleanup
  useEffect(() => {
    return () => {
      if (observerRef.current) {
        observerRef.current.disconnect();
      }
      if (sentinelRef.current && scrollElementRef.current) {
        scrollElementRef.current.removeChild(sentinelRef.current);
      }
    };
  }, []);

  return {
    data,
    isLoading,
    isLoadingMore,
    error,
    hasMore,
    loadMore,
    refresh,
    setScrollRef
  };
}

// Simplified hook for virtualized lists (alternative approach)
export function useVirtualizedLoading<T>({
  loadData,
  pageSize = 50,
  bufferSize = 10, // How many items to render outside viewport
  itemHeight = 60, // Height of each item in pixels
  dependencies = []
}: {
  loadData: (offset: number, limit: number) => Promise<T[]>;
  pageSize?: number;
  bufferSize?: number;
  itemHeight?: number;
  dependencies?: unknown[];
}) {
  const [data, setData] = useState<T[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);
  const [hasMore, setHasMore] = useState(true);
  const [visibleRange, setVisibleRange] = useState({ start: 0, end: pageSize });

  const loadPage = useCallback(async (offset: number) => {
    if (isLoading) return;

    try {
      setIsLoading(true);
      setError(null);
      
      const newData = await loadData(offset, pageSize);
      
      if (newData.length < pageSize) {
        setHasMore(false);
      }

      setData(prev => {
        const updated = [...prev];
        newData.forEach((item, index) => {
          updated[offset + index] = item;
        });
        return updated;
      });
    } catch (err) {
      setError(err instanceof Error ? err : new Error('Failed to load data'));
    } finally {
      setIsLoading(false);
    }
  }, [loadData, pageSize, isLoading]);

  const updateVisibleRange = useCallback((scrollTop: number, containerHeight: number) => {
    const start = Math.max(0, Math.floor(scrollTop / itemHeight) - bufferSize);
    const end = Math.min(
      data.length,
      Math.ceil((scrollTop + containerHeight) / itemHeight) + bufferSize
    );

    setVisibleRange({ start, end });

    // Load more data if we're approaching the end
    if (end > data.length - pageSize && hasMore && !isLoading) {
      loadPage(data.length);
    }
  }, [itemHeight, bufferSize, data.length, hasMore, isLoading, loadPage, pageSize]);

  // Initial load
  useEffect(() => {
    loadPage(0);
  }, dependencies);

  return {
    data,
    isLoading,
    error,
    hasMore,
    visibleRange,
    updateVisibleRange,
    totalHeight: data.length * itemHeight,
    itemHeight
  };
}