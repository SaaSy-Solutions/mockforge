/**
 * @jest-environment jsdom
 */

import { describe, it, expect } from 'vitest';
import {
  performanceQueryKeys,
  usePerformanceStatus,
  usePerformanceSnapshot,
  useStartPerformance,
  useStopPerformance,
  useUpdateRps,
  useAddBottleneck,
  useClearBottlenecks,
} from '../usePerformance';

describe('usePerformance exports', () => {
  it('exports stable query keys', () => {
    expect(performanceQueryKeys.status).toEqual(['performance', 'status']);
    expect(performanceQueryKeys.snapshot).toEqual(['performance', 'snapshot']);
  });

  it('exports performance hooks', () => {
    expect(typeof usePerformanceStatus).toBe('function');
    expect(typeof usePerformanceSnapshot).toBe('function');
    expect(typeof useStartPerformance).toBe('function');
    expect(typeof useStopPerformance).toBe('function');
    expect(typeof useUpdateRps).toBe('function');
    expect(typeof useAddBottleneck).toBe('function');
    expect(typeof useClearBottlenecks).toBe('function');
  });
});
