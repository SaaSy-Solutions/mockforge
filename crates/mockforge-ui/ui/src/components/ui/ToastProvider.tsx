import React, { createContext, useContext, useState, useCallback } from 'react';
import type { ReactNode } from 'react';
import { Toast } from './Toast';
import type { ToastProps, ToastType } from './Toast';

interface ToastContextType {
  showToast: (type: ToastType, title: string, message?: string, duration?: number) => string;
  hideToast: (id: string) => void;
  clearAllToasts: () => void;
}

const ToastContext = createContext<ToastContextType | undefined>(undefined);

let toastCounter = 0;

export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<ToastProps[]>([]);

  const showToast = useCallback((
    type: ToastType,
    title: string,
    message?: string,
    duration?: number
  ): string => {
    const id = `toast-${++toastCounter}`;
    const toast: ToastProps = {
      id,
      type,
      title,
      message,
      duration,
      onClose: () => hideToast(id),
    };

    setToasts(prev => [...prev, toast]);
    return id;
  }, []);

  const hideToast = useCallback((id: string) => {
    setToasts(prev => prev.filter(toast => toast.id !== id));
  }, []);

  const clearAllToasts = useCallback(() => {
    setToasts([]);
  }, []);

  return (
    <ToastContext.Provider value={{ showToast, hideToast, clearAllToasts }}>
      {children}

      {/* Toast Container */}
      <div className="fixed top-4 right-4 z-50 space-y-2 max-w-sm">
        {toasts.map(toast => (
          <Toast key={toast.id} {...toast} />
        ))}
      </div>
    </ToastContext.Provider>
  );
}

export function useToast() {
  const context = useContext(ToastContext);
  if (!context) {
    throw new Error('useToast must be used within a ToastProvider');
  }
  return context;
}

// Convenience hooks for different toast types
export function useSuccessToast() {
  const { showToast } = useToast();
  return useCallback((title: string, message?: string, duration?: number) =>
    showToast('success', title, message, duration), [showToast]);
}

export function useErrorToast() {
  const { showToast } = useToast();
  return useCallback((title: string, message?: string, duration?: number) =>
    showToast('error', title, message, duration), [showToast]);
}

export function useWarningToast() {
  const { showToast } = useToast();
  return useCallback((title: string, message?: string, duration?: number) =>
    showToast('warning', title, message, duration), [showToast]);
}

export function useInfoToast() {
  const { showToast } = useToast();
  return useCallback((title: string, message?: string, duration?: number) =>
    showToast('info', title, message, duration), [showToast]);
}
