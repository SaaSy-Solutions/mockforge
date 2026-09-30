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
    default: 'bg-muted text-muted-foreground ring-1 ring-inset ring-border',
    secondary: 'bg-secondary text-secondary-foreground ring-1 ring-inset ring-border',
    success: 'bg-success-50 text-success-700 ring-1 ring-inset ring-success/25 dark:bg-success/15 dark:text-success-400',
    warning: 'bg-warning-50 text-warning-700 ring-1 ring-inset ring-warning/30 dark:bg-warning/15 dark:text-warning-400',
    danger: 'bg-danger-50 text-danger-700 ring-1 ring-inset ring-danger/25 dark:bg-danger/15 dark:text-danger-400',
    destructive: 'bg-danger-50 text-danger-700 ring-1 ring-inset ring-danger/25 dark:bg-danger/15 dark:text-danger-400',
    error: 'bg-danger-50 text-danger-700 ring-1 ring-inset ring-danger/25 dark:bg-danger/15 dark:text-danger-400',
    brand: 'bg-brand-50 text-brand-700 ring-1 ring-inset ring-brand/25 dark:bg-brand/15 dark:text-brand-400',
    info: 'bg-info-50 text-info-700 ring-1 ring-inset ring-info/25 dark:bg-info/15 dark:text-info-400',
    outline: 'border border-border text-foreground',
  };

  return (
    <span
      className={cn(
        'inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-xs font-medium',
        variantClasses[variant],
        className
      )}
      {...props}
    >
      {children}
    </span>
  );
}
