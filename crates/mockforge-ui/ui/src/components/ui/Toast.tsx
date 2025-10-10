import { logger } from '@/utils/logger';
import React, { useEffect, useState } from 'react';
import { cn } from '../../utils/cn';
import { CheckCircle, XCircle, AlertCircle, Info, X } from 'lucide-react';

export type ToastType = 'success' | 'error' | 'warning' | 'info';

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
    success: 'bg-green-50 border-green-200 text-green-800',
    error: 'bg-red-50 border-red-200 text-red-800',
    warning: 'bg-yellow-50 border-yellow-200 text-yellow-800',
    info: 'bg-blue-50 border-blue-200 text-blue-800',
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
        className="flex-shrink-0 p-1 rounded hover:bg-black/10 transition-colors"
        aria-label="Close notification"
      >
        <X className="h-4 w-4" />
      </button>
    </div>
  );
}

// Toast function for convenience
let toastIdCounter = 0;
// const toastCallbacks: { [key: string]: () => void } = {};

function showToast(type: ToastType, title: string, message?: string, duration?: number) {
  const id = `toast-${++toastIdCounter}`;

  // Create toast element using DOM APIs to prevent XSS
  const toastElement = document.createElement('div');
  toastElement.id = id;

  // Create container
  const container = document.createElement('div');
  container.className = 'fixed top-4 right-4 z-50 space-y-2 max-w-sm';

  // Create toast card
  const toastCard = document.createElement('div');
  const colorClasses = {
    success: 'bg-green-50 border-green-200 text-green-800',
    error: 'bg-red-50 border-red-200 text-red-800',
    warning: 'bg-yellow-50 border-yellow-200 text-yellow-800',
    info: 'bg-blue-50 border-blue-200 text-blue-800',
  };
  toastCard.className = `flex items-start gap-3 p-4 border rounded-lg shadow-lg transition-all duration-300 opacity-100 transform translate-x-0 ${colorClasses[type]}`;
  toastCard.setAttribute('role', 'alert');

  // Create icon container
  const iconContainer = document.createElement('div');
  const iconColorClasses = {
    success: 'text-green-500',
    error: 'text-red-500',
    warning: 'text-yellow-500',
    info: 'text-blue-500',
  };
  iconContainer.className = `h-5 w-5 mt-0.5 flex-shrink-0 ${iconColorClasses[type]}`;

  // SVG icons (safe to use innerHTML for static SVG)
  const svgIcons = {
    success: '<svg fill="currentColor" viewBox="0 0 20 20"><path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clip-rule="evenodd"></path></svg>',
    error: '<svg fill="currentColor" viewBox="0 0 20 20"><path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z" clip-rule="evenodd"></path></svg>',
    warning: '<svg fill="currentColor" viewBox="0 0 20 20"><path fill-rule="evenodd" d="M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z" clip-rule="evenodd"></path></svg>',
    info: '<svg fill="currentColor" viewBox="0 0 20 20"><path fill-rule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7-4a1 1 0 11-2 0 1 1 0 012 0zM9 9a1 1 0 000 2v3a1 1 0 001 1h1a1 1 0 100-2v-3a1 1 0 00-1-1H9z" clip-rule="evenodd"></path></svg>',
  };
  iconContainer.innerHTML = svgIcons[type];

  // Create text content container
  const textContainer = document.createElement('div');
  textContainer.className = 'flex-1 min-w-0';

  // Create title element (using textContent to prevent XSS)
  const titleElement = document.createElement('h4');
  titleElement.className = 'text-sm font-medium';
  titleElement.textContent = title; // Safe: textContent auto-escapes

  textContainer.appendChild(titleElement);

  // Create message element if provided (using textContent to prevent XSS)
  if (message) {
    const messageElement = document.createElement('p');
    messageElement.className = 'text-sm opacity-90 mt-1';
    messageElement.textContent = message; // Safe: textContent auto-escapes
    textContainer.appendChild(messageElement);
  }

  // Create close button
  const closeButton = document.createElement('button');
  closeButton.className = 'flex-shrink-0 p-1 rounded hover:bg-black/10 transition-colors';
  closeButton.setAttribute('aria-label', 'Close notification');
  closeButton.innerHTML = '<svg class="h-4 w-4" fill="currentColor" viewBox="0 0 20 20"><path fill-rule="evenodd" d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z" clip-rule="evenodd"></path></svg>';

  // Assemble the toast
  toastCard.appendChild(iconContainer);
  toastCard.appendChild(textContainer);
  toastCard.appendChild(closeButton);
  container.appendChild(toastCard);
  toastElement.appendChild(container);

  // Add close callback (using addEventListener instead of inline onclick)
  const closeToast = () => {
    const element = document.getElementById(id);
    if (element) {
      element.remove();
    }
  };

  closeButton.addEventListener('click', closeToast);

  document.body.appendChild(toastElement);

  // Auto remove after duration
  if (duration && duration > 0) {
    setTimeout(() => {
      closeToast();
    }, duration);
  }

  return id;
}

export const toast = {
  success: (title: string, message?: string, duration?: number) => showToast('success', title, message, duration),
  error: (title: string, message?: string, duration?: number) => showToast('error', title, message, duration),
  warning: (title: string, message?: string, duration?: number) => showToast('warning', title, message, duration),
  info: (title: string, message?: string, duration?: number) => showToast('info', title, message, duration),
};
