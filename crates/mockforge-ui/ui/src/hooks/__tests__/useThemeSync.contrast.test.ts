import { describe, expect, it } from 'vitest';
import { PRIMARY_PALETTES, useThemeSync } from '../useThemeSync';
import { act, renderHook } from '@testing-library/react';
import { useThemePaletteStore } from '../../stores/useThemePaletteStore';
import { usePreferencesStore } from '../../stores/usePreferencesStore';

// WCAG relative luminance, measured from the actual HSL color tokens.
function luminance(hsl: string): number {
  const [h, sPercent, lPercent] = hsl.split(' ').map(parseFloat);
  const s = sPercent / 100;
  const l = lPercent / 100;
  const a = s * Math.min(l, 1 - l);
  const channel = (n: number) => {
    const k = (n + h / 30) % 12;
    const rgb = l - a * Math.max(-1, Math.min(k - 3, 9 - k, 1));
    return rgb <= 0.04045 ? rgb / 12.92 : ((rgb + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * channel(0) + 0.7152 * channel(8) + 0.0722 * channel(4);
}

for (const [accent, colors] of Object.entries(PRIMARY_PALETTES)) {
  describe(`${accent} primary button contrast`, () => {
    it.each(['light', 'dark'] as const)('meets 4.5:1 in %s mode', (mode) => {
      const background = luminance(colors[mode]);
      const foreground = luminance(mode === 'light' ? '0 0% 100%' : '220 15% 7%');
      const ratio = (Math.max(background, foreground) + 0.05) / (Math.min(background, foreground) + 0.05);
      expect(ratio).toBeGreaterThanOrEqual(4.5);
    });
  });
}

it('keeps the selected accent and readable foreground after a theme round trip', () => {
  const preferences = usePreferencesStore.getState().preferences;
  const originalMode = useThemePaletteStore.getState().mode;
  useThemePaletteStore.getState().setMode('light');
  usePreferencesStore.setState({ preferences: { ...preferences, theme: { ...preferences.theme, accentColor: 'blue' } } });
  const { unmount } = renderHook(() => useThemeSync());
  const root = document.documentElement;
  expect(root.style.getPropertyValue('--primary')).toBe(PRIMARY_PALETTES.blue.light);
  expect(root.style.getPropertyValue('--primary-foreground')).toBe('0 0% 100%');
  act(() => useThemePaletteStore.getState().setMode('dark'));
  expect(root.style.getPropertyValue('--primary')).toBe(PRIMARY_PALETTES.blue.dark);
  expect(root.style.getPropertyValue('--primary-foreground')).toBe('220 15% 7%');
  act(() => useThemePaletteStore.getState().setMode('light'));
  expect(root.style.getPropertyValue('--primary')).toBe(PRIMARY_PALETTES.blue.light);
  unmount();
  usePreferencesStore.setState({ preferences });
  useThemePaletteStore.getState().setMode(originalMode);
});
