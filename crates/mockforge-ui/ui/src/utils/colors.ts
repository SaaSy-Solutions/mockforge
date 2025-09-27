// Semantic color mappings for consistent UI theming
export const semanticColors = {
  // Status colors with clear meanings
  status: {
    success: {
      text: 'text-success',
      bg: 'bg-green-50 dark:bg-green-900/20',
      border: 'border-green-200 dark:border-green-800',
      icon: 'text-green-600 dark:text-green-400',
    },
    warning: {
      text: 'text-warning',
      bg: 'bg-yellow-50 dark:bg-yellow-900/20',
      border: 'border-yellow-200 dark:border-yellow-800',
      icon: 'text-yellow-600 dark:text-yellow-400',
    },
    danger: {
      text: 'text-danger',
      bg: 'bg-red-50 dark:bg-red-900/20',
      border: 'border-red-200 dark:border-red-800',
      icon: 'text-red-600 dark:text-red-400',
    },
    info: {
      text: 'text-info',
      bg: 'bg-blue-50 dark:bg-blue-900/20',
      border: 'border-blue-200 dark:border-blue-800',
      icon: 'text-blue-600 dark:text-blue-400',
    },
    neutral: {
      text: 'text-secondary',
      bg: 'bg-gray-50 dark:bg-gray-900/20',
      border: 'border-gray-200 dark:border-gray-800',
      icon: 'text-gray-600 dark:text-gray-400',
    },
  },

  // Service state colors
  service: {
    running: {
      text: 'text-green-700 dark:text-green-300',
      bg: 'bg-green-100 dark:bg-green-900/30',
      dot: 'bg-green-500',
    },
    stopped: {
      text: 'text-red-700 dark:text-red-300',
      bg: 'bg-red-100 dark:bg-red-900/30',
      dot: 'bg-red-500',
    },
    starting: {
      text: 'text-yellow-700 dark:text-yellow-300',
      bg: 'bg-yellow-100 dark:bg-yellow-900/30',
      dot: 'bg-yellow-500',
    },
    error: {
      text: 'text-red-700 dark:text-red-300',
      bg: 'bg-red-100 dark:bg-red-900/30',
      dot: 'bg-red-500',
    },
  },

  // HTTP status code colors
  http: {
    '2xx': {
      text: 'text-green-700 dark:text-green-300',
      bg: 'bg-green-50 dark:bg-green-900/20',
      badge: 'bg-green-100 text-green-800 dark:bg-green-900/20 dark:text-green-400',
    },
    '3xx': {
      text: 'text-blue-700 dark:text-blue-300',
      bg: 'bg-blue-50 dark:bg-blue-900/20',
      badge: 'bg-blue-100 text-blue-800 dark:bg-blue-900/20 dark:text-blue-400',
    },
    '4xx': {
      text: 'text-yellow-700 dark:text-yellow-300',
      bg: 'bg-yellow-50 dark:bg-yellow-900/20',
      badge: 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900/20 dark:text-yellow-400',
    },
    '5xx': {
      text: 'text-red-700 dark:text-red-300',
      bg: 'bg-red-50 dark:bg-red-900/20',
      badge: 'bg-red-100 text-red-800 dark:bg-red-900/20 dark:text-red-400',
    },
  },

  // Performance metric colors
  performance: {
    excellent: {
      text: 'text-green-700 dark:text-green-300',
      bg: 'bg-green-50 dark:bg-green-900/20',
      indicator: 'bg-green-500',
    },
    good: {
      text: 'text-blue-700 dark:text-blue-300',
      bg: 'bg-blue-50 dark:bg-blue-900/20',
      indicator: 'bg-blue-500',
    },
    warning: {
      text: 'text-yellow-700 dark:text-yellow-300',
      bg: 'bg-yellow-50 dark:bg-yellow-900/20',
      indicator: 'bg-yellow-500',
    },
    critical: {
      text: 'text-red-700 dark:text-red-300',
      bg: 'bg-red-50 dark:bg-red-900/20',
      indicator: 'bg-red-500',
    },
  },

  // Priority levels
  priority: {
    low: {
      text: 'text-gray-700 dark:text-gray-300',
      bg: 'bg-gray-50 dark:bg-gray-900/20',
      badge: 'bg-gray-100 text-gray-800 dark:bg-gray-900/20 dark:text-gray-400',
    },
    medium: {
      text: 'text-yellow-700 dark:text-yellow-300',
      bg: 'bg-yellow-50 dark:bg-yellow-900/20',
      badge: 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900/20 dark:text-yellow-400',
    },
    high: {
      text: 'text-orange-700 dark:text-orange-300',
      bg: 'bg-orange-50 dark:bg-orange-900/20',
      badge: 'bg-orange-100 text-orange-800 dark:bg-orange-900/20 dark:text-orange-400',
    },
    critical: {
      text: 'text-red-700 dark:text-red-300',
      bg: 'bg-red-50 dark:bg-red-900/20',
      badge: 'bg-red-100 text-red-800 dark:bg-red-900/20 dark:text-red-400',
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
  success: 'bg-green-100 text-green-800 dark:bg-green-900/20 dark:text-green-400',
  warning: 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900/20 dark:text-yellow-400',
  danger: 'bg-red-100 text-red-800 dark:bg-red-900/20 dark:text-red-400',
  info: 'bg-blue-100 text-blue-800 dark:bg-blue-900/20 dark:text-blue-400',
  brand: 'bg-orange-100 text-orange-800 dark:bg-orange-900/20 dark:text-orange-400',
  neutral: 'bg-gray-100 text-gray-800 dark:bg-gray-900/20 dark:text-gray-400',
} as const;

// Export type definitions for TypeScript
export type StatusType = keyof typeof semanticColors.status;
export type ServiceState = keyof typeof semanticColors.service;
export type PriorityLevel = keyof typeof semanticColors.priority;
export type BadgeVariant = keyof typeof badgeVariants;