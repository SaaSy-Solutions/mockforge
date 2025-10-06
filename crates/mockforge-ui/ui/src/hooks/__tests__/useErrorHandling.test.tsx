/**
 * @jest-environment jsdom
 */

import { renderHook, act } from '@testing-library/react';
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { useErrorHandling, useApiErrorHandling } from '../useErrorHandling';
import React from 'react';

// Create a wrapper component with QueryClientProvider
const createWrapper = () => {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });
  return ({ children }: { children: React.ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
};

describe('useErrorHandling', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.spyOn(console, 'error').mockImplementation(() => {});
    vi.spyOn(console, 'warn').mockImplementation(() => {});
  });

  it('initializes with empty error state', () => {
    const { result } = renderHook(() => useErrorHandling(), { wrapper: createWrapper() });

    expect(result.current.errorState.error).toBeNull();
    expect(result.current.errorState.isRetrying).toBe(false);
    expect(result.current.errorState.retryCount).toBe(0);
  });

  it('handles errors correctly', () => {
    const { result } = renderHook(() => useErrorHandling(), { wrapper: createWrapper() });
    const testError = new Error('Test error');

    act(() => {
      result.current.handleError(testError, { operation: 'test', component: 'TestComponent' });
    });

    expect(result.current.errorState.error).toEqual(testError);
    expect(result.current.errorState.context?.operation).toBe('test');
    expect(result.current.errorState.context?.component).toBe('TestComponent');
  });

  it('clears error state', () => {
    const { result } = renderHook(() => useErrorHandling(), { wrapper: createWrapper() });

    act(() => {
      result.current.handleError(new Error('Test'), { operation: 'test' });
    });

    expect(result.current.errorState.error).not.toBeNull();

    act(() => {
      result.current.clearError();
    });

    expect(result.current.errorState.error).toBeNull();
    expect(result.current.errorState.retryCount).toBe(0);
  });

  it('detects network errors', () => {
    const { result } = renderHook(() => useErrorHandling(), { wrapper: createWrapper() });

    expect(result.current.isNetworkError(new Error('fetch failed'))).toBe(true);
    expect(result.current.isNetworkError(new Error('network error'))).toBe(true);
    expect(result.current.isNetworkError(new Error('Failed to load'))).toBe(true);
    expect(result.current.isNetworkError(new Error('other error'))).toBe(false);
  });

  it('detects timeout errors', () => {
    const { result } = renderHook(() => useErrorHandling(), { wrapper: createWrapper() });

    expect(result.current.isTimeoutError(new Error('timeout'))).toBe(true);
    expect(result.current.isTimeoutError(new Error('Request Timeout'))).toBe(true);
    expect(result.current.isTimeoutError(new Error('other error'))).toBe(false);
  });

  it('categorizes error types correctly', () => {
    const { result } = renderHook(() => useErrorHandling(), { wrapper: createWrapper() });

    expect(result.current.getErrorType(new Error('network error'))).toBe('network');
    expect(result.current.getErrorType(new Error('timeout'))).toBe('timeout');
    expect(result.current.getErrorType(new Error('generic'))).toBe('generic');
  });

  it('tracks retry count and enforces max retries', async () => {
    const { result } = renderHook(() => useErrorHandling(2), { wrapper: createWrapper() });
    const failedOp = vi.fn().mockRejectedValue(new Error('Failed'));

    act(() => {
      result.current.handleError(new Error('Test'), { operation: 'test' });
    });

    expect(result.current.canRetry).toBe(true);

    // First retry fails, increments count
    await act(async () => {
      await result.current.retry(failedOp);
    });

    expect(result.current.errorState.retryCount).toBe(1);
    expect(result.current.canRetry).toBe(true);

    // Second retry fails, increments count
    await act(async () => {
      await result.current.retry(failedOp);
    });

    expect(result.current.errorState.retryCount).toBe(2);
    expect(result.current.canRetry).toBe(false);
  });

  it('successfully retries operation', async () => {
    const { result } = renderHook(() => useErrorHandling(), { wrapper: createWrapper() });
    const successfulOperation = vi.fn().mockResolvedValue('success');

    act(() => {
      result.current.handleError(new Error('Test'), { operation: 'test' });
    });

    expect(result.current.errorState.error).not.toBeNull();

    await act(async () => {
      await result.current.retry(successfulOperation);
    });

    expect(successfulOperation).toHaveBeenCalled();
    expect(result.current.errorState.error).toBeNull();
    expect(result.current.errorState.retryCount).toBe(0);
  });

  it('handles retry failure', async () => {
    const { result } = renderHook(() => useErrorHandling(), { wrapper: createWrapper() });
    const failedOperation = vi.fn().mockRejectedValue(new Error('Retry failed'));

    act(() => {
      result.current.handleError(new Error('Initial error'), { operation: 'test' });
    });

    await act(async () => {
      await result.current.retry(failedOperation);
    });

    expect(result.current.errorState.error?.message).toBe('Retry failed');
    expect(result.current.errorState.isRetrying).toBe(false);
  });
});

describe('useApiErrorHandling', () => {
  it('handles API errors', () => {
    const { result } = renderHook(() => useApiErrorHandling(), { wrapper: createWrapper() });

    act(() => {
      result.current.handleApiError(new Error('API error'), 'fetchData');
    });

    expect(result.current.errorState.error?.message).toBe('API error');
    expect(result.current.errorState.context?.operation).toBe('fetchData');
    expect(result.current.errorState.context?.component).toBe('API');
  });

  it('retries API calls successfully', async () => {
    const { result } = renderHook(() => useApiErrorHandling(), { wrapper: createWrapper() });
    const apiCall = vi.fn().mockResolvedValue({ data: 'success' });

    act(() => {
      result.current.handleApiError(new Error('Initial error'), 'fetchData');
    });

    const response = await act(async () => {
      return await result.current.retryApiCall(apiCall);
    });

    expect(response).toEqual({ data: 'success' });
    expect(result.current.errorState.error).toBeNull();
  });

  it('handles API call retry failure', async () => {
    const { result } = renderHook(() => useApiErrorHandling(), { wrapper: createWrapper() });
    const apiCall = vi.fn().mockRejectedValue(new Error('API failed'));

    let thrownError: Error | undefined;

    await act(async () => {
      try {
        await result.current.retryApiCall(apiCall);
      } catch (error) {
        thrownError = error as Error;
      }
    });

    expect(thrownError?.message).toBe('API failed');
    expect(result.current.errorState.error?.message).toBe('API failed');
  });
});
