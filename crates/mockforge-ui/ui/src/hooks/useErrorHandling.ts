import { logger } from '@/utils/logger';
import { useCallback, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useToastStore } from '@/stores/useToastStore';

export interface ErrorContext {
  operation: string;
  component?: string;
  retryable?: boolean;
  timestamp: number;
  /** Whether to show a toast notification for this error */
  showToast?: boolean;
}

export interface ErrorState {
  error: Error | null;
  isRetrying: boolean;
  retryCount: number;
  context?: ErrorContext;
}

export function useErrorHandling(maxRetries = 3) {
  const [errorState, setErrorState] = useState<ErrorState>({
    error: null,
    isRetrying: false,
    retryCount: 0,
  });

  const queryClient = useQueryClient();
  const { error: showErrorToast } = useToastStore();

  const handleError = useCallback((error: Error, context?: Partial<ErrorContext>) => {
    const showToast = context?.showToast ?? true;

    setErrorState(prev => ({
      ...prev,
      error,
      context: {
        operation: context?.operation || 'unknown',
        component: context?.component,
        retryable: context?.retryable ?? true,
        timestamp: Date.now(),
        showToast,
      },
    }));

    // Log error for monitoring
    logger.error('Error handled',{
      error: error.message,
      stack: error.stack,
      context,
    });

    // Show toast notification for the error
    if (showToast) {
      const title = context?.operation
        ? `Failed to ${context.operation}`
        : 'An error occurred';
      showErrorToast(title, error.message);
    }

    // In production, you might want to send this to an error tracking service
    // logErrorToService(error, context);
  }, [showErrorToast]);

  const retry = useCallback(async (operation?: () => Promise<void>) => {
    if (errorState.retryCount >= maxRetries) {
      logger.warn('Max retries reached, not retrying');
      return;
    }

    setErrorState(prev => ({
      ...prev,
      isRetrying: true,
      retryCount: prev.retryCount + 1,
    }));

    try {
      if (operation) {
        await operation();
      } else {
        // Default retry behavior: invalidate all queries
        await queryClient.invalidateQueries();
      }

      // Clear error on successful retry
      setErrorState({
        error: null,
        isRetrying: false,
        retryCount: 0,
      });
    } catch (error) {
      setErrorState(prev => ({
        ...prev,
        error: error instanceof Error ? error : new Error('Retry failed'),
        isRetrying: false,
      }));
    }
  }, [errorState.retryCount, maxRetries, queryClient]);

  const clearError = useCallback(() => {
    setErrorState({
      error: null,
      isRetrying: false,
      retryCount: 0,
    });
  }, []);

  const isNetworkError = useCallback((error: Error) => {
    return error.message.includes('fetch') ||
           error.message.includes('network') ||
           error.message.includes('Failed to load');
  }, []);

  const isTimeoutError = useCallback((error: Error) => {
    return error.message.includes('timeout') ||
           error.message.includes('Timeout');
  }, []);

  const getErrorType = useCallback((error: Error) => {
    if (isNetworkError(error)) return 'network';
    if (isTimeoutError(error)) return 'timeout';
    return 'generic';
  }, [isNetworkError, isTimeoutError]);

  return {
    errorState,
    handleError,
    retry,
    clearError,
    isNetworkError,
    isTimeoutError,
    getErrorType,
    canRetry: errorState.retryCount < maxRetries && errorState.context?.retryable !== false,
  };
}

// Hook for API error handling specifically
export function useApiErrorHandling() {
  const { handleError, retry, clearError, errorState, getErrorType, canRetry } = useErrorHandling();

  const handleApiError = useCallback((error: unknown, operation: string) => {
    let errorObj: Error;

    if (error instanceof Error) {
      errorObj = error;
    } else if (typeof error === 'string') {
      errorObj = new Error(error);
    } else {
      errorObj = new Error('An unknown error occurred');
    }

    handleError(errorObj, {
      operation,
      component: 'API',
      retryable: true,
    });
  }, [handleError]);

  const retryApiCall = useCallback(async (apiCall: () => Promise<unknown>) => {
    try {
      const result = await apiCall();
      clearError();
      return result;
    } catch (error) {
      handleApiError(error, 'retry');
      throw error;
    }
  }, [clearError, handleApiError]);

  return {
    errorState,
    handleApiError,
    retry,
    retryApiCall,
    clearError,
    getErrorType,
    canRetry,
  };
}

// Global error handler for unhandled promise rejections
export function setupGlobalErrorHandlers() {
  if (typeof window !== 'undefined') {
    window.addEventListener('unhandledrejection', (event) => {
      logger.error('Unhandled promise rejection',event.reason);

      // Prevent the default browser behavior
      event.preventDefault();

      // In production, you might want to send this to an error tracking service
      // logErrorToService(new Error(event.reason), { operation: 'unhandled_rejection' });
    });

    window.addEventListener('error', (event) => {
      logger.error('Global error',event.error);

      // In production, you might want to send this to an error tracking service
      // logErrorToService(event.error, { operation: 'global_error' });
    });
  }
}

// Utility for wrapping async operations with error handling
export function withErrorHandling<T extends unknown[], R>(
  fn: (...args: T) => Promise<R>,
  errorHandler: (error: Error) => void,
  context?: Partial<ErrorContext>
) {
  return async (...args: T): Promise<R | undefined> => {
    try {
      return await fn(...args);
    } catch (error) {
      const errorObj = error instanceof Error ? error : new Error(String(error));
      errorHandler(errorObj);
      logger.error('Error in wrapped function', errorObj,context);
      return undefined;
    }
  };
}
