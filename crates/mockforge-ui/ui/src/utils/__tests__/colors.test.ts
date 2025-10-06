import { describe, it, expect } from 'vitest';
import {
  getStatusColors,
  getServiceColors,
  getHttpStatusColors,
  getPerformanceColors,
  getPriorityColors,
  getTrendColor,
  semanticColors,
  badgeVariants,
} from '../colors';

describe('getStatusColors', () => {
  it('returns success colors', () => {
    const colors = getStatusColors('success');
    expect(colors).toBe(semanticColors.status.success);
    expect(colors.text).toBe('text-success');
  });

  it('returns warning colors', () => {
    const colors = getStatusColors('warning');
    expect(colors).toBe(semanticColors.status.warning);
    expect(colors.text).toBe('text-warning');
  });

  it('returns danger colors', () => {
    const colors = getStatusColors('danger');
    expect(colors).toBe(semanticColors.status.danger);
    expect(colors.text).toBe('text-danger');
  });

  it('returns info colors', () => {
    const colors = getStatusColors('info');
    expect(colors).toBe(semanticColors.status.info);
    expect(colors.text).toBe('text-info');
  });

  it('returns neutral colors', () => {
    const colors = getStatusColors('neutral');
    expect(colors).toBe(semanticColors.status.neutral);
    expect(colors.text).toBe('text-secondary');
  });
});

describe('getServiceColors', () => {
  it('returns running service colors', () => {
    const colors = getServiceColors('running');
    expect(colors).toBe(semanticColors.service.running);
    expect(colors.dot).toBe('bg-green-500');
  });

  it('returns stopped service colors', () => {
    const colors = getServiceColors('stopped');
    expect(colors).toBe(semanticColors.service.stopped);
    expect(colors.dot).toBe('bg-red-500');
  });

  it('returns starting service colors', () => {
    const colors = getServiceColors('starting');
    expect(colors).toBe(semanticColors.service.starting);
    expect(colors.dot).toBe('bg-yellow-500');
  });

  it('returns error service colors', () => {
    const colors = getServiceColors('error');
    expect(colors).toBe(semanticColors.service.error);
    expect(colors.dot).toBe('bg-red-500');
  });
});

describe('getHttpStatusColors', () => {
  it('returns 2xx colors for 200-299 status codes', () => {
    expect(getHttpStatusColors(200)).toBe(semanticColors.http['2xx']);
    expect(getHttpStatusColors(201)).toBe(semanticColors.http['2xx']);
    expect(getHttpStatusColors(299)).toBe(semanticColors.http['2xx']);
  });

  it('returns 3xx colors for 300-399 status codes', () => {
    expect(getHttpStatusColors(300)).toBe(semanticColors.http['3xx']);
    expect(getHttpStatusColors(301)).toBe(semanticColors.http['3xx']);
    expect(getHttpStatusColors(399)).toBe(semanticColors.http['3xx']);
  });

  it('returns 4xx colors for 400-499 status codes', () => {
    expect(getHttpStatusColors(400)).toBe(semanticColors.http['4xx']);
    expect(getHttpStatusColors(404)).toBe(semanticColors.http['4xx']);
    expect(getHttpStatusColors(499)).toBe(semanticColors.http['4xx']);
  });

  it('returns 5xx colors for 500+ status codes', () => {
    expect(getHttpStatusColors(500)).toBe(semanticColors.http['5xx']);
    expect(getHttpStatusColors(502)).toBe(semanticColors.http['5xx']);
    expect(getHttpStatusColors(599)).toBe(semanticColors.http['5xx']);
  });

  it('returns neutral colors for invalid status codes', () => {
    expect(getHttpStatusColors(100)).toBe(semanticColors.status.neutral);
    expect(getHttpStatusColors(0)).toBe(semanticColors.status.neutral);
  });
});

describe('getPerformanceColors', () => {
  const thresholds = { good: 100, warning: 200 };

  it('returns excellent colors for values below good threshold', () => {
    expect(getPerformanceColors(50, thresholds)).toBe(semanticColors.performance.excellent);
    expect(getPerformanceColors(100, thresholds)).toBe(semanticColors.performance.excellent);
  });

  it('returns good colors for values between good and warning', () => {
    expect(getPerformanceColors(150, thresholds)).toBe(semanticColors.performance.good);
    expect(getPerformanceColors(200, thresholds)).toBe(semanticColors.performance.good);
  });

  it('returns warning colors for values between warning and 2x warning', () => {
    expect(getPerformanceColors(250, thresholds)).toBe(semanticColors.performance.warning);
    expect(getPerformanceColors(400, thresholds)).toBe(semanticColors.performance.warning);
  });

  it('returns critical colors for values above 2x warning threshold', () => {
    expect(getPerformanceColors(401, thresholds)).toBe(semanticColors.performance.critical);
    expect(getPerformanceColors(1000, thresholds)).toBe(semanticColors.performance.critical);
  });
});

describe('getPriorityColors', () => {
  it('returns low priority colors', () => {
    const colors = getPriorityColors('low');
    expect(colors).toBe(semanticColors.priority.low);
  });

  it('returns medium priority colors', () => {
    const colors = getPriorityColors('medium');
    expect(colors).toBe(semanticColors.priority.medium);
  });

  it('returns high priority colors', () => {
    const colors = getPriorityColors('high');
    expect(colors).toBe(semanticColors.priority.high);
  });

  it('returns critical priority colors', () => {
    const colors = getPriorityColors('critical');
    expect(colors).toBe(semanticColors.priority.critical);
  });
});

describe('getTrendColor', () => {
  it('returns neutral color for neutral trend', () => {
    expect(getTrendColor('neutral')).toBe('text-secondary');
    expect(getTrendColor('neutral', false)).toBe('text-secondary');
  });

  it('returns success color for positive upward trend', () => {
    expect(getTrendColor('up', true)).toBe('text-success');
  });

  it('returns danger color for negative upward trend', () => {
    expect(getTrendColor('up', false)).toBe('text-danger');
  });

  it('returns danger color for positive downward trend', () => {
    expect(getTrendColor('down', true)).toBe('text-danger');
  });

  it('returns success color for negative downward trend', () => {
    expect(getTrendColor('down', false)).toBe('text-success');
  });

  it('defaults to positive trend when not specified', () => {
    expect(getTrendColor('up')).toBe('text-success');
    expect(getTrendColor('down')).toBe('text-danger');
  });
});

describe('badgeVariants', () => {
  it('has success variant', () => {
    expect(badgeVariants.success).toContain('bg-green-100');
    expect(badgeVariants.success).toContain('text-green-800');
  });

  it('has warning variant', () => {
    expect(badgeVariants.warning).toContain('bg-yellow-100');
    expect(badgeVariants.warning).toContain('text-yellow-800');
  });

  it('has danger variant', () => {
    expect(badgeVariants.danger).toContain('bg-red-100');
    expect(badgeVariants.danger).toContain('text-red-800');
  });

  it('has info variant', () => {
    expect(badgeVariants.info).toContain('bg-blue-100');
    expect(badgeVariants.info).toContain('text-blue-800');
  });

  it('has brand variant', () => {
    expect(badgeVariants.brand).toContain('bg-orange-100');
    expect(badgeVariants.brand).toContain('text-orange-800');
  });

  it('has neutral variant', () => {
    expect(badgeVariants.neutral).toContain('bg-gray-100');
    expect(badgeVariants.neutral).toContain('text-gray-800');
  });
});
