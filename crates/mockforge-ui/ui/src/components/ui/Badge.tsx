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
    destructive: 'bg-danger-100 text-danger-700 dark:bg-danger-900/20 dark:text-danger-400',
    error: 'bg-danger/15 text-danger',
    brand: 'bg-brand/15 text-brand',
    info: 'bg-info-100 text-info-700 dark:bg-info-900/20 dark:text-info-400',
    outline: 'border border-border text-foreground',
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
