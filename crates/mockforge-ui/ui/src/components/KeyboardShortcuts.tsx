/**
 * Keyboard Shortcuts Component
 *
 * Displays available keyboard shortcuts and handles shortcut events.
 */

import { useEffect } from 'react';
import { isTauri, listenToTauriEvent } from '@/utils/tauri';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';

const SHORTCUTS = [
  { keys: ['Ctrl/Cmd', 'Shift', 'S'], action: 'Start Server', event: 'shortcut-start-server' },
  { keys: ['Ctrl/Cmd', 'Shift', 'X'], action: 'Stop Server', event: 'shortcut-stop-server' },
  { keys: ['Ctrl/Cmd', 'Shift', 'H'], action: 'Show/Hide Window', event: 'shortcut-show-hide' },
  { keys: ['Ctrl/Cmd', 'O'], action: 'Open Config File', event: 'shortcut-open-config' },
  { keys: ['Ctrl/Cmd', 'W'], action: 'Close Window (minimize to tray)', event: 'shortcut-close' },
  { keys: ['F11'], action: 'Toggle Fullscreen', event: 'shortcut-fullscreen' },
  { keys: ['Ctrl/Cmd', ','], action: 'Open Settings', event: 'shortcut-settings' },
];

export function KeyboardShortcuts() {
  useEffect(() => {
    if (!isTauri) return;

    // Listen for shortcut events
    const cleanup1 = listenToTauriEvent('shortcut-start-server', () => {
      // Trigger server start via custom event
      window.dispatchEvent(new CustomEvent('mockforge-start-server'));
    });

    const cleanup2 = listenToTauriEvent('shortcut-stop-server', () => {
      // Trigger server stop via custom event
      window.dispatchEvent(new CustomEvent('mockforge-stop-server'));
    });

    const cleanup3 = listenToTauriEvent('shortcut-open-config', () => {
      // Trigger file open dialog via custom event
      window.dispatchEvent(new CustomEvent('mockforge-open-config'));
    });

    return () => {
      cleanup1();
      cleanup2();
      cleanup3();
    };
  }, []);

  if (!isTauri) {
    return null; // Don't show in web version
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>Keyboard Shortcuts</CardTitle>
        <CardDescription>
          Global shortcuts for quick actions
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="space-y-2">
          {SHORTCUTS.map((shortcut, index) => (
            <div key={index} className="flex items-center justify-between py-2 border-b last:border-0">
              <span className="text-sm">{shortcut.action}</span>
              <div className="flex gap-1">
                {shortcut.keys.map((key, keyIndex) => (
                  <Badge key={keyIndex} variant="outline" className="font-mono">
                    {key}
                  </Badge>
                ))}
              </div>
            </div>
          ))}
        </div>
      </CardContent>
    </Card>
  );
}
