import { logger } from '@/utils/logger';
// Semantic color mappings for consistent UI theming
export const semanticColors = {
  // Status colors with clear meanings
  status: {
    success: {
      text: 'text-success',
      bg: 'bg-success-50 dark:bg-success-900/20',
      border: 'border-success-200 dark:border-success-800',
      icon: 'text-success-600 dark:text-success-400',
    },
    warning: {
      text: 'text-warning',
      bg: 'bg-warning-50 dark:bg-warning-900/20',
      border: 'border-warning-200 dark:border-warning-800',
      icon: 'text-warning-600 dark:text-warning-400',
    },
    danger: {
      text: 'text-danger',
      bg: 'bg-danger-50 dark:bg-danger-900/20',
      border: 'border-danger-200 dark:border-danger-800',
      icon: 'text-danger-600 dark:text-danger-400',
    },
    info: {
      text: 'text-info',
      bg: 'bg-info-50 dark:bg-info-900/20',
      border: 'border-info-200 dark:border-info-800',
      icon: 'text-info-600 dark:text-info-400',
    },
    neutral: {
      text: 'text-secondary',
      bg: 'bg-muted/20',
      border: 'border-border',
      icon: 'text-muted-foreground',
    },
  },

  // Service state colors
  service: {
    running: {
      text: 'text-success-700 dark:text-success-300',
      bg: 'bg-success-100 dark:bg-success-900/30',
      dot: 'bg-success-500',
    },
    stopped: {
      text: 'text-danger-700 dark:text-danger-300',
      bg: 'bg-danger-100 dark:bg-danger-900/30',
      dot: 'bg-danger-500',
    },
    starting: {
      text: 'text-warning-700 dark:text-warning-300',
      bg: 'bg-warning-100 dark:bg-warning-900/30',
      dot: 'bg-warning-500',
    },
    error: {
      text: 'text-danger-700 dark:text-danger-300',
      bg: 'bg-danger-100 dark:bg-danger-900/30',
      dot: 'bg-danger-500',
    },
  },

  // HTTP status code colors
  http: {
    '2xx': {
      text: 'text-success-700 dark:text-success-300',
      bg: 'bg-success-50 dark:bg-success-900/20',
      badge: 'bg-success-100 text-success-700 dark:bg-success-900/20 dark:text-success-400',
    },
    '3xx': {
      text: 'text-info-700 dark:text-info-300',
      bg: 'bg-info-50 dark:bg-info-900/20',
      badge: 'bg-info-100 text-info-700 dark:bg-info-900/20 dark:text-info-400',
    },
    '4xx': {
      text: 'text-warning-700 dark:text-warning-300',
      bg: 'bg-warning-50 dark:bg-warning-900/20',
      badge: 'bg-warning-100 text-warning-700 dark:bg-warning-900/20 dark:text-warning-400',
    },
    '5xx': {
      text: 'text-danger-700 dark:text-danger-300',
      bg: 'bg-danger-50 dark:bg-danger-900/20',
      badge: 'bg-danger-100 text-danger-700 dark:bg-danger-900/20 dark:text-danger-400',
    },
  },

  // Performance metric colors
  performance: {
    excellent: {
      text: 'text-success-700 dark:text-success-300',
      bg: 'bg-success-50 dark:bg-success-900/20',
      indicator: 'bg-success-500',
    },
    good: {
      text: 'text-info-700 dark:text-info-300',
      bg: 'bg-info-50 dark:bg-info-900/20',
      indicator: 'bg-info-500',
    },
    warning: {
      text: 'text-warning-700 dark:text-warning-300',
      bg: 'bg-warning-50 dark:bg-warning-900/20',
      indicator: 'bg-warning-500',
    },
    critical: {
      text: 'text-danger-700 dark:text-danger-300',
      bg: 'bg-danger-50 dark:bg-danger-900/20',
      indicator: 'bg-danger-500',
    },
  },

  // Priority levels
  priority: {
    low: {
      text: 'text-foreground',
      bg: 'bg-muted/20',
      badge: 'bg-gray-100 text-gray-800 dark:bg-gray-900/20 dark:text-gray-400',
    },
    medium: {
      text: 'text-warning-700 dark:text-warning-300',
      bg: 'bg-warning-50 dark:bg-warning-900/20',
      badge: 'bg-warning-100 text-warning-700 dark:bg-warning-900/20 dark:text-warning-400',
    },
    high: {
      text: 'text-orange-700 dark:text-orange-300',
      bg: 'bg-orange-50 dark:bg-orange-900/20',
      badge: 'bg-orange-100 text-orange-800 dark:bg-orange-900/20 dark:text-orange-400',
    },
    critical: {
      text: 'text-danger-700 dark:text-danger-300',
      bg: 'bg-danger-50 dark:bg-danger-900/20',
      badge: 'bg-danger-100 text-danger-700 dark:bg-danger-900/20 dark:text-danger-400',
    },
  },
} as const;

// Helper functions for getting semantic colors
export function getStatusColors(status: keyof typeof semanticColors.status) {
  return semanticColors.status[status];
}

export function getServiceColors(state: keyof typeof semanticColors.service) {
  return semanticColors.service[state];
}

export function getHttpStatusColors(statusCode: number) {
  if (statusCode >= 200 && statusCode < 300) return semanticColors.http['2xx'];
  if (statusCode >= 300 && statusCode < 400) return semanticColors.http['3xx'];
  if (statusCode >= 400 && statusCode < 500) return semanticColors.http['4xx'];
  if (statusCode >= 500) return semanticColors.http['5xx'];
  return semanticColors.status.neutral;
}

export function getPerformanceColors(value: number, thresholds: { good: number; warning: number }) {
  if (value <= thresholds.good) return semanticColors.performance.excellent;
  if (value <= thresholds.warning) return semanticColors.performance.good;
  if (value <= thresholds.warning * 2) return semanticColors.performance.warning;
  return semanticColors.performance.critical;
}

export function getPriorityColors(priority: keyof typeof semanticColors.priority) {
  return semanticColors.priority[priority];
}

// Color utility for trend indicators
export function getTrendColor(direction: 'up' | 'down' | 'neutral', isPositive: boolean = true) {
  if (direction === 'neutral') return 'text-secondary';

  const isGood = (direction === 'up' && isPositive) || (direction === 'down' && !isPositive);
  return isGood ? 'text-success' : 'text-danger';
}

// Badge color variants
export const badgeVariants = {
  success: 'bg-success-100 text-success-700 dark:bg-success-900/20 dark:text-success-400',
  warning: 'bg-warning-100 text-warning-700 dark:bg-warning-900/20 dark:text-warning-400',
  danger: 'bg-danger-100 text-danger-700 dark:bg-danger-900/20 dark:text-danger-400',
  info: 'bg-info-100 text-info-700 dark:bg-info-900/20 dark:text-info-400',
  brand: 'bg-orange-100 text-orange-800 dark:bg-orange-900/20 dark:text-orange-400',
  neutral: 'bg-gray-100 text-gray-800 dark:bg-gray-900/20 dark:text-gray-400',
} as const;

// Export type definitions for TypeScript
export type StatusType = keyof typeof semanticColors.status;
export type ServiceState = keyof typeof semanticColors.service;
export type PriorityLevel = keyof typeof semanticColors.priority;
export type BadgeVariant = keyof typeof badgeVariants;
