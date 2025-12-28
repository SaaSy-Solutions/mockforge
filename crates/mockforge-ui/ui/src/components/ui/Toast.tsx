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
    success: 'bg-green-50 border-green-200 text-green-800 dark:bg-green-950 dark:border-green-800 dark:text-green-200',
    error: 'bg-red-50 border-red-200 text-red-800 dark:bg-red-950 dark:border-red-800 dark:text-red-200',
    warning: 'bg-yellow-50 border-yellow-200 text-yellow-800 dark:bg-yellow-950 dark:border-yellow-800 dark:text-yellow-200',
    info: 'bg-blue-50 border-blue-200 text-blue-800 dark:bg-blue-950 dark:border-blue-800 dark:text-blue-200',
  };

  const iconColors: Record<ToastType, string> = {
    success: 'text-green-500',
    error: 'text-red-500',
    warning: 'text-yellow-500',
    info: 'text-blue-500',
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
        className="flex-shrink-0 p-1 rounded hover:bg-black/10 dark:hover:bg-white/10 transition-colors"
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
