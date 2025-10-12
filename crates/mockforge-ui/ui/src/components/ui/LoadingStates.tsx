import { logger } from '@/utils/logger';
import React from 'react';
import { cn } from '../../utils/cn';
import { SkeletonCard, SkeletonMetricCard, SkeletonChart, SkeletonTable, SkeletonList } from './Skeleton';
import { Icon, StatusIcon, Icons } from './IconSystem';
import { Button } from './DesignSystem';

// Loading spinner component
interface SpinnerProps {
  size?: 'sm' | 'md' | 'lg' | 'xl';
  className?: string;
  color?: 'primary' | 'brand' | 'muted';
}

export function Spinner({ size = 'md', className, color = 'primary' }: SpinnerProps) {
  const sizes = {
    sm: 'h-4 w-4',
    md: 'h-6 w-6',
    lg: 'h-8 w-8',
    xl: 'h-12 w-12',
  };

  const colors = {
    primary: 'text-primary',
    brand: 'text-brand',
    muted: 'text-secondary',
  };

  return (
    <div
      className={cn(
        'animate-spin rounded-full border-2 border-current border-t-transparent',
        sizes[size],
        colors[color],
        className
      )}
      role="status"
      aria-label="Loading"
    >
      <span className="sr-only">Loading...</span>
    </div>
  );
}

// Standardized loading state component
interface LoadingStateProps {
  title?: string;
  description?: string;
  variant?: 'spinner' | 'skeleton' | 'pulse';
  size?: 'sm' | 'md' | 'lg';
  className?: string;
}

export function LoadingState({
  title = 'Loading...',
  description,
  variant = 'spinner',
  size = 'md',
  className
}: LoadingStateProps) {
  const sizeClasses = {
    sm: 'py-8',
    md: 'py-12',
    lg: 'py-16',
  };

  if (variant === 'skeleton') {
    return (
      <div className={cn('space-y-4', sizeClasses[size], className)}>
        <SkeletonCard />
      </div>
    );
  }

  if (variant === 'pulse') {
    return (
      <div className={cn('text-center', sizeClasses[size], className)}>
        <div className="inline-flex items-center justify-center w-16 h-16 rounded-full bg-brand/10 mb-4 pulse-subtle">
          <Icon icon={Icons.Activity} size="xl" color="brand" />
        </div>
        <h3 className="text-lg font-bold text-gray-900 dark:text-gray-100 mb-2">{title}</h3>
        {description && (
          <p className="text-base text-gray-600 dark:text-gray-400 max-w-md mx-auto">{description}</p>
        )}
      </div>
    );
  }

  return (
    <div className={cn('flex flex-col items-center justify-center text-center', sizeClasses[size], className)}>
      <Spinner size={size === 'sm' ? 'md' : size === 'md' ? 'lg' : 'xl'} className="mb-4" />
      <h3 className="text-lg font-bold text-gray-900 dark:text-gray-100 mb-2">{title}</h3>
      {description && (
        <p className="text-base text-gray-600 dark:text-gray-400 max-w-md">{description}</p>
      )}
    </div>
  );
}

// Standardized empty state component
interface EmptyStateProps {
  icon?: React.ReactNode;
  title: string;
  description?: string;
  action?: {
    label: string;
    onClick: () => void;
    variant?: 'primary' | 'secondary';
  };
  className?: string;
  size?: 'sm' | 'md' | 'lg';
}

export function EmptyState({
  icon,
  title,
  description,
  action,
  className,
  size = 'md'
}: EmptyStateProps) {
  const sizeClasses = {
    sm: 'py-8',
    md: 'py-12',
    lg: 'py-16',
  };

  const iconSizes = {
    sm: '2xl' as const,
    md: '3xl' as const,
    lg: '3xl' as const,
  };

  const defaultIcon = <Icon icon={Icons.Search} size={iconSizes[size]} color="muted" />;

  return (
    <div className={cn(
      'flex flex-col items-center justify-center text-center',
      sizeClasses[size],
      className
    )}>
      <div className="p-4 rounded-full bg-gray-100 dark:bg-gray-800 mb-6 spring-in">
        {icon || defaultIcon}
      </div>
      <h3 className="text-2xl font-bold text-gray-900 dark:text-gray-100 mb-3">
        {title}
      </h3>
      {description && (
        <p className="text-lg text-gray-600 dark:text-gray-400 mb-8 max-w-md">
          {description}
        </p>
      )}
      {action && (
        <Button
          variant={action.variant || 'primary'}
          onClick={action.onClick}
          className="spring-hover"
        >
          {action.label}
        </Button>
      )}
    </div>
  );
}

