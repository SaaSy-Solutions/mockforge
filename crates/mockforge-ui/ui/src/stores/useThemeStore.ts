import { logger } from '@/utils/logger';
import { create } from 'zustand';
import { persist } from 'zustand/middleware';

type Theme = 'light' | 'dark' | 'system';

interface ThemeStore {
  theme: Theme;
  resolvedTheme: 'light' | 'dark';
  setTheme: (theme: Theme) => void;
  toggleTheme: () => void;
  init: () => void;
}

export const useThemeStore = create<ThemeStore>()(
  persist(
    (set, get) => ({
      theme: 'system',
      resolvedTheme: 'light',

      setTheme: (theme: Theme) => {
        const resolvedTheme = theme === 'system'
          ? (window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light')
          : theme;

        set({ theme, resolvedTheme });

        // Apply theme to document
        const root = document.documentElement;
        root.classList.remove('light', 'dark');
        root.classList.add(resolvedTheme);
      },

      toggleTheme: () => {
        const currentResolved = get().resolvedTheme;
        const newTheme = currentResolved === 'light' ? 'dark' : 'light';
        get().setTheme(newTheme);
      },

      // Initialize theme on load
      init: () => {
        const { theme } = get();
        get().setTheme(theme);
      },
    }),
    {
      name: 'mockforge-theme',
      onRehydrateStorage: () => (state) => {
        if (state) {
          state.init();
        }
      },
    }
  )
);

// Initialize theme on module load
if (typeof window !== 'undefined') {
  const store = useThemeStore.getState();
  store.init();

  // Listen for system theme changes
  window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => {
    const { theme } = useThemeStore.getState();
    if (theme === 'system') {
      useThemeStore.getState().setTheme('system');
    }
  });
}
