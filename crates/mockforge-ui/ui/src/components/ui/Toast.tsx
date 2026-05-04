import React, { useEffect, useState } from 'react';
import { cn } from '../../utils/cn';
import { CheckCircle, XCircle, AlertCircle, Info, X } from 'lucide-react';
import { useToastStore, type Toast as ToastData, type ToastType } from '../../stores/useToastStore';

export type { ToastType };

export interface ToastProps {
  type: ToastType;
  title: string;
  message?: string;
  duration?: number;
  onClose?: () => void;
}

export function Toast({
  type,
  title,
  message,
  duration = 5000,
  onClose,
}: ToastProps) {
  const [isVisible, setIsVisible] = useState(true);
  const [isExiting, setIsExiting] = useState(false);

  useEffect(() => {
    if (duration > 0) {
      const timer = setTimeout(() => {
        handleClose();
      }, duration);

      return () => clearTimeout(timer);
    }
  }, [duration]);

  const handleClose = () => {
    setIsExiting(true);
    setTimeout(() => {
      setIsVisible(false);
      onClose?.();
    }, 300); // Match animation duration
  };

  if (!isVisible) return null;

  const icons: Record<ToastType, React.ComponentType<{ className?: string }>> = {
    success: CheckCircle,
    error: XCircle,
    warning: AlertCircle,
    info: Info,
  };

  const colors: Record<ToastType, string> = {
    success: 'bg-success-50 border-success-200 text-success-700 dark:bg-success-900/30 dark:border-success-800 dark:text-success-300',
    error: 'bg-danger-50 border-danger-200 text-danger-700 dark:bg-danger-900/30 dark:border-danger-800 dark:text-danger-300',
    warning: 'bg-warning-50 border-warning-200 text-warning-700 dark:bg-warning-900/30 dark:border-warning-800 dark:text-warning-300',
    info: 'bg-info-50 border-info-200 text-info-700 dark:bg-info-900/30 dark:border-info-800 dark:text-info-300',
  };

  const iconColors: Record<ToastType, string> = {
    success: 'text-success-500',
    error: 'text-danger-500',
    warning: 'text-warning-500',
    info: 'text-info-500',
  };

  const Icon = icons[type];

  return (
    <div
      className={cn(
        'flex items-start gap-3 p-4 border rounded-lg shadow-lg transition-all duration-300',
        colors[type],
        isExiting ? 'opacity-0 transform translate-x-full' : 'opacity-100 transform translate-x-0'
      )}
      role="alert"
      aria-live={type === 'error' ? 'assertive' : 'polite'}
    >
      <Icon className={cn('h-5 w-5 mt-0.5 flex-shrink-0', iconColors[type])} />
      <div className="flex-1 min-w-0">
        <h4 className="text-sm font-medium">{title}</h4>
        {message && (
          <p className="text-sm opacity-90 mt-1">{message}</p>
        )}
      </div>
      <button
        onClick={handleClose}
        className="flex-shrink-0 p-1 rounded hover:bg-black/10 dark:hover:bg-card/10 transition-colors"
        aria-label="Close notification"
      >
        <X className="h-4 w-4" />
      </button>
    </div>
  );
}

/**
 * ToastContainer - Renders all active toasts from the store
 * Place this component once at the root of your app (e.g., in App.tsx)
 */
export function ToastContainer() {
  const { toasts, removeToast } = useToastStore();

  if (toasts.length === 0) return null;

  return (
    <div
      className="fixed top-4 right-4 z-50 space-y-2 max-w-sm pointer-events-none"
      aria-label="Notifications"
    >
      {toasts.map((toast) => (
        <div key={toast.id} className="pointer-events-auto">
          <Toast
            type={toast.type}
            title={toast.title}
            message={toast.message}
            duration={0} // Store handles auto-removal
            onClose={() => removeToast(toast.id)}
          />
        </div>
      ))}
    </div>
  );
}

/**
 * Toast utility for imperative toast notifications.
 * Uses the Zustand store under the hood for consistent state management.
 *
 * Note: Ensure ToastContainer is rendered in your app root.
 *
 * @example
 * ```tsx
 * import { toast } from '@/components/ui/Toast';
 *
 * // Show success notification
 * toast.success('Changes saved', 'Your settings have been updated.');
 *
 * // Show error notification
 * toast.error('Failed to save', error.message);
 * ```
 */
export const toast = {
  success: (title: string, message?: string) => {
    return useToastStore.getState().success(title, message);
  },
  error: (title: string, message?: string) => {
    return useToastStore.getState().error(title, message);
  },
  warning: (title: string, message?: string) => {
    return useToastStore.getState().warning(title, message);
  },
  info: (title: string, message?: string) => {
    return useToastStore.getState().info(title, message);
  },
};
