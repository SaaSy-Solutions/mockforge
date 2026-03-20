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
    success: 'bg-green-50 border-green-200 text-green-800 dark:bg-green-900/20 dark:border-green-800 dark:text-green-400',
    warning: 'bg-yellow-50 border-yellow-200 text-yellow-800 dark:bg-yellow-900/20 dark:border-yellow-800 dark:text-yellow-400',
    error: 'bg-red-50 border-red-200 text-red-800 dark:bg-red-900/20 dark:border-red-800 dark:text-red-400',
    info: 'bg-blue-50 border-blue-200 text-blue-800 dark:bg-blue-900/20 dark:border-blue-800 dark:text-blue-400',
    destructive: 'bg-red-50 border-red-200 text-red-800 dark:bg-red-900/20 dark:border-red-800 dark:text-red-400',
    default: 'bg-gray-50 border-gray-200 text-gray-800 dark:bg-gray-900/20 dark:border-gray-800 dark:text-gray-400',
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
