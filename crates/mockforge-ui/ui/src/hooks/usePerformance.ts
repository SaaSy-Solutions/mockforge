import { logger } from '@/utils/logger';
import { useEffect, useRef, useState } from 'react';

/**
 * Hook for monitoring component render performance
 */
export function useRenderPerformance(componentName: string) {
  const renderCount = useRef(0);
  const lastRenderTime = useRef(performance.now());
  const renderTimesRef = useRef<number[]>([]);
  const [averageTime, setAverageTime] = useState(0);

  useEffect(() => {
    const now = performance.now();
    const renderTime = now - lastRenderTime.current;
    renderCount.current += 1;

    // Update render times array
    renderTimesRef.current = [...renderTimesRef.current, renderTime].slice(-10);

    // Calculate average
    const avg = renderTimesRef.current.reduce((a, b) => a + b, 0) / renderTimesRef.current.length;
    setAverageTime(avg);

    lastRenderTime.current = now;

    // Log performance info in development
    if (import.meta.env.DEV && renderCount.current > 1) {
      logger.info(`[${componentName}] Render #${renderCount.current}, Time: ${renderTime.toFixed(2)}ms, Avg: ${avg.toFixed(2)}ms`);
    }
  });

  return {
    renderCount: renderCount.current,
    lastRenderTime: renderTimesRef.current[renderTimesRef.current.length - 1] || 0,
    averageRenderTime: averageTime,
  };
}

/**
 * Hook for monitoring API call performance
 */
export function useApiPerformance(apiName: string) {
  const [callTimes, setCallTimes] = useState<number[]>([]);
  const activeCalls = useRef(0);

  const startCall = () => {
    activeCalls.current += 1;
    return performance.now();
  };

  const endCall = (startTime: number) => {
    const duration = performance.now() - startTime;
    activeCalls.current -= 1;

    setCallTimes(prev => {
      const newTimes = [...prev, duration];
      // Keep only last 20 call times
      return newTimes.slice(-20);
    });

    // Log slow API calls
    if (duration > 1000 && import.meta.env.DEV) {
      logger.warn(`[${apiName}] Slow API call: ${duration.toFixed(2)}ms`);
    }

    return duration;
  };

  const stats = {
    totalCalls: callTimes.length,
    averageCallTime: callTimes.length > 0
      ? callTimes.reduce((a, b) => a + b, 0) / callTimes.length
      : 0,
    slowestCall: Math.max(...callTimes, 0),
    fastestCall: Math.min(...callTimes, 0),
    activeCalls: activeCalls.current,
  };

  return {
    startCall,
    endCall,
    stats,
  };
}

/**
 * Hook for debouncing values to reduce unnecessary re-renders
 */
export function useDebounce<T>(value: T, delay: number): T {
  const [debouncedValue, setDebouncedValue] = useState<T>(value);

  useEffect(() => {
    const handler = setTimeout(() => {
      setDebouncedValue(value);
    }, delay);

    return () => {
      clearTimeout(handler);
    };
  }, [value, delay]);

  return debouncedValue;
}

/**
 * Hook for throttling function calls
 */
export function useThrottle<T extends (...args: unknown[]) => unknown>(
  callback: T,
  delay: number
): T {
  const lastRan = useRef(Date.now());

  return ((...args) => {
    if (Date.now() - lastRan.current >= delay) {
      callback(...args);
      lastRan.current = Date.now();
    }
  }) as T;
}

/**
 * Hook for memoizing expensive computations
 */
export function useMemoizedComputation<T>(
  computeFn: () => T,
  dependencies: React.DependencyList
): T {
  const [result, setResult] = useState<T>(() => computeFn());
  const computationTime = useRef(0);

  useEffect(() => {
    const startTime = performance.now();
    const newResult = computeFn();
    const endTime = performance.now();

    computationTime.current = endTime - startTime;
    setResult(newResult);

    // Log slow computations
    if (computationTime.current > 50 && import.meta.env.DEV) {
      logger.warn(`Slow computation: ${computationTime.current.toFixed(2)}ms`);
    }
  }, dependencies);

  return result;
}

/**
 * Hook for virtual scrolling performance
 */
export function useVirtualScroll<T>(
  items: T[],
  itemHeight: number,
  containerHeight: number,
  overscan: number = 5
) {
  const [scrollTop, setScrollTop] = useState(0);

  const startIndex = Math.max(0, Math.floor(scrollTop / itemHeight) - overscan);
  const endIndex = Math.min(
    items.length - 1,
    Math.ceil((scrollTop + containerHeight) / itemHeight) + overscan
  );

  const visibleItems = items.slice(startIndex, endIndex + 1);
  const totalHeight = items.length * itemHeight;
  const offsetY = startIndex * itemHeight;

  return {
    visibleItems,
    totalHeight,
    offsetY,
    onScroll: (event: React.UIEvent<HTMLDivElement>) => {
      setScrollTop(event.currentTarget.scrollTop);
    },
  };
}
