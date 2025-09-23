import React from 'react';
import { cn } from '../../utils/cn';
import { ArrowUp, ArrowDown } from 'lucide-react';

interface StatCardProps {
  title: string;
  value: string | number;
  subtitle?: string;
  trend?: 'up' | 'down';
  trendValue?: string;
  icon?: React.ReactNode;
  className?: string;
}

export function StatCard({
  title,
  value,
  subtitle,
  trend,
  trendValue,
  icon,
  className
}: StatCardProps) {
  return (
    <div className={cn(
      "bg-bg-primary border border-border rounded-xl p-6",
      "shadow-sm hover:shadow-lg transition-all duration-200 ease-out",
      "hover:border-brand-200 hover:-translate-y-0.5",
      "animate-fade-in-up group",
      className
    )}>
      <div className="flex items-start justify-between">
        <div className="flex-1 min-w-0">
          <h4 className="text-sm font-medium text-text-secondary uppercase tracking-wide mb-2">
            {title}
          </h4>

          <div className="flex items-baseline gap-2 mb-3">
            <span className="text-3xl font-bold text-text-primary tabular-nums">
              {value}
            </span>
            {subtitle && (
              <span className="text-sm font-medium text-text-secondary">
                {subtitle}
              </span>
            )}
          </div>

          {trendValue && (
            <div className="flex items-center gap-1">
              {trend === 'up' ? (
                <div className="flex items-center gap-1 px-2 py-1 rounded-full bg-success-50 text-success-700 dark:bg-success-900/20 dark:text-success-400">
                  <ArrowUp className="h-3 w-3" />
                  <span className="text-xs font-semibold">{trendValue}</span>
                </div>
              ) : (
                <div className="flex items-center gap-1 px-2 py-1 rounded-full bg-danger-50 text-danger-700 dark:bg-danger-900/20 dark:text-danger-400">
                  <ArrowDown className="h-3 w-3" />
                  <span className="text-xs font-semibold">{trendValue}</span>
                </div>
              )}
              <span className="text-xs text-text-tertiary ml-1">vs last hour</span>
            </div>
          )}
        </div>

        {icon && (
          <div className="flex-shrink-0 ml-4">
            <div className="p-3 rounded-xl bg-brand-50 text-brand-600 group-hover:bg-brand-100 group-hover:scale-110 transition-all duration-200 dark:bg-brand-900/20 dark:text-brand-400">
              {icon}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
