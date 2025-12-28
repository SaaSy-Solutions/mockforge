import React, { createContext, useContext, useCallback } from 'react';
import type { ReactNode } from 'react';
import { ToastContainer } from './Toast';
import type { ToastType } from './Toast';
import { useToastStore } from '../../stores/useToastStore';

interface ToastContextType {
  showToast: (type: ToastType, title: string, message?: string, duration?: number) => string;
  hideToast: (id: string) => void;
  clearAllToasts: () => void;
}

const ToastContext = createContext<ToastContextType | undefined>(undefined);

/**
 * ToastProvider - Provides toast notification functionality via Context API.
 * Uses Zustand store under the hood for consistent state management.
 * Both context hooks (useToast) and imperative API (toast.success()) work together.
 */
export function ToastProvider({ children }: { children: ReactNode }) {
  const { addToast, removeToast, clearAllToasts: clearAll } = useToastStore();

  const showToast = useCallback((
    type: ToastType,
    title: string,
    message?: string,
    duration?: number
  ): string => {
    return addToast({
      type,
      title,
      message,
      duration: duration ?? (type === 'error' ? 8000 : 5000),
    });
  }, [addToast]);

  const hideToast = useCallback((id: string) => {
    removeToast(id);
  }, [removeToast]);

  const clearAllToasts = useCallback(() => {
    clearAll();
  }, [clearAll]);

  return (
    <ToastContext.Provider value={{ showToast, hideToast, clearAllToasts }}>
      {children}
      <ToastContainer />
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
