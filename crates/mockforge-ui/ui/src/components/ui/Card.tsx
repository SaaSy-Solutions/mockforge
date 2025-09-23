import React from 'react';
import { cn } from '../../utils/cn';

interface CardProps extends React.HTMLAttributes<HTMLDivElement> {
  title?: string;
  icon?: React.ReactNode;
}

export function Card({ title, icon, children, className, ...props }: CardProps) {
  return (
    <div
      className={cn(
        "bg-bg-primary border border-border rounded-xl shadow-sm",
        // subtle brand accent on the left edge to add color without overpowering
        "border-l-4 border-l-brand-200",
        "hover:shadow-lg hover:border-brand-200 transition-all duration-200 ease-out",
        "hover:-translate-y-0.5 group",
        className
      )}
      {...props}
    >
      {title && (
        <div className="border-b border-border/50 px-6 py-4 bg-brand-50 dark:bg-brand-900/10 rounded-t-xl">
          <h3 className="text-lg font-semibold text-text-primary flex items-center gap-3">
            {icon && (
              <span className="p-1.5 rounded-lg bg-brand-50 text-brand-600 group-hover:bg-brand-100 transition-colors duration-200 dark:bg-brand-900/20 dark:text-brand-400">
                {icon}
              </span>
            )}
            {title}
          </h3>
        </div>
      )}
      <div className={cn("p-6", title ? "" : "pt-6")}>
        {children}
      </div>
    </div>
  );
}

export function CardHeader({ className, ...props }: React.HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      className={cn("flex flex-col space-y-1.5 p-6", className)}
      {...props}
    />
  );
}

export function CardTitle({ className, ...props }: React.HTMLAttributes<HTMLHeadingElement>) {
  return (
    <h3
      className={cn("text-2xl font-semibold leading-none tracking-tight", className)}
      {...props}
    />
  );
}

export function CardDescription({ className, ...props }: React.HTMLAttributes<HTMLParagraphElement>) {
  return (
    <p
      className={cn("text-sm text-muted-foreground", className)}
      {...props}
    />
  );
}

export function CardContent({ className, ...props }: React.HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      className={cn("p-6 pt-0", className)}
      {...props}
    />
  );
}
