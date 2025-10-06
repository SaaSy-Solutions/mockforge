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

// Default preferences
const defaultThemePreferences: UIThemePreferences = {
  theme: 'system',
  accentColor: 'blue',
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

interface PreferencesStore extends PreferencesState, PreferencesActions {}

// Simulate API delay
const delay = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));

export const usePreferencesStore = create<PreferencesStore>()(
  persist(
    (set, get) => ({
      preferences: defaultPreferences,
      loading: false,
      error: null,

      updatePreferences: (newPreferences) => {
        const current = get().preferences;
        set({
          preferences: { ...current, ...newPreferences },
          error: null,
        });
      },

      updateTheme: (themeUpdates) => {
        const current = get().preferences;
        set({
          preferences: {
            ...current,
            theme: { ...current.theme, ...themeUpdates },
          },
          error: null,
        });
      },

      updateLogs: (logsUpdates) => {
        const current = get().preferences;
        set({
          preferences: {
            ...current,
            logs: { ...current.logs, ...logsUpdates },
          },
          error: null,
        });
      },

      updateNotifications: (notificationUpdates) => {
        const current = get().preferences;
        set({
          preferences: {
            ...current,
            notifications: { ...current.notifications, ...notificationUpdates },
          },
          error: null,
        });
      },

      updateSearch: (searchUpdates) => {
        const current = get().preferences;
        set({
          preferences: {
            ...current,
            search: { ...current.search, ...searchUpdates },
          },
          error: null,
        });
      },

      updateUI: (uiUpdates) => {
        const current = get().preferences;
        set({
          preferences: {
            ...current,
            ui: { ...current.ui, ...uiUpdates },
          },
          error: null,
        });
      },

      loadPreferences: async () => {
        set({ loading: true, error: null });

        try {
          // Simulate API call to load preferences
          await delay(800);
          // In a real app, this would make an API call to load preferences
          set({ loading: false });
        } catch (error) {
          set({
            loading: false,
            error: error instanceof Error ? error.message : 'Failed to load preferences',
          });
          throw error;
        }
      },

      resetToDefaults: () => {
        set({
          preferences: defaultPreferences,
          error: null,
        });
      },

      savePreferences: async () => {
        set({ loading: true, error: null });

        try {
          // Simulate API call to save preferences
          await delay(800);
          // In a real app, this would make an API call
          set({ loading: false });
        } catch (error) {
          set({
            loading: false,
            error: error instanceof Error ? error.message : 'Failed to save preferences',
          });
          throw error;
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
