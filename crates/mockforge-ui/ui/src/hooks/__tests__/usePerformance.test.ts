/**
 * @jest-environment jsdom
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';
import {
  useRenderPerformance,
  useApiPerformance,
  useDebounce,
} from '../usePerformance';

describe('useRenderPerformance', () => {
  it('tracks render count', () => {
    const { result, rerender } = renderHook(() => useRenderPerformance('TestComponent'));

    expect(result.current.renderCount).toBeGreaterThan(0);

    rerender();
    expect(result.current.renderCount).toBeGreaterThan(1);
  });

  it('calculates average render time', () => {
    const { result } = renderHook(() => useRenderPerformance('TestComponent'));

    expect(result.current.averageRenderTime).toBeGreaterThanOrEqual(0);
  });
});

describe('useApiPerformance', () => {
  it('tracks API call performance', async () => {
    const { result } = renderHook(() => useApiPerformance('testApi'));

    let startTime: number = 0;
    act(() => {
      startTime = result.current.startCall();
    });

    // Simulate API delay
    await new Promise(resolve => setTimeout(resolve, 10));

    let duration: number = 0;
    act(() => {
      duration = result.current.endCall(startTime);
    });

    expect(duration).toBeGreaterThan(0);
    expect(result.current.stats.totalCalls).toBe(1);
  });

  it('calculates average call time', async () => {
    const { result } = renderHook(() => useApiPerformance('testApi'));

    // Make multiple calls
    for (let i = 0; i < 3; i++) {
      const startTime = result.current.startCall();
      await new Promise(resolve => setTimeout(resolve, 5));
      act(() => {
        result.current.endCall(startTime);
      });
    }

    await waitFor(() => {
      expect(result.current.stats.totalCalls).toBe(3);
      expect(result.current.stats.averageCallTime).toBeGreaterThan(0);
    });
  });
});

describe('useDebounce', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('debounces value', () => {
    const { result, rerender } = renderHook(
      ({ value }) => useDebounce(value, 500),
      { initialProps: { value: 'initial' } }
    );

    expect(result.current).toBe('initial');

    // Change value
    rerender({ value: 'updated' });
    expect(result.current).toBe('initial'); // Still old value

    // Advance timers
    act(() => {
      vi.advanceTimersByTime(500);
    });

    expect(result.current).toBe('updated');
  });

  it('cancels previous timeout', () => {
    const { result, rerender } = renderHook(
      ({ value }) => useDebounce(value, 500),
      { initialProps: { value: 'initial' } }
    );

    rerender({ value: 'update1' });
    act(() => {
      vi.advanceTimersByTime(250);
    });

    rerender({ value: 'update2' });
    act(() => {
      vi.advanceTimersByTime(500);
    });

    expect(result.current).toBe('update2');
  });
});
