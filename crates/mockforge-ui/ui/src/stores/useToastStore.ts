import { create } from 'zustand';
import { usePreferencesStore } from './usePreferencesStore';

export type ToastType = 'success' | 'error' | 'warning' | 'info';

export interface Toast {
  id: string;
  type: ToastType;
  title: string;
  message?: string;
  duration?: number;
  dismissible?: boolean;
  action?: {
    label: string;
    onClick: () => void;
  };
}

interface ToastState {
  toasts: Toast[];
}

interface ToastActions {
  addToast: (toast: Omit<Toast, 'id'>) => string;
  removeToast: (id: string) => void;
  clearAllToasts: () => void;
  // Convenience methods
  success: (title: string, message?: string) => string;
  error: (title: string, message?: string) => string;
  warning: (title: string, message?: string) => string;
  info: (title: string, message?: string) => string;
}

interface ToastStore extends ToastState, ToastActions {}

let toastIdCounter = 0;

const generateId = () => {
  toastIdCounter += 1;
  return `toast-${toastIdCounter}-${Date.now()}`;
};

/** Best-effort audible beep for the "enable sounds" preference. Falls back
 * silently if the browser blocks AudioContext (e.g. autoplay policy). */
function playErrorBeep(): void {
  try {
    const Ctx =
      (window as typeof window & { AudioContext?: typeof AudioContext; webkitAudioContext?: typeof AudioContext })
        .AudioContext ||
      (window as typeof window & { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;
    if (!Ctx) return;
    const ctx = new Ctx();
    const osc = ctx.createOscillator();
    const gain = ctx.createGain();
    osc.type = 'sine';
    osc.frequency.value = 520;
    gain.gain.value = 0.08;
    osc.connect(gain).connect(ctx.destination);
    osc.start();
    osc.stop(ctx.currentTime + 0.18);
    osc.onended = () => void ctx.close();
  } catch {
    /* no-op */
  }
}

export const useToastStore = create<ToastStore>()((set, get) => ({
  toasts: [],

  addToast: (toast) => {
    const prefs = usePreferencesStore.getState().preferences.notifications;

    // Respect "Show toasts" toggle.
    if (!prefs.showToasts) {
      return '';
    }
    // Type-specific opt-outs.
    if (toast.type === 'success' && !prefs.notifyOnSuccess) return '';
    if (toast.type === 'error' && !prefs.notifyOnErrors) return '';

    const id = generateId();
    // Default duration comes from user prefs (seconds → ms) unless the caller
    // supplied an explicit override.
    const defaultMs = Math.max(1, prefs.toastDuration) * 1000;
    const newToast: Toast = {
      id,
      duration: defaultMs,
      dismissible: true,
      ...toast,
    };

    set((state) => ({
      toasts: [...state.toasts, newToast],
    }));

    if (toast.type === 'error' && prefs.enableSounds) {
      playErrorBeep();
    }

    if (newToast.duration && newToast.duration > 0) {
      setTimeout(() => {
        get().removeToast(id);
      }, newToast.duration);
    }

    return id;
  },

  removeToast: (id) => {
    set((state) => ({
      toasts: state.toasts.filter((toast) => toast.id !== id),
    }));
  },

  clearAllToasts: () => {
    set({ toasts: [] });
  },

  success: (title, message) => {
    return get().addToast({ type: 'success', title, message });
  },

  error: (title, message) => {
    // Errors keep a longer default floor so users can read the message even
    // when they've dialed down the general toast duration.
    const prefs = usePreferencesStore.getState().preferences.notifications;
    const errorMs = Math.max(prefs.toastDuration * 1000, 8000);
    return get().addToast({ type: 'error', title, message, duration: errorMs });
  },

  warning: (title, message) => {
    return get().addToast({ type: 'warning', title, message });
  },

  info: (title, message) => {
    return get().addToast({ type: 'info', title, message });
  },
}));

// Hook for showing API errors as toasts
export function useApiErrorToast() {
  const { error } = useToastStore();

  return (err: unknown, operation?: string) => {
    const message = err instanceof Error ? err.message : String(err);
    const title = operation ? `Failed to ${operation}` : 'API Error';
    return error(title, message);
  };
}
