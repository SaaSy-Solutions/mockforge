import React from 'react';
import { cn } from '../../utils/cn';
import { ArrowUp, ArrowDown } from 'lucide-react';
import { Card } from '../ui/Card';

interface MetricCardProps {
  title: string;
  value: string | number;
  subtitle?: string;
  trend?: 'up' | 'down' | 'neutral';
  trendValue?: string;
  icon?: React.ReactNode;
  className?: string;
}

export function MetricCard({
  title,
  value,
  subtitle,
  trend,
  trendValue,
  icon,
  className,
}: MetricCardProps) {
  return (
    <Card className={cn('group', className)}>
      <div className="flex items-start justify-between">
        <div className="flex-1 min-w-0">
          <p className="text-xs font-medium text-gray-600 dark:text-gray-400 uppercase tracking-wider mb-2">{title}</p>
          <div className="flex items-baseline gap-2 mb-3">
            <p className="text-3xl font-semibold text-gray-900 dark:text-gray-100 tabular-nums">{value}</p>
            {subtitle && <p className="text-sm font-medium text-gray-600 dark:text-gray-400">{subtitle}</p>}
          </div>
          {trendValue && trend && (
            <div className="flex items-center gap-2">
              {trend === 'up' ? (
                <div className="flex items-center gap-1 px-2 py-1 rounded-full bg-success-50 text-success-700 dark:bg-success-900/20 dark:text-success-400">
                  <ArrowUp className="h-3 w-3" />
                  <span className="text-xs font-semibold">{trendValue}</span>
                </div>
              ) : trend === 'down' ? (
                <div className="flex items-center gap-1 px-2 py-1 rounded-full bg-danger-50 text-danger-700 dark:bg-danger-900/20 dark:text-red-700 dark:text-red-500-400">
                  <ArrowDown className="h-3 w-3" />
                  <span className="text-xs font-semibold">{trendValue}</span>
                </div>
              ) : (
                <div className="flex items-center gap-1 px-2 py-1 rounded-full bg-neutral-100 text-neutral-600 dark:bg-neutral-800 dark:text-neutral-400">
                  <span className="text-xs font-semibold">{trendValue}</span>
                </div>
              )}
              <span className="text-xs text-gray-600 dark:text-gray-400">vs last hour</span>
            </div>
          )}
        </div>
        {icon && (
          <div className="flex-shrink-0 ml-4">
            <div className="p-3 rounded-lg bg-brand-50 text-brand-600 group-hover:bg-brand-100 group-hover:scale-105 transition-all duration-200 dark:bg-brand-900/20 dark:text-brand-400">
              {icon}
            </div>
          </div>
        )}
      </div>
    </Card>
  );
}
