import React from 'react';
import { cn } from '../../../utils/cn';

interface BadgeProps {
  children: React.ReactNode;
  variant?: 'default' | 'success' | 'warning' | 'error' | 'info' | 'outline' | 'destructive';
  size?: 'sm' | 'md' | 'lg';
  className?: string;
}

export function ModernBadge({
  children,
  variant = 'default',
  size = 'md',
  className
}: BadgeProps) {
  const variants = {
    default: 'bg-muted text-foreground',
    success: 'bg-success-100 text-success-700 dark:bg-success-900/30 dark:text-success-400',
    warning: 'bg-warning-100 text-warning-700 dark:bg-warning-900/30 dark:text-warning-400',
    error: 'bg-danger-100 text-danger-700 dark:bg-danger-900/30 dark:text-danger-400',
    info: 'bg-info-100 text-info-700 dark:bg-info-900/30 dark:text-info-400',
    outline: 'border border-border text-foreground',
    destructive: 'bg-destructive/10 text-destructive dark:bg-destructive/20',
  };

  const sizes = {
    sm: 'px-2 py-0.5 text-xs',
    md: 'px-2.5 py-1 text-xs',
    lg: 'px-3 py-1.5 text-sm',
  };

  return (
    <span className={cn(
      'inline-flex items-center font-medium rounded-full transition-colors duration-200',
      variants[variant],
      sizes[size],
      className
    )}>
      {children}
    </span>
  );
}

export const Badge = ModernBadge;
