import React from 'react';
import { cn } from '../../../utils/cn';
import { CheckCircle, AlertCircle, Info, AlertTriangle } from 'lucide-react';

interface AlertProps {
  type?: 'success' | 'warning' | 'error' | 'info' | 'destructive' | 'default';
  variant?: 'success' | 'warning' | 'error' | 'info' | 'destructive' | 'default';
  title?: string;
  message?: string;
  className?: string;
  children?: React.ReactNode;
}

export function Alert({ type, variant, title, message, className, children }: AlertProps) {
  const alertType = type || variant || 'info';

  const icons = {
    success: CheckCircle,
    warning: AlertTriangle,
    error: AlertCircle,
    info: Info,
    destructive: AlertCircle,
    default: Info,
  };

  const colors = {
    success: 'bg-success-50 border-success-200 text-success-700 dark:bg-success-900/20 dark:border-success-700 dark:text-success-400',
    warning: 'bg-warning-50 border-warning-200 text-warning-700 dark:bg-warning-900/20 dark:border-warning-700 dark:text-warning-400',
    error: 'bg-danger-50 border-danger-200 text-danger-700 dark:bg-danger-900/20 dark:border-danger-700 dark:text-danger-400',
    info: 'bg-info-50 border-info-200 text-info-700 dark:bg-info-900/20 dark:border-info-700 dark:text-info-400',
    destructive: 'bg-destructive/10 border-destructive/30 text-destructive dark:bg-destructive/20',
    default: 'bg-muted border-border text-foreground',
  };

  const Icon = icons[alertType];

  return (
    <div className={cn(
      'flex items-start gap-3 p-4 border rounded-xl transition-all duration-200 spring-in',
      colors[alertType],
      className
    )}>
      <Icon className="h-5 w-5 mt-0.5 flex-shrink-0 spring-hover" />
      <div className="flex-1 min-w-0">
        {title && <h4 className="font-semibold text-sm">{title}</h4>}
        {message && (
          <p className="text-sm opacity-90 mt-1">{message}</p>
        )}
        {children}
      </div>
    </div>
  );
}
