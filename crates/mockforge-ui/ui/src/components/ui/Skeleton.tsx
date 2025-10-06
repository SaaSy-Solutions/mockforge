import { logger } from '@/utils/logger';
import React from 'react';
import { cn } from '../../utils/cn';

interface SkeletonProps {
  className?: string;
  width?: string | number;
  height?: string | number;
  circle?: boolean;
  animation?: 'pulse' | 'shimmer' | 'none';
}

export function Skeleton({
  className,
  width,
  height,
  circle = false,
  animation = 'shimmer',
  ...props
}: SkeletonProps & React.HTMLAttributes<HTMLDivElement>) {
  const animationClasses = {
    pulse: 'animate-pulse',
    shimmer: 'loading-shimmer',
    none: '',
  };

  return (
    <div
      className={cn(
        'bg-gray-200 dark:bg-gray-700',
        circle ? 'rounded-full' : 'rounded-md',
        animationClasses[animation],
        className
      )}
      style={{
        width: typeof width === 'number' ? `${width}px` : width,
        height: typeof height === 'number' ? `${height}px` : height,
      }}
      {...props}
    />
  );
}

export function SkeletonCard({ className, ...props }: React.HTMLAttributes<HTMLDivElement>) {
  return (
    <div className={cn('p-6 border border-gray-200 dark:border-gray-800 rounded-xl space-y-4', className)} {...props}>
      <div className="flex items-center space-x-4">
        <Skeleton circle width={48} height={48} />
        <div className="space-y-2 flex-1">
          <Skeleton height={16} width="60%" />
          <Skeleton height={12} width="40%" />
        </div>
      </div>
      <div className="space-y-2">
        <Skeleton height={12} width="100%" />
        <Skeleton height={12} width="80%" />
        <Skeleton height={12} width="90%" />
      </div>
    </div>
  );
}

export function SkeletonTable({ rows = 5, cols = 4, className, ...props }: { rows?: number; cols?: number } & React.HTMLAttributes<HTMLDivElement>) {
  return (
    <div className={cn('space-y-4', className)} {...props}>
      {/* Table Header */}
      <div className="flex space-x-4">
        {Array.from({ length: cols }).map((_, index) => (
          <Skeleton key={`header-${index}`} height={16} className="flex-1" />
        ))}
      </div>
      
      {/* Table Rows */}
      {Array.from({ length: rows }).map((_, rowIndex) => (
        <div key={`row-${rowIndex}`} className="flex space-x-4">
          {Array.from({ length: cols }).map((_, colIndex) => (
            <Skeleton key={`cell-${rowIndex}-${colIndex}`} height={12} className="flex-1" />
          ))}
        </div>
      ))}
    </div>
  );
}

export function SkeletonMetricCard({ className, ...props }: React.HTMLAttributes<HTMLDivElement>) {
  return (
    <div className={cn('p-6 border border-gray-200 dark:border-gray-800 rounded-xl', className)} {...props}>
      <div className="flex items-center justify-between">
        <div className="space-y-2 flex-1">
          <Skeleton height={12} width="60%" />
          <Skeleton height={20} width="40%" />
          <Skeleton height={10} width="50%" />
        </div>
        <Skeleton circle width={40} height={40} />
      </div>
    </div>
  );
}

export function SkeletonList({ items = 5, className, ...props }: { items?: number } & React.HTMLAttributes<HTMLDivElement>) {
  return (
    <div className={cn('space-y-3', className)} {...props}>
      {Array.from({ length: items }).map((_, index) => (
        <div key={`list-item-${index}`} className="flex items-center space-x-3 p-3 border border-gray-200 dark:border-gray-800 rounded-lg">
          <Skeleton circle width={24} height={24} />
          <div className="space-y-1 flex-1">
            <Skeleton height={14} width="70%" />
            <Skeleton height={10} width="50%" />
          </div>
          <Skeleton height={8} width={60} />
        </div>
      ))}
    </div>
  );
}

export function SkeletonChart({ className, ...props }: React.HTMLAttributes<HTMLDivElement>) {
  return (
    <div className={cn('p-6 border border-gray-200 dark:border-gray-800 rounded-xl space-y-4', className)} {...props}>
      <div className="flex items-center justify-between">
        <Skeleton height={16} width="30%" />
        <Skeleton height={12} width="20%" />
      </div>
      <div className="h-64 flex items-end space-x-2">
        {Array.from({ length: 12 }).map((_, index) => (
          <Skeleton
            key={`bar-${index}`}
            className="flex-1"
            height={Math.random() * 200 + 40}
          />
        ))}
      </div>
    </div>
  );
}