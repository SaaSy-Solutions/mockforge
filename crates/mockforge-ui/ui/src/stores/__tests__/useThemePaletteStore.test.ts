import { describe, expect, it } from 'vitest';
import { useThemePaletteStore } from '../useThemePaletteStore';
describe('theme persistence and compatibility', () => {
  it('keeps the compatibility mode and document in sync across repeated toggles and reload', async () => {
    const store = useThemePaletteStore.getState();
    store.setMode('light');
    store.setMode('dark');
    expect(useThemePaletteStore.getState().theme).toBe('dark');
    expect(document.documentElement.classList.contains('dark')).toBe(true);
    store.setMode('light');
    expect(useThemePaletteStore.getState().theme).toBe('light');
    await useThemePaletteStore.persist.rehydrate();
    expect(useThemePaletteStore.getState().resolvedMode).toBe('light');
    expect(document.documentElement.classList.contains('light')).toBe(true);
  });
});
