import { useEffect, useRef, useState } from 'react';
import { logger } from '@/utils/logger';

export interface AutosaveOptions {
  /** Debounce delay in milliseconds (default: 2000) */
  delay?: number;
  /** Whether autosave is enabled (default: true) */
  enabled?: boolean;
  /** Callback when save starts */
  onSaveStart?: () => void;
  /** Callback when save completes successfully */
  onSaveSuccess?: () => void;
  /** Callback when save fails */
  onSaveError?: (error: Error) => void;
  /** Storage key for localStorage persistence (optional) */
  storageKey?: string;
}

/**
 * Hook for automatic form saving with debouncing
 * 
 * @example
 * ```tsx
 * const { save, isSaving, hasUnsavedChanges } = useAutosave({
 *   delay: 2000,
 *   onSave: async (data) => {
 *     await api.updateConfig(data);
 *   },
 *   storageKey: 'config-form'
 * });
 * 
 * // Call save whenever form data changes
 * useEffect(() => {
 *   save(formData);
 * }, [formData, save]);
 * ```
 */
export function useAutosave<T>(
  onSave: (data: T) => Promise<void> | void,
  options: AutosaveOptions = {}
) {
  const {
    delay = 2000,
    enabled = true,
    onSaveStart,
    onSaveSuccess,
    onSaveError,
    storageKey,
  } = options;

  const [isSaving, setIsSaving] = useState(false);
  const [hasUnsavedChanges, setHasUnsavedChanges] = useState(false);
  const [lastSaved, setLastSaved] = useState<Date | null>(null);
  const timeoutRef = useRef<NodeJS.Timeout | null>(null);
  const pendingDataRef = useRef<T | null>(null);
  const isInitialMount = useRef(true);

  // Load from localStorage on mount if storageKey is provided
  useEffect(() => {
    if (storageKey && isInitialMount.current) {
      try {
        const saved = localStorage.getItem(`autosave_${storageKey}`);
        if (saved) {
          const data = JSON.parse(saved);
          pendingDataRef.current = data;
          setHasUnsavedChanges(true);
          logger.info(`Loaded unsaved changes from localStorage for ${storageKey}`);
        }
      } catch (error) {
        logger.error(`Failed to load autosave data for ${storageKey}`, error);
      }
      isInitialMount.current = false;
    }
  }, [storageKey]);

  // Save to localStorage when data changes (if storageKey is provided)
  const saveToStorage = (data: T) => {
    if (storageKey) {
      try {
        localStorage.setItem(`autosave_${storageKey}`, JSON.stringify(data));
      } catch (error) {
        logger.error(`Failed to save to localStorage for ${storageKey}`, error);
      }
    }
  };

  // Clear localStorage after successful save
  const clearStorage = () => {
    if (storageKey) {
      try {
        localStorage.removeItem(`autosave_${storageKey}`);
      } catch (error) {
        logger.error(`Failed to clear localStorage for ${storageKey}`, error);
      }
    }
  };

  const performSave = async (data: T) => {
    if (!enabled) return;

    setIsSaving(true);
    setHasUnsavedChanges(false);
    onSaveStart?.();

    try {
      await onSave(data);
      setLastSaved(new Date());
      clearStorage();
      onSaveSuccess?.();
      logger.debug('Autosave successful');
    } catch (error) {
      const err = error instanceof Error ? error : new Error('Unknown error');
      setHasUnsavedChanges(true);
      saveToStorage(data);
      onSaveError?.(err);
      logger.error('Autosave failed', err);
    } finally {
      setIsSaving(false);
    }
  };

  const save = (data: T) => {
    if (!enabled) return;

    pendingDataRef.current = data;
    setHasUnsavedChanges(true);
    saveToStorage(data);

    // Clear existing timeout
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
    }

    // Set new timeout
    timeoutRef.current = setTimeout(() => {
      if (pendingDataRef.current !== null) {
        performSave(pendingDataRef.current);
        pendingDataRef.current = null;
      }
    }, delay);
  };

  // Manual save (immediate, no debounce)
  const saveNow = async (data: T) => {
    if (!enabled) return;

    // Clear any pending debounced save
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
      timeoutRef.current = null;
    }

    await performSave(data);
  };

  // Clear unsaved changes
  const clearUnsavedChanges = () => {
    setHasUnsavedChanges(false);
    pendingDataRef.current = null;
    clearStorage();
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
      timeoutRef.current = null;
    }
  };

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
      }
    };
  }, []);

  return {
    save,
    saveNow,
    isSaving,
    hasUnsavedChanges,
    lastSaved,
    clearUnsavedChanges,
  };
}
