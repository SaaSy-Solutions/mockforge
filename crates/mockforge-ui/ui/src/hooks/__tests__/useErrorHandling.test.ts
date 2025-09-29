/**
 * @jest-environment jsdom
 */

import { renderHook, act } from '@testing-library/react';
import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import { useErrorHandling } from '../useErrorHandling';

// Mock the error reporting service
const mockReportError = vi.fn();
const mockClearErrors = vi.fn();
vi.mock('../../services/errorReporting', () => ({
  reportError: mockReportError,
  clearErrors: mockClearErrors,
}));

// Mock toast notifications
const mockToast = {
  error: vi.fn(),
  success: vi.fn(),
  warning: vi.fn(),
  info: vi.fn(),
};
vi.mock('sonner', () => ({
  toast: mockToast,
}));

describe('useErrorHandling', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Mock console methods to avoid noise in tests
    vi.spyOn(console, 'error').mockImplementation(() => {});
    vi.spyOn(console, 'warn').mockImplementation(() => {});
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('initializes with empty error state', () => {
    const { result } = renderHook(() => useErrorHandling());

    expect(result.current.errors).toEqual([]);
    expect(result.current.hasErrors).toBe(false);
    expect(result.current.latestError).toBeNull();
  });

  it('adds error and reports it', () => {
    const { result } = renderHook(() => useErrorHandling());

    act(() => {
      result.current.addError('Test error message');
    });

    expect(result.current.errors).toHaveLength(1);
    expect(result.current.errors[0]).toMatchObject({
      message: 'Test error message',
      timestamp: expect.any(Date),
      id: expect.any(String),
      severity: 'error',
    });
    expect(result.current.hasErrors).toBe(true);
    expect(result.current.latestError?.message).toBe('Test error message');

    expect(mockReportError).toHaveBeenCalledWith({
      message: 'Test error message',
      severity: 'error',
      timestamp: expect.any(Date),
      context: undefined,
    });

    expect(mockToast.error).toHaveBeenCalledWith('Test error message');
  });

  it('adds error with custom severity', () => {
    const { result } = renderHook(() => useErrorHandling());

    act(() => {
      result.current.addError('Warning message', 'warning');
    });

    expect(result.current.errors[0].severity).toBe('warning');
    expect(mockToast.warning).toHaveBeenCalledWith('Warning message');
  });

  it('adds error with context information', () => {
    const { result } = renderHook(() => useErrorHandling());
    const context = { component: 'TestComponent', action: 'save' };

    act(() => {
      result.current.addError('Context error', 'error', context);
    });

    expect(result.current.errors[0].context).toEqual(context);
    expect(mockReportError).toHaveBeenCalledWith({
      message: 'Context error',
      severity: 'error',
      timestamp: expect.any(Date),
      context,
    });
  });

  it('handles Error objects', () => {
    const { result } = renderHook(() => useErrorHandling());
    const error = new Error('Test error object');
    error.stack = 'Error stack trace';

    act(() => {
      result.current.handleError(error);
    });

    expect(result.current.errors[0]).toMatchObject({
      message: 'Test error object',
      stack: 'Error stack trace',
      severity: 'error',
    });

    expect(mockReportError).toHaveBeenCalledWith({
      message: 'Test error object',
      severity: 'error',
      timestamp: expect.any(Date),
      stack: 'Error stack trace',
      context: undefined,
    });
  });

  it('handles network errors specifically', () => {
    const { result } = renderHook(() => useErrorHandling());

    act(() => {
      result.current.handleNetworkError('Failed to fetch data', 404);
    });

    expect(result.current.errors[0]).toMatchObject({
      message: 'Failed to fetch data',
      severity: 'error',
      type: 'network',
      statusCode: 404,
    });

    expect(mockReportError).toHaveBeenCalledWith({
      message: 'Failed to fetch data',
      severity: 'error',
      timestamp: expect.any(Date),
      type: 'network',
      statusCode: 404,
      context: undefined,
    });
  });

  it('handles validation errors', () => {
    const { result } = renderHook(() => useErrorHandling());
    const validationErrors = [
      { field: 'email', message: 'Invalid email format' },
      { field: 'password', message: 'Password too short' },
    ];

    act(() => {
      result.current.handleValidationErrors(validationErrors);
    });

    expect(result.current.errors).toHaveLength(2);
    expect(result.current.errors[0]).toMatchObject({
      message: 'Invalid email format',
      field: 'email',
      severity: 'warning',
      type: 'validation',
    });
    expect(result.current.errors[1]).toMatchObject({
      message: 'Password too short',
      field: 'password',
      severity: 'warning',
      type: 'validation',
    });
  });

  it('removes error by id', () => {
    const { result } = renderHook(() => useErrorHandling());

    act(() => {
      result.current.addError('Error 1');
      result.current.addError('Error 2');
    });

    const errorId = result.current.errors[0].id;

    act(() => {
      result.current.removeError(errorId);
    });

    expect(result.current.errors).toHaveLength(1);
    expect(result.current.errors[0].message).toBe('Error 2');
  });

  it('clears all errors', () => {
    const { result } = renderHook(() => useErrorHandling());

    act(() => {
      result.current.addError('Error 1');
      result.current.addError('Error 2');
    });

    expect(result.current.errors).toHaveLength(2);

    act(() => {
      result.current.clearErrors();
    });

    expect(result.current.errors).toHaveLength(0);
    expect(result.current.hasErrors).toBe(false);
    expect(result.current.latestError).toBeNull();
    expect(mockClearErrors).toHaveBeenCalled();
  });

  it('retries failed operations', async () => {
    const { result } = renderHook(() => useErrorHandling());
    const mockOperation = vi.fn()
      .mockRejectedValueOnce(new Error('First attempt failed'))
      .mockResolvedValueOnce('Success');

    let operationResult;
    
    await act(async () => {
      operationResult = await result.current.retry(mockOperation, 2);
    });

    expect(mockOperation).toHaveBeenCalledTimes(2);
    expect(operationResult).toBe('Success');
    expect(result.current.errors).toHaveLength(0); // Should not add error if retry succeeds
  });

  it('adds error after all retry attempts fail', async () => {
    const { result } = renderHook(() => useErrorHandling());
    const mockOperation = vi.fn()
      .mockRejectedValue(new Error('Operation failed'));

    let thrownError;
    
    await act(async () => {
      try {
        await result.current.retry(mockOperation, 2);
      } catch (error) {
        thrownError = error;
      }
    });

    expect(mockOperation).toHaveBeenCalledTimes(2);
    expect(thrownError.message).toBe('Operation failed');
    expect(result.current.errors).toHaveLength(1);
    expect(result.current.errors[0].message).toBe('Operation failed after 2 attempts');
  });

  it('debounces similar errors', () => {
    const { result } = renderHook(() => useErrorHandling({ debounceMs: 100 }));

    act(() => {
      result.current.addError('Duplicate error');
      result.current.addError('Duplicate error');
      result.current.addError('Duplicate error');
    });

    // Should only add one error due to debouncing
    expect(result.current.errors).toHaveLength(1);
    expect(mockReportError).toHaveBeenCalledTimes(1);
    expect(mockToast.error).toHaveBeenCalledTimes(1);
  });

  it('groups similar errors', () => {
    const { result } = renderHook(() => useErrorHandling({ groupSimilar: true }));

    act(() => {
      result.current.addError('Network error');
      result.current.addError('Network error');
      result.current.addError('Network error');
    });

    expect(result.current.errors).toHaveLength(1);
    expect(result.current.errors[0].count).toBe(3);
    expect(result.current.errors[0].message).toBe('Network error (3x)');
  });

  it('limits maximum number of errors', () => {
    const { result } = renderHook(() => useErrorHandling({ maxErrors: 2 }));

    act(() => {
      result.current.addError('Error 1');
      result.current.addError('Error 2');
      result.current.addError('Error 3');
    });

    expect(result.current.errors).toHaveLength(2);
    expect(result.current.errors[0].message).toBe('Error 2');
    expect(result.current.errors[1].message).toBe('Error 3');
  });

  it('auto-clears errors after timeout', async () => {
    vi.useFakeTimers();
    
    const { result } = renderHook(() => useErrorHandling({ autoClearMs: 1000 }));

    act(() => {
      result.current.addError('Auto-clear error');
    });

    expect(result.current.errors).toHaveLength(1);

    act(() => {
      vi.advanceTimersByTime(1000);
    });

    expect(result.current.errors).toHaveLength(0);

    vi.useRealTimers();
  });

  it('handles async errors in promise chains', async () => {
    const { result } = renderHook(() => useErrorHandling());
    
    const failingPromise = Promise.reject(new Error('Async error'));

    await act(async () => {
      await result.current.handleAsync(failingPromise);
    });

    expect(result.current.errors).toHaveLength(1);
    expect(result.current.errors[0].message).toBe('Async error');
  });

  it('provides error context for debugging', () => {
    const { result } = renderHook(() => useErrorHandling());

    act(() => {
      result.current.addError('Debug error', 'error', {
        component: 'TestComponent',
        props: { id: 123 },
        state: { loading: false },
      });
    });

    expect(result.current.errors[0].context).toEqual({
      component: 'TestComponent',
      props: { id: 123 },
      state: { loading: false },
    });
  });

  it('categorizes errors by type', () => {
    const { result } = renderHook(() => useErrorHandling());

    act(() => {
      result.current.handleNetworkError('Network error');
      result.current.handleValidationErrors([{ field: 'email', message: 'Invalid email' }]);
      result.current.addError('Generic error');
    });

    const errorsByType = result.current.getErrorsByType();
    
    expect(errorsByType.network).toHaveLength(1);
    expect(errorsByType.validation).toHaveLength(1);
    expect(errorsByType.generic).toHaveLength(1);
  });
});