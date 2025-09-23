import { useState, useCallback } from 'react';

interface ErrorState {
  error: Error | null;
  isError: boolean;
}

export function useErrorHandler() {
  const [errorState, setErrorState] = useState<ErrorState>({
    error: null,
    isError: false,
  });

  const setError = useCallback((error: Error | string) => {
    const errorObj = typeof error === 'string' ? new Error(error) : error;
    setErrorState({
      error: errorObj,
      isError: true,
    });
  }, []);

  const clearError = useCallback(() => {
    setErrorState({
      error: null,
      isError: false,
    });
  }, []);

  const resetError = useCallback(() => {
    clearError();
  }, [clearError]);

  return {
    ...errorState,
    setError,
    clearError,
    resetError,
  };
}

/**
 * Hook for handling async operations with error states
 */
export function useAsyncErrorHandler() {
  const { error, isError, setError, clearError, resetError } = useErrorHandler();

  const wrapAsync = useCallback(
    async <T>(asyncFn: () => Promise<T>): Promise<T | null> => {
      try {
        clearError();
        return await asyncFn();
      } catch (err) {
        const error = err instanceof Error ? err : new Error(String(err));
        setError(error);
        return null;
      }
    },
    [clearError, setError]
  );

  return {
    error,
    isError,
    setError,
    clearError,
    resetError,
    wrapAsync,
  };
}

/**
 * Hook for handling form submissions with error states
 */
export function useFormErrorHandler() {
  const { error, isError, setError, clearError, resetError } = useErrorHandler();

  const handleSubmit = useCallback(
    async <T>(
      submitFn: () => Promise<T>,
      options?: {
        onSuccess?: (result: T) => void;
        onError?: (error: Error) => void;
        successMessage?: string;
      }
    ): Promise<T | null> => {
      try {
        clearError();
        const result = await submitFn();

        if (options?.onSuccess) {
          options.onSuccess(result);
        }

        return result;
      } catch (err) {
        const error = err instanceof Error ? err : new Error(String(err));
        setError(error);

        if (options?.onError) {
          options.onError(error);
        }

        return null;
      }
    },
    [clearError, setError]
  );

  return {
    error,
    isError,
    setError,
    clearError,
    resetError,
    handleSubmit,
  };
}
