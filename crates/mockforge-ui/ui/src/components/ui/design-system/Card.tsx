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
    default: 'bg-card text-card-foreground border border-border shadow-xs',
    elevated: 'bg-card text-card-foreground border border-border shadow-md',
    outlined: 'bg-card text-card-foreground border border-border',
  };

  const paddings = {
    none: '',
    sm: 'p-3',
    md: 'p-5',
    lg: 'p-6',
  };

  return (
    <div
      className={cn(
        'rounded-xl',
        variants[variant],
        className
      )}
      {...props}
    >
      {(title || subtitle || icon || action) && (
        <div className="flex items-center justify-between gap-3 border-b border-border px-5 py-3.5">
          <div className="flex items-center gap-2.5 min-w-0">
            {icon && (
              <div className="flex h-7 w-7 items-center justify-center rounded-md bg-muted text-muted-foreground flex-shrink-0 [&_svg]:h-4 [&_svg]:w-4">
                {icon}
              </div>
            )}
            <div className="min-w-0">
              {title && <h3 className="text-sm font-semibold text-foreground truncate">{title}</h3>}
              {subtitle && <p className="text-xs text-muted-foreground mt-0.5">{subtitle}</p>}
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
    up: 'text-success-600 dark:text-success-400',
    down: 'text-danger-600 dark:text-danger-400',
    neutral: 'text-muted-foreground',
  };

  return (
    <ModernCard className={className} padding="sm">
      <div className="flex items-start justify-between gap-3 px-1 py-0.5">
        <div className="min-w-0 flex-1">
          <p className="text-[13px] font-medium text-muted-foreground truncate">
            {title}
          </p>
          <div className="flex items-baseline gap-2 mt-1.5">
            <p className="font-mono text-2xl font-semibold tracking-tight tabular-nums text-foreground">
              {typeof value === 'number' ? value.toLocaleString() : value}
            </p>
            {trend && (
              <span className={cn(
                'text-xs font-medium',
                trendColors[trend.direction]
              )}>
                {trend.value}
              </span>
            )}
          </div>
          {subtitle && (
            <p className="text-xs text-muted-foreground mt-1 truncate">
              {subtitle}
            </p>
          )}
        </div>
        {icon && (
          <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-md bg-muted text-muted-foreground [&_svg]:h-4 [&_svg]:w-4">
            {icon}
          </div>
        )}
      </div>
    </ModernCard>
  );
}