// Error state component
interface ErrorStateProps {
  title?: string;
  description?: string;
  error?: Error | string;
  retry?: () => void;
  className?: string;
  size?: 'sm' | 'md' | 'lg';
}

export function ErrorState({
  title = 'Something went wrong',
  description,
  error,
  retry,
  className,
  size = 'md'
}: ErrorStateProps) {
  const sizeClasses = {
    sm: 'py-8',
    md: 'py-12',
    lg: 'py-16',
  };

  const errorMessage = error instanceof Error ? error.message : error;

  return (
    <div className={cn(
      'flex flex-col items-center justify-center text-center',
      sizeClasses[size],
      className
    )}>
      <div className="p-4 rounded-full bg-red-50 dark:bg-red-900/20 mb-6 spring-in">
        <StatusIcon status="error" size="3xl" />
      </div>
      <h3 className="text-2xl font-bold text-gray-900 dark:text-gray-100 mb-3">
        {title}
      </h3>
      <div className="space-y-3">
        {description && (
          <p className="text-lg text-gray-600 dark:text-gray-400 max-w-md">
            {description}
          </p>
        )}
        {errorMessage && (
          <div className="text-sm font-mono text-red-700 dark:text-red-500 bg-red-50 dark:bg-red-900/20 rounded-lg p-3 max-w-md">
            {errorMessage}
          </div>
        )}
      </div>
      {retry && (
        <Button
          variant="primary"
          onClick={retry}
          className="mt-6 spring-hover"
        >
          Try Again
        </Button>
      )}
    </div>
  );
}

// Success state component
interface SuccessStateProps {
  title: string;
  description?: string;
  action?: {
    label: string;
    onClick: () => void;
  };
  className?: string;
  size?: 'sm' | 'md' | 'lg';
}

export function SuccessState({
  title,
  description,
  action,
  className,
  size = 'md'
}: SuccessStateProps) {
  const sizeClasses = {
    sm: 'py-8',
    md: 'py-12',
    lg: 'py-16',
  };

  return (
    <div className={cn(
      'flex flex-col items-center justify-center text-center',
      sizeClasses[size],
      className
    )}>
      <div className="p-4 rounded-full bg-green-50 dark:bg-green-900/20 mb-6 spring-bounce">
        <StatusIcon status="success" size="3xl" />
      </div>
      <h3 className="text-2xl font-bold text-gray-900 dark:text-gray-100 mb-3">
        {title}
      </h3>
      {description && (
        <p className="text-lg text-gray-600 dark:text-gray-400 mb-8 max-w-md">
          {description}
        </p>
      )}
      {action && (
        <Button
          variant="success"
          onClick={action.onClick}
          className="spring-hover"
        >
          {action.label}
        </Button>
      )}
    </div>
  );
}

// Specialized loading states for different content types
export function DashboardLoading() {
  return (
    <div className="space-y-8">
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
        <SkeletonMetricCard className="animate-stagger-in animate-delay-75" />
        <SkeletonMetricCard className="animate-stagger-in animate-delay-150" />
        <SkeletonMetricCard className="animate-stagger-in animate-delay-200" />
        <SkeletonMetricCard className="animate-stagger-in animate-delay-300" />
      </div>
      <SkeletonChart />
      <SkeletonTable />
    </div>
  );
}

export function TableLoading({ rows = 5, cols = 4 }: { rows?: number; cols?: number }) {
  return <SkeletonTable rows={rows} cols={cols} />;
}

export function ListLoading({ items = 5 }: { items?: number }) {
  return <SkeletonList items={items} />;
}

export function CardLoading() {
  return <SkeletonCard />;
}

export function MetricLoading() {
  return <SkeletonMetricCard />;
}
