/**
 * @jest-environment jsdom
 */

import { describe, it, expect, vi } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useErrorHandler, useAsyncErrorHandler, useFormErrorHandler } from '../useErrorHandler';

describe('useErrorHandler', () => {
  it('initializes with no error', () => {
    const { result } = renderHook(() => useErrorHandler());

    expect(result.current.error).toBeNull();
    expect(result.current.isError).toBe(false);
  });

  it('sets error from Error object', () => {
    const { result } = renderHook(() => useErrorHandler());
    const testError = new Error('Test error');

    act(() => {
      result.current.setError(testError);
    });

    expect(result.current.error).toBe(testError);
    expect(result.current.isError).toBe(true);
  });

  it('sets error from string', () => {
    const { result } = renderHook(() => useErrorHandler());

    act(() => {
      result.current.setError('String error');
    });

    expect(result.current.error?.message).toBe('String error');
    expect(result.current.isError).toBe(true);
  });

  it('clears error', () => {
    const { result } = renderHook(() => useErrorHandler());

    act(() => {
      result.current.setError('Test error');
      result.current.clearError();
    });

    expect(result.current.error).toBeNull();
    expect(result.current.isError).toBe(false);
  });

  it('resets error', () => {
    const { result } = renderHook(() => useErrorHandler());

    act(() => {
      result.current.setError('Test error');
      result.current.resetError();
    });

    expect(result.current.error).toBeNull();
    expect(result.current.isError).toBe(false);
  });
});

describe('useAsyncErrorHandler', () => {
  it('wraps successful async function', async () => {
    const { result } = renderHook(() => useAsyncErrorHandler());
    const asyncFn = vi.fn().mockResolvedValue('success');

    let returnValue: string | null = null;
    await act(async () => {
      returnValue = await result.current.wrapAsync(asyncFn);
    });

    expect(asyncFn).toHaveBeenCalled();
    expect(returnValue).toBe('success');
    expect(result.current.error).toBeNull();
    expect(result.current.isError).toBe(false);
  });

  it('catches async errors', async () => {
    const { result } = renderHook(() => useAsyncErrorHandler());
    const testError = new Error('Async error');
    const asyncFn = vi.fn().mockRejectedValue(testError);

    let returnValue: unknown = undefined;
    await act(async () => {
      returnValue = await result.current.wrapAsync(asyncFn);
    });

    expect(returnValue).toBeNull();
    expect(result.current.error).toBe(testError);
    expect(result.current.isError).toBe(true);
  });

  it('handles non-Error rejections', async () => {
    const { result } = renderHook(() => useAsyncErrorHandler());
    const asyncFn = vi.fn().mockRejectedValue('String error');

    await act(async () => {
      await result.current.wrapAsync(asyncFn);
    });

    expect(result.current.error?.message).toBe('String error');
    expect(result.current.isError).toBe(true);
  });

  it('clears previous errors on new call', async () => {
    const { result } = renderHook(() => useAsyncErrorHandler());

    // First call fails
    await act(async () => {
      await result.current.wrapAsync(() => Promise.reject(new Error('First error')));
    });

    expect(result.current.isError).toBe(true);

    // Second call succeeds
    await act(async () => {
      await result.current.wrapAsync(() => Promise.resolve('success'));
    });

    expect(result.current.error).toBeNull();
    expect(result.current.isError).toBe(false);
  });
});

describe('useFormErrorHandler', () => {
  it('handles successful form submission', async () => {
    const { result } = renderHook(() => useFormErrorHandler());
    const submitFn = vi.fn().mockResolvedValue({ id: 1, name: 'Test' });
    const onSuccess = vi.fn();

    await act(async () => {
      await result.current.handleSubmit(submitFn, { onSuccess });
    });

    expect(submitFn).toHaveBeenCalled();
    expect(onSuccess).toHaveBeenCalledWith({ id: 1, name: 'Test' });
    expect(result.current.error).toBeNull();
  });

  it('handles form submission errors', async () => {
    const { result } = renderHook(() => useFormErrorHandler());
    const testError = new Error('Submission failed');
    const submitFn = vi.fn().mockRejectedValue(testError);
    const onError = vi.fn();

    await act(async () => {
      await result.current.handleSubmit(submitFn, { onError });
    });

    expect(onError).toHaveBeenCalledWith(testError);
    expect(result.current.error).toBe(testError);
    expect(result.current.isError).toBe(true);
  });

  it('returns null on error', async () => {
    const { result } = renderHook(() => useFormErrorHandler());
    const submitFn = vi.fn().mockRejectedValue(new Error('Failed'));

    let returnValue: unknown = undefined;
    await act(async () => {
      returnValue = await result.current.handleSubmit(submitFn);
    });

    expect(returnValue).toBeNull();
  });

  it('clears errors on new submission', async () => {
    const { result } = renderHook(() => useFormErrorHandler());

    // First submission fails
    await act(async () => {
      await result.current.handleSubmit(() => Promise.reject(new Error('First error')));
    });

    expect(result.current.isError).toBe(true);

    // Second submission succeeds
    await act(async () => {
      await result.current.handleSubmit(() => Promise.resolve('success'));
    });

    expect(result.current.error).toBeNull();
    expect(result.current.isError).toBe(false);
  });
});
