import React from 'react';
import { cn } from '../../../utils/cn';
import { X } from 'lucide-react';

interface ToastProps {
  message: string;
  type?: 'success' | 'error' | 'warning' | 'info';
  onClose?: () => void;
}

export function Toast({ message, type = 'info', onClose }: ToastProps) {
  const colors = {
    success: 'bg-success-50 border-success-200 text-success-700 dark:bg-success-900/20 dark:border-success-700 dark:text-success-400',
    error: 'bg-danger-50 border-danger-200 text-danger-700 dark:bg-danger-900/20 dark:border-danger-700 dark:text-danger-400',
    warning: 'bg-warning-50 border-warning-200 text-warning-700 dark:bg-warning-900/20 dark:border-warning-700 dark:text-warning-400',
    info: 'bg-info-50 border-info-200 text-info-700 dark:bg-info-900/20 dark:border-info-700 dark:text-info-400',
  };

  return (
    <div className={cn(
      'flex items-center justify-between p-4 border rounded-lg shadow-sm',
      colors[type]
    )}>
      <span className="text-sm">{message}</span>
      {onClose && (
        <button
          onClick={onClose}
          className="ml-4 text-muted-foreground hover:text-foreground"
        >
          <X className="h-4 w-4" />
        </button>
      )}
    </div>
  );
}

export function ToastProvider({ children }: { children: React.ReactNode }) {
  return <div>{children}</div>;
}
