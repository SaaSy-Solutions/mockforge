/**
 * @jest-environment jsdom
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { usePreferencesStore } from '../usePreferencesStore';

describe('usePreferencesStore', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it('initializes with default preferences', () => {
    const { result } = renderHook(() => usePreferencesStore());

    expect(result.current.preferences).toBeDefined();
    expect(result.current.preferences.theme).toBeDefined();
    expect(result.current.preferences.logs).toBeDefined();
    expect(result.current.preferences.notifications).toBeDefined();
  });

  it('updates theme preferences', () => {
    const { result } = renderHook(() => usePreferencesStore());

    act(() => {
      result.current.updateTheme({ theme: 'dark', accentColor: 'purple' });
    });

    expect(result.current.preferences.theme.theme).toBe('dark');
    expect(result.current.preferences.theme.accentColor).toBe('purple');
  });

  it('updates log preferences', () => {
    const { result } = renderHook(() => usePreferencesStore());

    act(() => {
      result.current.updateLogs({ autoScroll: false, itemsPerPage: 50 });
    });

    expect(result.current.preferences.logs.autoScroll).toBe(false);
    expect(result.current.preferences.logs.itemsPerPage).toBe(50);
  });

  it('updates notification preferences', () => {
    const { result } = renderHook(() => usePreferencesStore());

    act(() => {
      result.current.updateNotifications({ enableSounds: true, toastDuration: 10 });
    });

    expect(result.current.preferences.notifications.enableSounds).toBe(true);
    expect(result.current.preferences.notifications.toastDuration).toBe(10);
  });

  it('updates UI preferences', () => {
    const { result } = renderHook(() => usePreferencesStore());

    act(() => {
      result.current.updateUI({ sidebarCollapsed: true, serverTableDensity: 'compact' });
    });

    expect(result.current.preferences.ui.sidebarCollapsed).toBe(true);
    expect(result.current.preferences.ui.serverTableDensity).toBe('compact');
  });

  it('resets to defaults', () => {
    const { result } = renderHook(() => usePreferencesStore());

    act(() => {
      result.current.updateTheme({ theme: 'dark' });
      result.current.resetToDefaults();
    });

    expect(result.current.preferences.theme.theme).not.toBe('dark');
  });
});
