import { logger } from '@/utils/logger';
import React from 'react';
import { cn } from '../../utils/cn';

interface BadgeProps extends React.HTMLAttributes<HTMLSpanElement> {
  variant?: 'default' | 'secondary' | 'success' | 'warning' | 'danger' | 'brand' | 'destructive' | 'info' | 'error' | 'outline';
}

export function Badge({
  children,
  variant = 'default',
  className,
  ...props
}: BadgeProps) {
  const variantClasses = {
    default: 'bg-muted text-muted-foreground',
    secondary: 'bg-secondary text-secondary-foreground',
    success: 'bg-success/15 text-success',
    warning: 'bg-warning/15 text-warning',
    danger: 'bg-danger/15 text-danger',
    destructive: 'bg-red-100 text-red-700 dark:bg-red-900/20 dark:text-red-400',
    error: 'bg-danger/15 text-danger',
    brand: 'bg-brand/15 text-brand',
    info: 'bg-blue-100 text-blue-700 dark:bg-blue-900/20 dark:text-blue-400',
    outline: 'border border-gray-300 dark:border-gray-700 text-gray-700 dark:text-gray-300',
  };

  return (
    <span
      className={cn(
        'inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-medium transition-colors',
        variantClasses[variant],
        className
      )}
      {...props}
    >
      {children}
    </span>
  );
}