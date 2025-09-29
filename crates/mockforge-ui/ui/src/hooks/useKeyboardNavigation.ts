import React, { useEffect, useCallback, useRef, useState } from 'react';

export interface KeyboardShortcut {
  key: string;
  ctrl?: boolean;
  shift?: boolean;
  alt?: boolean;
  meta?: boolean;
  handler: (event: KeyboardEvent) => void;
  description?: string;
  preventDefault?: boolean;
  stopPropagation?: boolean;
  element?: HTMLElement | null;
  enabled?: boolean;
}

export interface UseKeyboardNavigationOptions {
  shortcuts?: KeyboardShortcut[];
  element?: HTMLElement | null;
  enabled?: boolean;
  capture?: boolean;
}

export function useKeyboardNavigation({
  shortcuts = [],
  element = null,
  enabled = true,
  capture = false,
}: UseKeyboardNavigationOptions = {}) {
  const [isEnabled, setIsEnabled] = useState(enabled);
  const shortcutsRef = useRef<KeyboardShortcut[]>(shortcuts);

  // Update shortcuts ref when shortcuts change
  useEffect(() => {
    shortcutsRef.current = shortcuts;
  }, [shortcuts]);

  const handleKeyDown = useCallback((event: KeyboardEvent) => {
    if (!isEnabled) return;

    const activeShortcuts = shortcutsRef.current.filter(shortcut => 
      shortcut.enabled !== false
    );

    for (const shortcut of activeShortcuts) {
      const isMatch = 
        event.key.toLowerCase() === shortcut.key.toLowerCase() &&
        !!event.ctrlKey === !!shortcut.ctrl &&
        !!event.shiftKey === !!shortcut.shift &&
        !!event.altKey === !!shortcut.alt &&
        !!event.metaKey === !!shortcut.meta;

      if (isMatch) {
        if (shortcut.preventDefault !== false) {
          event.preventDefault();
        }
        if (shortcut.stopPropagation) {
          event.stopPropagation();
        }
        
        shortcut.handler(event);
        break; // Only handle the first matching shortcut
      }
    }
  }, [isEnabled]);

  useEffect(() => {
    const targetElement = element || document;
    
    if (!isEnabled || !targetElement) return;

    targetElement.addEventListener('keydown', handleKeyDown, capture);

    return () => {
      targetElement.removeEventListener('keydown', handleKeyDown, capture);
    };
  }, [element, isEnabled, handleKeyDown, capture]);

  const addShortcut = useCallback((shortcut: KeyboardShortcut) => {
    shortcutsRef.current = [...shortcutsRef.current, shortcut];
  }, []);

  const removeShortcut = useCallback((key: string, modifiers?: {
    ctrl?: boolean;
    shift?: boolean;
    alt?: boolean;
    meta?: boolean;
  }) => {
    shortcutsRef.current = shortcutsRef.current.filter(shortcut => {
      if (shortcut.key.toLowerCase() !== key.toLowerCase()) return true;
      
      if (modifiers) {
        return !(
          !!shortcut.ctrl === !!modifiers.ctrl &&
          !!shortcut.shift === !!modifiers.shift &&
          !!shortcut.alt === !!modifiers.alt &&
          !!shortcut.meta === !!modifiers.meta
        );
      }
      
      return false;
    });
  }, []);

  const enable = useCallback(() => setIsEnabled(true), []);
  const disable = useCallback(() => setIsEnabled(false), []);
  const toggle = useCallback(() => setIsEnabled(prev => !prev), []);

  return {
    addShortcut,
    removeShortcut,
    enable,
    disable,
    toggle,
    isEnabled,
  };
}

// Hook for managing common navigation shortcuts
export function useCommonShortcuts(options: {
  onEscape?: () => void;
  onEnter?: () => void;
  onSpace?: () => void;
  onArrowUp?: () => void;
  onArrowDown?: () => void;
  onArrowLeft?: () => void;
  onArrowRight?: () => void;
  onHome?: () => void;
  onEnd?: () => void;
  onPageUp?: () => void;
  onPageDown?: () => void;
  enabled?: boolean;
} = {}) {
  const shortcuts: KeyboardShortcut[] = [];

  if (options.onEscape) {
    shortcuts.push({
      key: 'Escape',
      handler: options.onEscape,
      description: 'Close/Cancel',
    });
  }

  if (options.onEnter) {
    shortcuts.push({
      key: 'Enter',
      handler: options.onEnter,
      description: 'Confirm/Select',
    });
  }

  if (options.onSpace) {
    shortcuts.push({
      key: ' ',
      handler: options.onSpace,
      description: 'Activate/Toggle',
    });
  }

  if (options.onArrowUp) {
    shortcuts.push({
      key: 'ArrowUp',
      handler: options.onArrowUp,
      description: 'Move up',
    });
  }

  if (options.onArrowDown) {
    shortcuts.push({
      key: 'ArrowDown',
      handler: options.onArrowDown,
      description: 'Move down',
    });
  }

  if (options.onArrowLeft) {
    shortcuts.push({
      key: 'ArrowLeft',
      handler: options.onArrowLeft,
      description: 'Move left',
    });
  }

  if (options.onArrowRight) {
    shortcuts.push({
      key: 'ArrowRight',
      handler: options.onArrowRight,
      description: 'Move right',
    });
  }

  if (options.onHome) {
    shortcuts.push({
      key: 'Home',
      handler: options.onHome,
      description: 'Go to beginning',
    });
  }

  if (options.onEnd) {
    shortcuts.push({
      key: 'End',
      handler: options.onEnd,
      description: 'Go to end',
    });
  }

  if (options.onPageUp) {
    shortcuts.push({
      key: 'PageUp',
      handler: options.onPageUp,
      description: 'Page up',
    });
  }

  if (options.onPageDown) {
    shortcuts.push({
      key: 'PageDown',
      handler: options.onPageDown,
      description: 'Page down',
    });
  }

  return useKeyboardNavigation({
    shortcuts,
    enabled: options.enabled,
  });
}

