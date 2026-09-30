import React from 'react';
import { cn } from '../../utils/cn';

interface SectionProps extends React.HTMLAttributes<HTMLDivElement> {
  title?: string;
  subtitle?: string;
  actions?: React.ReactNode;
}

export function Section({
  title,
  subtitle,
  actions,
  children,
  className,
  ...props
}: SectionProps) {
  return (
    <div className={cn("section-gap", className)} {...props}>
      {(title || subtitle || actions) && (
        <div className="flex flex-col md:flex-row md:items-end md:justify-between gap-3 mb-3">
          <div>
            {title && <h2 className="text-base font-semibold text-foreground">{title}</h2>}
            {subtitle && <p className="text-sm text-muted-foreground mt-0.5">{subtitle}</p>}
          </div>
          {actions && <div>{actions}</div>}
        </div>
      )}
      <div className="content-gap">
        {children}
      </div>
    </div>
  );
}
