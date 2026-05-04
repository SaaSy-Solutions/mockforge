import { logger } from '@/utils/logger';
import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import type {
  UserPreferences,
  PreferencesState,
  PreferencesActions,
  UIThemePreferences,
  LogPreferences,
  NotificationPreferences,
  SearchPreferences,
  UIBehaviorPreferences,
} from '../types';
import { authApi } from '../services/authApi';

// Default preferences
const defaultThemePreferences: UIThemePreferences = {
  theme: 'system',
  // Default to the MockForge brand orange. `useThemeSync` writes
  // --primary/--brand* from this on mount; if it doesn't match the brand,
  // every CTA renders in the wrong colour even though the CSS tokens are
  // correct.
  accentColor: 'orange',
  fontSize: 'medium',
  highContrast: false,
};

const defaultLogPreferences: LogPreferences = {
  autoScroll: true,
  pauseOnError: false,
  defaultTimeRange: 24,
  itemsPerPage: 100,
  showTimestamps: true,
  compactView: false,
};

const defaultNotificationPreferences: NotificationPreferences = {
  enableSounds: false,
  showToasts: true,
  toastDuration: 5,
  notifyOnErrors: true,
  notifyOnSuccess: false,
};

const defaultSearchPreferences: SearchPreferences = {
  defaultScope: 'all',
  searchHistory: [],
  maxHistoryItems: 10,
  caseSensitive: false,
  regexEnabled: false,
};

const defaultUIBehaviorPreferences: UIBehaviorPreferences = {
  sidebarCollapsed: false,
  defaultPage: 'dashboard',
  confirmDelete: true,
  autoSave: true,
  keyboardShortcuts: true,
  serverTableDensity: 'comfortable',
};

const defaultPreferences: UserPreferences = {
  theme: defaultThemePreferences,
  logs: defaultLogPreferences,
  notifications: defaultNotificationPreferences,
  search: defaultSearchPreferences,
  ui: defaultUIBehaviorPreferences,
};

// Debounced server save — module-level so re-renders don't reset the timer.
let saveTimer: ReturnType<typeof setTimeout> | null = null;
const SAVE_DEBOUNCE_MS = 800;

interface PreferencesStore extends PreferencesState, PreferencesActions {}

function mergePartial(
  current: UserPreferences,
  partial: Partial<UserPreferences>,
): UserPreferences {
  return {
    ...current,
    ...partial,
    theme: { ...current.theme, ...(partial.theme ?? {}) },
    logs: { ...current.logs, ...(partial.logs ?? {}) },
    notifications: { ...current.notifications, ...(partial.notifications ?? {}) },
    search: { ...current.search, ...(partial.search ?? {}) },
    ui: { ...current.ui, ...(partial.ui ?? {}) },
  };
}

/** Schedule a best-effort server save after the user stops twiddling controls. */
function scheduleSync(get: () => PreferencesStore) {
  if (saveTimer) clearTimeout(saveTimer);
  saveTimer = setTimeout(() => {
    saveTimer = null;
    void get()
      .savePreferences()
      .catch((err) => logger.warn('Preference auto-save failed', err));
  }, SAVE_DEBOUNCE_MS);
}

export const usePreferencesStore = create<PreferencesStore>()(
  persist(
    (set, get) => ({
      preferences: defaultPreferences,
      loading: false,
      error: null,

      updatePreferences: (newPreferences) => {
        set({
          preferences: mergePartial(get().preferences, newPreferences),
          error: null,
        });
        scheduleSync(get);
      },

      updateTheme: (themeUpdates) => {
        set({
          preferences: mergePartial(get().preferences, { theme: themeUpdates }),
          error: null,
        });
        scheduleSync(get);
      },

      updateLogs: (logsUpdates) => {
        set({
          preferences: mergePartial(get().preferences, { logs: logsUpdates }),
          error: null,
        });
        scheduleSync(get);
      },

      updateNotifications: (notificationUpdates) => {
        set({
          preferences: mergePartial(get().preferences, {
            notifications: notificationUpdates,
          }),
          error: null,
        });
        scheduleSync(get);
      },

      updateSearch: (searchUpdates) => {
        set({
          preferences: mergePartial(get().preferences, { search: searchUpdates }),
          error: null,
        });
        scheduleSync(get);
      },

      updateUI: (uiUpdates) => {
        set({
          preferences: mergePartial(get().preferences, { ui: uiUpdates }),
          error: null,
        });
        scheduleSync(get);
      },

      resetToDefaults: () => {
        set({
          preferences: defaultPreferences,
          error: null,
        });
        scheduleSync(get);
      },

      loadPreferences: async () => {
        if (!authApi.isCloud()) return; // local mode keeps localStorage only
        set({ loading: true, error: null });
        try {
          const remote = (await authApi.getPreferences()) as Partial<UserPreferences>;
          const hasRemote = remote && Object.keys(remote).length > 0;
          if (hasRemote) {
            set({
              preferences: mergePartial(defaultPreferences, remote),
              loading: false,
            });
          } else {
            set({ loading: false });
          }
        } catch (err) {
          set({
            loading: false,
            error: err instanceof Error ? err.message : 'Failed to load preferences',
          });
          logger.warn('Loading preferences from server failed', err);
        }
      },

      savePreferences: async () => {
        if (!authApi.isCloud()) return;
        if (saveTimer) {
          clearTimeout(saveTimer);
          saveTimer = null;
        }
        set({ loading: true, error: null });
        try {
          await authApi.updatePreferences(
            get().preferences as unknown as Record<string, unknown>,
          );
          set({ loading: false });
        } catch (err) {
          set({
            loading: false,
            error: err instanceof Error ? err.message : 'Failed to save preferences',
          });
          throw err;
        }
      },
    }),
    {
      name: 'mockforge-preferences',
      partialize: (state) => ({
        preferences: state.preferences,
      }),
    }
  )
);
