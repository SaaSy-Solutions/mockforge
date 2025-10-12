import { logger } from '@/utils/logger';
import { useState, useCallback, useRef } from 'react';
import { useQueryClient } from '@tanstack/react-query';

export interface OptimisticUpdate<T> {
  id: string;
  type: 'create' | 'update' | 'delete';
  data: T;
  timestamp: number;
  rollback?: () => void;
}

export interface UseOptimisticUpdatesOptions<T> {
  queryKey: unknown[];
  getId: (item: T) => string;
  onSuccess?: (data: T) => void;
  onError?: (error: Error, rollback: () => void) => void;
}

export function useOptimisticUpdates<T>({
  queryKey,
  getId,
  onSuccess,
  onError,
}: UseOptimisticUpdatesOptions<T>) {
  const queryClient = useQueryClient();
  const [pendingUpdates, setPendingUpdates] = useState<Map<string, OptimisticUpdate<T>>>(new Map());
  const rollbackTimeouts = useRef<Map<string, ReturnType<typeof setTimeout>>>(new Map());

  const applyOptimisticUpdate = useCallback(
    (type: OptimisticUpdate<T>['type'], data: T, rollbackDelay = 5000) => {
      const id = getId(data);
      const timestamp = Date.now();

      // Clear any existing timeout for this item
      const existingTimeout = rollbackTimeouts.current.get(id);
      if (existingTimeout) {
        clearTimeout(existingTimeout);
      }

      // Get current query data
      const currentData = queryClient.getQueryData<T[]>(queryKey) || [];

      // Create rollback function
      const rollback = () => {
        queryClient.setQueryData(queryKey, currentData);
        setPendingUpdates(prev => {
          const newMap = new Map(prev);
          newMap.delete(id);
          return newMap;
        });
        rollbackTimeouts.current.delete(id);
      };

      // Apply optimistic update
      let newData: T[];
      switch (type) {
        case 'create':
          newData = [...currentData, data];
          break;
        case 'update':
          newData = currentData.map(item => getId(item) === id ? data : item);
          break;
        case 'delete':
          newData = currentData.filter(item => getId(item) !== id);
          break;
        default:
          newData = currentData;
      }

      queryClient.setQueryData(queryKey, newData);

      // Track the optimistic update
      const update: OptimisticUpdate<T> = {
        id,
        type,
        data,
        timestamp,
        rollback,
      };

      setPendingUpdates(prev => new Map(prev.set(id, update)));

      // Set auto-rollback timeout
      const timeout = setTimeout(() => {
        if (onError) {
          onError(new Error('Optimistic update timed out'), rollback);
        } else {
          rollback();
        }
      }, rollbackDelay);

      rollbackTimeouts.current.set(id, timeout);

      return {
        rollback,
        confirm: () => confirmUpdate(id),
      };
    },
    [queryKey, queryClient, getId, onError]
  );

  const confirmUpdate = useCallback((id: string) => {
    const update = pendingUpdates.get(id);
    if (update) {
      // Clear timeout
      const timeout = rollbackTimeouts.current.get(id);
      if (timeout) {
        clearTimeout(timeout);
        rollbackTimeouts.current.delete(id);
      }

      // Remove from pending updates
      setPendingUpdates(prev => {
        const newMap = new Map(prev);
        newMap.delete(id);
        return newMap;
      });

      // Call success callback
      if (onSuccess) {
        onSuccess(update.data);
      }
    }
  }, [pendingUpdates, onSuccess]);

  const rollbackUpdate = useCallback((id: string) => {
    const update = pendingUpdates.get(id);
    if (update && update.rollback) {
      update.rollback();
    }
  }, [pendingUpdates]);

  const rollbackAll = useCallback(() => {
    Array.from(pendingUpdates.values()).forEach(update => {
      if (update.rollback) {
        update.rollback();
      }
    });
  }, [pendingUpdates]);

  const isPending = useCallback((id: string) => {
    return pendingUpdates.has(id);
  }, [pendingUpdates]);

  const getPendingUpdate = useCallback((id: string) => {
    return pendingUpdates.get(id);
  }, [pendingUpdates]);

  // Cleanup timeouts on unmount
  const cleanup = useCallback(() => {
    Array.from(rollbackTimeouts.current.values()).forEach(timeout => {
      clearTimeout(timeout);
    });
    rollbackTimeouts.current.clear();
  }, []);

  return {
    applyOptimisticUpdate,
    confirmUpdate,
    rollbackUpdate,
    rollbackAll,
    isPending,
    getPendingUpdate,
    pendingUpdates: Array.from(pendingUpdates.values()),
    cleanup,
  };
}

// Specific hook for common CRUD operations
export function useOptimisticCrud<T>(options: UseOptimisticUpdatesOptions<T>) {
  const optimistic = useOptimisticUpdates(options);

  const optimisticCreate = useCallback(
    (data: T) => optimistic.applyOptimisticUpdate('create', data),
    [optimistic]
  );

  const optimisticUpdate = useCallback(
    (data: T) => optimistic.applyOptimisticUpdate('update', data),
    [optimistic]
  );

  const optimisticDelete = useCallback(
    (data: T) => optimistic.applyOptimisticUpdate('delete', data),
    [optimistic]
  );

  return {
    ...optimistic,
    optimisticCreate,
    optimisticUpdate,
    optimisticDelete,
  };
}

// Hook for optimistic toggle states (like favorites, likes, etc.)
export function useOptimisticToggle(
  initialValue: boolean,
  onToggle: (newValue: boolean) => Promise<boolean>,
  rollbackDelay = 3000
) {
  const [optimisticValue, setOptimisticValue] = useState(initialValue);
  const [isPending, setIsPending] = useState(false);
  const rollbackTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const toggle = useCallback(async () => {
    if (isPending) return;

    const previousValue = optimisticValue;
    const newValue = !optimisticValue;

    // Apply optimistic update
    setOptimisticValue(newValue);
    setIsPending(true);

    // Set rollback timeout
    rollbackTimeoutRef.current = setTimeout(() => {
      setOptimisticValue(previousValue);
      setIsPending(false);
    }, rollbackDelay);

    try {
      const actualValue = await onToggle(newValue);

      // Clear timeout
      if (rollbackTimeoutRef.current !== null) {
        clearTimeout(rollbackTimeoutRef.current);
      }

      // Update to actual value from server
      setOptimisticValue(actualValue);
      setIsPending(false);
    } catch (error) {
      // Rollback on error
      if (rollbackTimeoutRef.current !== null) {
        clearTimeout(rollbackTimeoutRef.current);
      }
      setOptimisticValue(previousValue);
      setIsPending(false);
      throw error;
    }
  }, [optimisticValue, isPending, onToggle, rollbackDelay]);

  const setValue = useCallback((value: boolean) => {
    if (rollbackTimeoutRef.current !== null) {
      clearTimeout(rollbackTimeoutRef.current);
    }
    setOptimisticValue(value);
    setIsPending(false);
  }, []);

  return {
    value: optimisticValue,
    isPending,
    toggle,
    setValue,
  };
}
