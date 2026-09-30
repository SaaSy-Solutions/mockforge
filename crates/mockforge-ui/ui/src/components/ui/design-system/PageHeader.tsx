import React from 'react';
import { cn } from '../../../utils/cn';

interface PageHeaderProps {
  title: string;
  subtitle?: string;
  action?: React.ReactNode;
  className?: string;
}

/**
 * Page title row: title + one-line description on the left, actions on the
 * right. Compact on purpose — the shell's breadcrumb already says where you
 * are, so this only has to say what the page is for.
 */
export function PageHeader({ title, subtitle, action, className }: PageHeaderProps) {
  return (
    <div className={cn(
      'mb-6 flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between',
      className
    )}>
      <div className="min-w-0">
        <h1 className="text-2xl font-semibold tracking-tight text-foreground truncate">
          {title}
        </h1>
        {subtitle && (
          <p className="mt-1 max-w-3xl text-sm text-muted-foreground">
            {subtitle}
          </p>
        )}
      </div>
      {action && <div className="flex flex-shrink-0 items-center gap-2">{action}</div>}
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
    <section className={cn('py-4', className)}>
      {(title || subtitle || action) && (
        <div className="mb-3 flex items-end justify-between gap-4">
          <div className="min-w-0">
            {title && (
              <h2 className="text-base font-semibold text-foreground">
                {title}
              </h2>
            )}
            {subtitle && (
              <p className="text-sm text-muted-foreground mt-0.5">
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
        <div className="mb-3 flex h-10 w-10 items-center justify-center rounded-lg border border-border bg-bg-primary text-muted-foreground shadow-xs [&_svg]:h-5 [&_svg]:w-5">
          {icon}
        </div>
      )}
      <h3 className="text-sm font-semibold text-foreground mb-1">
        {title}
      </h3>
      {description && (
        <p className="text-sm text-muted-foreground mb-4 max-w-sm">
          {description}
        </p>
      )}
      {action && <div>{action}</div>}
    </div>
  );
}
