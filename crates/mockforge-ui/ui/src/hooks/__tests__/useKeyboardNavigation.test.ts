/**
 * @jest-environment jsdom
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import {
  useKeyboardNavigation,
  useCommonShortcuts,
  useAppShortcuts,
} from '../useKeyboardNavigation';

describe('useKeyboardNavigation', () => {
  beforeEach(() => {
    // Clean up event listeners
    document.removeEventListener('keydown', vi.fn() as any);
  });

  it('initializes with enabled state', () => {
    const { result } = renderHook(() => useKeyboardNavigation());

    expect(result.current.isEnabled).toBe(true);
  });

  it('handles keyboard shortcuts', () => {
    const handler = vi.fn();
    const shortcuts = [{
      key: 's',
      ctrl: true,
      handler,
    }];

    renderHook(() => useKeyboardNavigation({ shortcuts }));

    const event = new KeyboardEvent('keydown', {
      key: 's',
      ctrlKey: true,
    });
    document.dispatchEvent(event);

    expect(handler).toHaveBeenCalled();
  });

  it('prevents default when specified', () => {
    const handler = vi.fn();
    const shortcuts = [{
      key: 's',
      ctrl: true,
      handler,
      preventDefault: true,
    }];

    renderHook(() => useKeyboardNavigation({ shortcuts }));

    const event = new KeyboardEvent('keydown', {
      key: 's',
      ctrlKey: true,
      bubbles: true,
    });
    const preventDefaultSpy = vi.spyOn(event, 'preventDefault');
    document.dispatchEvent(event);

    expect(preventDefaultSpy).toHaveBeenCalled();
  });

  it('stops propagation when specified', () => {
    const handler = vi.fn();
    const shortcuts = [{
      key: 's',
      ctrl: true,
      handler,
      stopPropagation: true,
    }];

    renderHook(() => useKeyboardNavigation({ shortcuts }));

    const event = new KeyboardEvent('keydown', {
      key: 's',
      ctrlKey: true,
      bubbles: true,
    });
    const stopPropagationSpy = vi.spyOn(event, 'stopPropagation');
    document.dispatchEvent(event);

    expect(stopPropagationSpy).toHaveBeenCalled();
  });

  it('ignores disabled shortcuts', () => {
    const handler = vi.fn();
    const shortcuts = [{
      key: 's',
      ctrl: true,
      handler,
      enabled: false,
    }];

    renderHook(() => useKeyboardNavigation({ shortcuts }));

    const event = new KeyboardEvent('keydown', {
      key: 's',
      ctrlKey: true,
    });
    document.dispatchEvent(event);

    expect(handler).not.toHaveBeenCalled();
  });

  it('can be enabled and disabled', () => {
    const handler = vi.fn();
    const shortcuts = [{
      key: 's',
      ctrl: true,
      handler,
    }];

    const { result } = renderHook(() => useKeyboardNavigation({ shortcuts, enabled: false }));

    const event = new KeyboardEvent('keydown', {
      key: 's',
      ctrlKey: true,
    });
    document.dispatchEvent(event);
    expect(handler).not.toHaveBeenCalled();

    act(() => {
      result.current.enable();
    });

    document.dispatchEvent(event);
    expect(handler).toHaveBeenCalled();
  });

  it('can toggle enabled state', () => {
    const { result } = renderHook(() => useKeyboardNavigation({ enabled: true }));

    expect(result.current.isEnabled).toBe(true);

    act(() => {
      result.current.toggle();
    });

    expect(result.current.isEnabled).toBe(false);

    act(() => {
      result.current.toggle();
    });

    expect(result.current.isEnabled).toBe(true);
  });

  it('handles multiple modifiers', () => {
    const handler = vi.fn();
    const shortcuts = [{
      key: 's',
      ctrl: true,
      shift: true,
      handler,
    }];

    renderHook(() => useKeyboardNavigation({ shortcuts }));

    const event = new KeyboardEvent('keydown', {
      key: 's',
      ctrlKey: true,
      shiftKey: true,
    });
    document.dispatchEvent(event);

    expect(handler).toHaveBeenCalled();
  });
});

describe('useCommonShortcuts', () => {
  it('handles Escape key', () => {
    const onEscape = vi.fn();
    renderHook(() => useCommonShortcuts({ onEscape }));

    const event = new KeyboardEvent('keydown', { key: 'Escape' });
    document.dispatchEvent(event);

    expect(onEscape).toHaveBeenCalled();
  });

  it('handles Enter key', () => {
    const onEnter = vi.fn();
    renderHook(() => useCommonShortcuts({ onEnter }));

    const event = new KeyboardEvent('keydown', { key: 'Enter' });
    document.dispatchEvent(event);

    expect(onEnter).toHaveBeenCalled();
  });

  it('handles arrow keys', () => {
    const onArrowUp = vi.fn();
    const onArrowDown = vi.fn();
    const onArrowLeft = vi.fn();
    const onArrowRight = vi.fn();

    renderHook(() =>
      useCommonShortcuts({
        onArrowUp,
        onArrowDown,
        onArrowLeft,
        onArrowRight,
      })
    );

    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowUp' }));
    expect(onArrowUp).toHaveBeenCalled();

    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown' }));
    expect(onArrowDown).toHaveBeenCalled();

    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowLeft' }));
    expect(onArrowLeft).toHaveBeenCalled();

    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight' }));
    expect(onArrowRight).toHaveBeenCalled();
  });
});

describe('useAppShortcuts', () => {
  it('handles Ctrl+K for search', () => {
    const onSearch = vi.fn();
    renderHook(() => useAppShortcuts({ onSearch }));

    const event = new KeyboardEvent('keydown', {
      key: 'k',
      ctrlKey: true,
    });
    document.dispatchEvent(event);

    expect(onSearch).toHaveBeenCalled();
  });

  it('handles Ctrl+S for save', () => {
    const onSave = vi.fn();
    renderHook(() => useAppShortcuts({ onSave }));

    const event = new KeyboardEvent('keydown', {
      key: 's',
      ctrlKey: true,
    });
    document.dispatchEvent(event);

    expect(onSave).toHaveBeenCalled();
  });

  it('handles Ctrl+Z for undo', () => {
    const onUndo = vi.fn();
    renderHook(() => useAppShortcuts({ onUndo }));

    const event = new KeyboardEvent('keydown', {
      key: 'z',
      ctrlKey: true,
    });
    document.dispatchEvent(event);

    expect(onUndo).toHaveBeenCalled();
  });
});
