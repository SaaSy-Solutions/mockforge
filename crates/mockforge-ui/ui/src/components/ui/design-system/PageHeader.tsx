import React from 'react';
import { cn } from '../../../utils/cn';

interface PageHeaderProps {
  title: string;
  subtitle?: string;
  action?: React.ReactNode;
  className?: string;
}

export function PageHeader({ title, subtitle, action, className }: PageHeaderProps) {
  return (
    <div className={cn(
      'flex items-center justify-between py-6 border-b border-border',
      className
    )}>
      <div className="min-w-0">
        <h1 className="text-3xl font-bold text-foreground truncate">
          {title}
        </h1>
        {subtitle && (
          <p className="text-lg text-muted-foreground mt-2">
            {subtitle}
          </p>
        )}
      </div>
      {action && <div className="flex-shrink-0 ml-4">{action}</div>}
    </div>
  );
}

interface SectionProps {
  title?: string;
  subtitle?: string;
  action?: React.ReactNode;
  className?: string;
  children: React.ReactNode;
}

export function Section({ title, subtitle, action, className, children }: SectionProps) {
  return (
    <section className={cn('py-8', className)}>
      {(title || subtitle || action) && (
        <div className="flex items-center justify-between mb-6">
          <div className="min-w-0">
            {title && (
              <h2 className="text-2xl font-bold text-foreground">
                {title}
              </h2>
            )}
            {subtitle && (
              <p className="text-base text-muted-foreground mt-1">
                {subtitle}
              </p>
            )}
          </div>
          {action && <div className="flex-shrink-0">{action}</div>}
        </div>
      )}
      {children}
    </section>
  );
}

interface EmptyStateProps {
  icon?: React.ReactNode;
  title: string;
  description?: string;
  action?: React.ReactNode;
  className?: string;
}

export function EmptyState({
  icon,
  title,
  description,
  action,
  className
}: EmptyStateProps) {
  return (
    <div className={cn(
      'flex flex-col items-center justify-center py-12 px-4 text-center',
      className
    )}>
      {icon && (
        <div className="p-4 rounded-full bg-muted text-muted-foreground mb-4">
          {icon}
        </div>
      )}
      <h3 className="text-lg font-semibold text-foreground mb-2">
        {title}
      </h3>
      {description && (
        <p className="text-sm text-muted-foreground mb-6 max-w-md">
          {description}
        </p>
      )}
      {action && <div>{action}</div>}
    </div>
  );
}
