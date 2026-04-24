import { useEffect } from 'react';
import { usePreferencesStore } from '../stores/usePreferencesStore';

/** HSL tuples (no hsl() wrapper — Tailwind uses the triplet form). */
const ACCENT_PALETTES: Record<string, { brand: string; brand50: string; brand100: string; brand500: string; brand600: string }> = {
  blue: {
    brand: '217 91% 60%',
    brand50: '217 100% 97%',
    brand100: '217 95% 92%',
    brand500: '217 91% 60%',
    brand600: '217 93% 55%',
  },
  green: {
    brand: '142 76% 36%',
    brand50: '142 100% 97%',
    brand100: '142 90% 92%',
    brand500: '142 76% 36%',
    brand600: '142 78% 32%',
  },
  purple: {
    brand: '271 76% 53%',
    brand50: '271 100% 97%',
    brand100: '271 90% 92%',
    brand500: '271 76% 53%',
    brand600: '271 78% 48%',
  },
  orange: {
    brand: '24 86% 42%',
    brand50: '24 100% 97%',
    brand100: '24 95% 92%',
    brand500: '24 86% 42%',
    brand600: '24 88% 36%',
  },
  red: {
    brand: '0 84% 50%',
    brand50: '0 100% 97%',
    brand100: '0 95% 92%',
    brand500: '0 84% 50%',
    brand600: '0 86% 45%',
  },
};

const FONT_SIZES: Record<string, string> = {
  small: '14px',
  medium: '16px',
  large: '18px',
};

/**
 * Keeps the <html> element in sync with theme preferences. Runs on every
 * mount plus whenever the relevant slice of the preferences store changes.
 *
 * - `accentColor` overrides the `--primary`, `--brand*`, and `--ring` CSS
 *   variables on document.documentElement (both light and dark modes read
 *   these same variables).
 * - `fontSize` sets the root font-size, which rescales every `rem`-based
 *   value in the app.
 * - `highContrast` toggles a `high-contrast` class on <html>; CSS rules in
 *   index.css apply the override styles.
 */
export function useThemeSync() {
  const accentColor = usePreferencesStore((s) => s.preferences.theme.accentColor);
  const fontSize = usePreferencesStore((s) => s.preferences.theme.fontSize);
  const highContrast = usePreferencesStore((s) => s.preferences.theme.highContrast);

  useEffect(() => {
    const palette = ACCENT_PALETTES[accentColor] ?? ACCENT_PALETTES.orange;
    const root = document.documentElement;
    root.style.setProperty('--primary', palette.brand);
    root.style.setProperty('--brand', palette.brand);
    root.style.setProperty('--brand-50', palette.brand50);
    root.style.setProperty('--brand-100', palette.brand100);
    root.style.setProperty('--brand-500', palette.brand500);
    root.style.setProperty('--brand-600', palette.brand600);
    root.style.setProperty('--ring', palette.brand);
  }, [accentColor]);

  useEffect(() => {
    const size = FONT_SIZES[fontSize] ?? FONT_SIZES.medium;
    document.documentElement.style.fontSize = size;
  }, [fontSize]);

  useEffect(() => {
    document.documentElement.classList.toggle('high-contrast', highContrast);
  }, [highContrast]);
}
