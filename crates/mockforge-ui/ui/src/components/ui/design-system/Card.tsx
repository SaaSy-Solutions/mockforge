import React from 'react';
import { cn } from '../../../utils/cn';

interface CardProps extends React.HTMLAttributes<HTMLDivElement> {
  title?: string;
  subtitle?: string;
  icon?: React.ReactNode;
  action?: React.ReactNode;
  variant?: 'default' | 'elevated' | 'outlined';
  padding?: 'none' | 'sm' | 'md' | 'lg';
}

export function ModernCard({
  title,
  subtitle,
  icon,
  action,
  variant = 'default',
  padding = 'md',
  children,
  className,
  ...props
}: CardProps) {
  const variants = {
    default: 'bg-white dark:bg-gray-900 border border-gray-200 dark:border-gray-800 shadow-sm',
    elevated: 'bg-white dark:bg-gray-900 border border-gray-200 dark:border-gray-800 shadow-lg',
    outlined: 'bg-white dark:bg-gray-900 border-2 border-gray-300 dark:border-gray-700',
  };

  const paddings = {
    none: '',
    sm: 'p-4',
    md: 'p-6',
    lg: 'p-8',
  };

  return (
    <div
      className={cn(
        'rounded-xl transition-all duration-200 hover:shadow-md animate-fade-in-scale',
        'card-hover',
        variants[variant],
        className
      )}
      {...props}
    >
      {(title || subtitle || icon || action) && (
        <div className="flex items-center justify-between p-6 pb-0 mb-6">
          <div className="flex items-center gap-3 min-w-0">
            {icon && (
              <div className="p-2 rounded-lg bg-blue-50 dark:bg-blue-900/20 text-blue-600 dark:text-blue-400 flex-shrink-0">
                {icon}
              </div>
            )}
            <div className="min-w-0">
              {title && <h3 className="font-semibold text-gray-900 dark:text-gray-100 truncate">{title}</h3>}
              {subtitle && <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">{subtitle}</p>}
            </div>
          </div>
          {action && <div className="flex-shrink-0">{action}</div>}
        </div>
      )}
      <div className={cn(paddings[padding], title ? '' : paddings[padding])}>
        {children}
      </div>
    </div>
  );
}

// Alias for backward compat
export const Card = ModernCard;

interface MetricCardProps {
  title: string;
  value: string | number;
  subtitle?: string;
  icon?: React.ReactNode;
  trend?: {
    direction: 'up' | 'down' | 'neutral';
    value: string;
  };
  className?: string;
}

export function MetricCard({
  title,
  value,
  subtitle,
  icon,
  trend,
  className
}: MetricCardProps) {
  const trendColors = {
    up: 'text-green-600 dark:text-green-400',
    down: 'text-red-600 dark:text-red-400',
    neutral: 'text-gray-600 dark:text-gray-400',
  };

  return (
    <ModernCard className={className}>
      <div className="flex items-center justify-between">
        <div className="min-w-0 flex-1">
          <p className="text-sm font-medium text-gray-600 dark:text-gray-400 truncate">
            {title}
          </p>
          <div className="flex items-baseline gap-2 mt-1">
            <p className="text-3xl font-bold text-gray-900 dark:text-gray-100">
              {typeof value === 'number' ? value.toLocaleString() : value}
            </p>
            {trend && (
              <span className={cn(
                'text-sm font-medium',
                trendColors[trend.direction]
              )}>
                {trend.value}
              </span>
            )}
          </div>
          {subtitle && (
            <p className="text-sm text-gray-600 dark:text-gray-400 mt-1">
              {subtitle}
            </p>
          )}
        </div>
        {icon && (
          <div className="p-3 rounded-lg bg-blue-50 dark:bg-blue-900/20 text-blue-600 dark:text-blue-400 spring-hover">
            {icon}
          </div>
        )}
      </div>
    </ModernCard>
  );
}
