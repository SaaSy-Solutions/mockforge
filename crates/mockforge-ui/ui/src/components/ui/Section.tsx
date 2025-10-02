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
        <div className="flex flex-col md:flex-row md:items-center md:justify-between gap-4 mb-5">
          <div>
            {title && <h2 className="text-h2">{title}</h2>}
            {subtitle && <p className="text-gray-600 dark:text-gray-400 mt-1">{subtitle}</p>}
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