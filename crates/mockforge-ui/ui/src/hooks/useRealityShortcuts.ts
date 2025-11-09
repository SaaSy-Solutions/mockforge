/**
 * Reality Slider Keyboard Shortcuts Hook
 *
 * Provides keyboard shortcuts for quick reality level changes:
 * - Ctrl+Shift+1-5: Set reality level 1-5
 * - Ctrl+Shift+R: Reset to default level (3)
 * - Ctrl+Shift+P: Open preset manager
 */

import { useCallback, useRef } from 'react';
import { useKeyboardNavigation, type KeyboardShortcut } from './useKeyboardNavigation';
import { useSetRealityLevel } from './useApi';
import { toast } from 'sonner';

interface UseRealityShortcutsOptions {
  /**
   * Callback to open preset manager dialog
   */
  onOpenPresetManager?: () => void;
  /**
   * Whether shortcuts are enabled
   */
  enabled?: boolean;
  /**
   * Default reality level for reset (default: 3)
   */
  defaultLevel?: number;
}

/**
 * Hook for managing reality slider keyboard shortcuts
 */
export function useRealityShortcuts({
  onOpenPresetManager,
  enabled = true,
  defaultLevel = 3,
}: UseRealityShortcutsOptions = {}) {
  const setLevelMutation = useSetRealityLevel();
  const onOpenPresetManagerRef = useRef(onOpenPresetManager);

  // Update ref when callback changes
  if (onOpenPresetManagerRef.current !== onOpenPresetManager) {
    onOpenPresetManagerRef.current = onOpenPresetManager;
  }

  const handleSetLevel = useCallback(
    (level: number) => {
      if (setLevelMutation.isPending) return;

      const levelNames = [
        'Static Stubs',
        'Light Simulation',
        'Moderate Realism',
        'High Realism',
        'Production Chaos',
      ];

      setLevelMutation.mutate(level, {
        onSuccess: () => {
          toast.success(`Reality level set to ${level}: ${levelNames[level - 1]}`, {
            description: 'Press Ctrl+Shift+R to reset to default',
          });
        },
        onError: (error) => {
          toast.error('Failed to set reality level', {
            description: error instanceof Error ? error.message : 'Unknown error',
          });
        },
      });
    },
    [setLevelMutation]
  );

  const handleReset = useCallback(() => {
    if (setLevelMutation.isPending) return;
    handleSetLevel(defaultLevel);
  }, [defaultLevel, handleSetLevel, setLevelMutation]);

  const handleOpenPresetManager = useCallback(() => {
    if (onOpenPresetManagerRef.current) {
      onOpenPresetManagerRef.current();
    } else {
      toast.info('Preset manager not available', {
        description: 'Navigate to Configuration > Reality Slider to manage presets',
      });
    }
  }, []);

  const shortcuts: KeyboardShortcut[] = [
    // Level 1: Static Stubs
    {
      key: '1',
      ctrl: true,
      shift: true,
      handler: () => handleSetLevel(1),
      description: 'Set reality level to 1 (Static Stubs)',
      enabled,
    },
    // Level 2: Light Simulation
    {
      key: '2',
      ctrl: true,
      shift: true,
      handler: () => handleSetLevel(2),
      description: 'Set reality level to 2 (Light Simulation)',
      enabled,
    },
    // Level 3: Moderate Realism
    {
      key: '3',
      ctrl: true,
      shift: true,
      handler: () => handleSetLevel(3),
      description: 'Set reality level to 3 (Moderate Realism)',
      enabled,
    },
    // Level 4: High Realism
    {
      key: '4',
      ctrl: true,
      shift: true,
      handler: () => handleSetLevel(4),
      description: 'Set reality level to 4 (High Realism)',
      enabled,
    },
    // Level 5: Production Chaos
    {
      key: '5',
      ctrl: true,
      shift: true,
      handler: () => handleSetLevel(5),
      description: 'Set reality level to 5 (Production Chaos)',
      enabled,
    },
    // Reset to default
    {
      key: 'r',
      ctrl: true,
      shift: true,
      handler: handleReset,
      description: `Reset reality level to ${defaultLevel} (default)`,
      enabled,
    },
    // Open preset manager
    {
      key: 'p',
      ctrl: true,
      shift: true,
      handler: handleOpenPresetManager,
      description: 'Open preset manager',
      enabled: enabled && !!onOpenPresetManager,
    },
  ];

  useKeyboardNavigation({
    shortcuts,
    enabled,
  });

  return {
    shortcuts: shortcuts.map((s) => ({
      key: s.key,
      modifiers: {
        ctrl: s.ctrl,
        shift: s.shift,
        alt: s.alt,
        meta: s.meta,
      },
      description: s.description,
    })),
  };
}
