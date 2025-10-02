import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import type { ThemePalette } from '../themes';
import { getThemeById, getDefaultTheme } from '../themes';

export type ThemeMode = 'light' | 'dark' | 'system';
export type ResolvedTheme = 'light' | 'dark';

interface ThemePaletteState {
  // Selected theme palette
  selectedThemeId: string;
  currentTheme: ThemePalette;

  // Preferences
  mode: ThemeMode;
  resolvedMode: ResolvedTheme;

  // Aliases for compatibility
  theme: ThemeMode;
  setTheme: (mode: ThemeMode) => void;

  // Actions
  setThemePalette: (themeId: string) => void;
  setMode: (mode: ThemeMode) => void;
  applyTheme: () => void;
  init: () => void;

  // Getters
  getResolvedColors: () => Record<string, string>;
}

export const useThemePaletteStore = create<ThemePaletteState>()(
  persist(
    (set, get) => ({
      selectedThemeId: 'core-brand',
      currentTheme: getDefaultTheme(),
      mode: 'system',
      resolvedMode: 'light',

      // Aliases for compatibility
      get theme() {
        return get().mode;
      },
      setTheme: (mode: ThemeMode) => {
        get().setMode(mode);
      },

      setThemePalette: (themeId: string) => {
        const theme = getThemeById(themeId);
        if (theme) {
          set({ selectedThemeId: themeId, currentTheme: theme });
          get().applyTheme();
        }
      },

      setMode: (mode: ThemeMode) => {
        const resolvedMode = mode === 'system'
          ? (window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light')
          : mode;

        set({ mode, resolvedMode });
        get().applyTheme();
      },

      applyTheme: () => {
        const { currentTheme, resolvedMode } = get();
        const colors = currentTheme.colors[resolvedMode];

        if (!colors) return;

        const root = document.documentElement;

        // Remove existing theme classes
        root.classList.remove('light', 'dark');

        // Apply resolved mode class for Tailwind dark mode
        root.classList.add(resolvedMode);

        // Apply CSS custom properties
        Object.entries(colors).forEach(([property, value]) => {
          root.style.setProperty(property, value);
        });
      },

      init: () => {
        // Load selected theme
        const { selectedThemeId } = get();
        const theme = getThemeById(selectedThemeId) || getDefaultTheme();
        set({ selectedThemeId: theme.id, currentTheme: theme });

        // Set initial mode and apply
        const { mode } = get();
        get().setMode(mode);
      },

      getResolvedColors: () => {
        const { currentTheme, resolvedMode } = get();
        return currentTheme.colors[resolvedMode] || {};
      },
    }),
    {
      name: 'mockforge-theme-palette',
      partialize: (state) => ({
        selectedThemeId: state.selectedThemeId,
        mode: state.mode,
      }),
      onRehydrateStorage: () => (state) => {
        if (state) {
          state.init();
        }
      },
    }
  )
);

// Mobile system theme change listener
if (typeof window !== 'undefined') {
  const { setMode } = useThemePaletteStore.getState();
  window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => {
    const { mode } = useThemePaletteStore.getState();
    if (mode === 'system') {
      setMode('system');
    }
  });
}