// Hook for managing application-level shortcuts
export function useAppShortcuts(options: {
  onSearch?: () => void;
  onHelp?: () => void;
  onSettings?: () => void;
  onToggleSidebar?: () => void;
  onNewItem?: () => void;
  onSave?: () => void;
  onUndo?: () => void;
  onRedo?: () => void;
  onCopy?: () => void;
  onPaste?: () => void;
  onCut?: () => void;
  onSelectAll?: () => void;
  enabled?: boolean;
} = {}) {
  const shortcuts: KeyboardShortcut[] = [];

  if (options.onSearch) {
    shortcuts.push({
      key: 'k',
      ctrl: true,
      handler: options.onSearch,
      description: 'Search',
    });
  }

  if (options.onHelp) {
    shortcuts.push({
      key: '?',
      shift: true,
      handler: options.onHelp,
      description: 'Help',
    });
  }

  if (options.onSettings) {
    shortcuts.push({
      key: ',',
      ctrl: true,
      handler: options.onSettings,
      description: 'Settings',
    });
  }

  if (options.onToggleSidebar) {
    shortcuts.push({
      key: 'b',
      ctrl: true,
      handler: options.onToggleSidebar,
      description: 'Toggle sidebar',
    });
  }

  if (options.onNewItem) {
    shortcuts.push({
      key: 'n',
      ctrl: true,
      handler: options.onNewItem,
      description: 'New item',
    });
  }

  if (options.onSave) {
    shortcuts.push({
      key: 's',
      ctrl: true,
      handler: options.onSave,
      description: 'Save',
    });
  }

  if (options.onUndo) {
    shortcuts.push({
      key: 'z',
      ctrl: true,
      handler: options.onUndo,
      description: 'Undo',
    });
  }

  if (options.onRedo) {
    shortcuts.push({
      key: 'y',
      ctrl: true,
      handler: options.onRedo,
      description: 'Redo',
    });
  }

  if (options.onCopy) {
    shortcuts.push({
      key: 'c',
      ctrl: true,
      handler: options.onCopy,
      description: 'Copy',
    });
  }

  if (options.onPaste) {
    shortcuts.push({
      key: 'v',
      ctrl: true,
      handler: options.onPaste,
      description: 'Paste',
    });
  }

  if (options.onCut) {
    shortcuts.push({
      key: 'x',
      ctrl: true,
      handler: options.onCut,
      description: 'Cut',
    });
  }

  if (options.onSelectAll) {
    shortcuts.push({
      key: 'a',
      ctrl: true,
      handler: options.onSelectAll,
      description: 'Select all',
    });
  }

  return useKeyboardNavigation({
    shortcuts,
    enabled: options.enabled,
  });
}

// Utility hook for displaying keyboard shortcuts help
export function useShortcutsHelp(shortcuts: KeyboardShortcut[]) {
  const formatShortcut = useCallback((shortcut: KeyboardShortcut) => {
    const keys: string[] = [];
    if (shortcut.ctrl) keys.push('Ctrl');
    if (shortcut.shift) keys.push('Shift');
    if (shortcut.alt) keys.push('Alt');
    if (shortcut.meta) keys.push('Cmd');
    keys.push(shortcut.key);
    return keys.join(' + ');
  }, []);

  const ShortcutsHelpComponent = useCallback(() => {
    return React.createElement('div', { className: 'space-y-4' }, [
      React.createElement('h3', { 
        key: 'title',
        className: 'text-heading-md text-primary' 
      }, 'Keyboard Shortcuts'),
      React.createElement('div', { 
        key: 'content',
        className: 'space-y-2' 
      }, shortcuts
        .filter(shortcut => shortcut.description)
        .map((shortcut, index) => 
          React.createElement('div', {
            key: index,
            className: 'flex justify-between items-center'
          }, [
            React.createElement('span', {
              key: 'desc',
              className: 'text-body-md text-secondary'
            }, shortcut.description),
            React.createElement('kbd', {
              key: 'kbd',
              className: 'px-2 py-1 bg-muted border border-border rounded text-mono-sm'
            }, formatShortcut(shortcut))
          ])
        ))
    ]);
  }, [shortcuts, formatShortcut]);

  return {
    formatShortcut,
    ShortcutsHelpComponent,
  };
}