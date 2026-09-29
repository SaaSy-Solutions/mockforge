import React from 'react';
import { cn } from '../../utils/cn';

interface CardProps extends Omit<React.HTMLAttributes<HTMLDivElement>, 'title'> {
  title?: string | React.ReactNode;
  icon?: React.ReactNode;
}

export function Card({ title, icon, children, className, ...props }: CardProps) {
  return (
    <div
      className={cn(
        "bg-bg-primary border border-border rounded-xl shadow-xs",
        className
      )}
      {...props}
    >
      {title && (
        <div className="border-b border-border px-5 py-3.5">
          <h3 className="text-sm font-semibold text-foreground flex items-center gap-2.5">
            {icon && typeof title === 'string' && (
              <span className="flex h-7 w-7 items-center justify-center rounded-md bg-muted text-muted-foreground [&_svg]:h-4 [&_svg]:w-4">
                {icon}
              </span>
            )}
            {title}
          </h3>
        </div>
      )}
      {usesComposedParts(children) ? (
        children
      ) : (
        <div className="p-5">
          {children}
        </div>
      )}
    </div>
  );
}

/**
 * shadcn-style usage (<Card><CardHeader/><CardContent/></Card>) lets the parts
 * own their padding; wrapping them again would double it.
 */
function usesComposedParts(children: React.ReactNode): boolean {
  return React.Children.toArray(children).some(
    (child) =>
      React.isValidElement(child) &&
      (child.type === CardHeader || child.type === CardContent),
  );
}

export function CardHeader({ className, ...props }: React.HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      className={cn("flex flex-col space-y-1 p-5", className)}
      {...props}
    />
  );
}

export function CardTitle({ className, ...props }: React.HTMLAttributes<HTMLHeadingElement>) {
  return (
    <h3
      className={cn("text-base font-semibold leading-tight tracking-tight", className)}
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
      className={cn("p-5 pt-0", className)}
      {...props}
    />
  );
}
