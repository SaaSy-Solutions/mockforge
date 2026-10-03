import { confirmAction } from '../components/ui/ConfirmationDialog';
import { useCallback } from 'react';
import { usePreferencesStore } from '../stores/usePreferencesStore';

/**
 * Returns a `(message) => Promise<boolean>` confirmation that callers use in place of
 * `window.confirm` for destructive actions. When the user has disabled
 * `preferences.ui.confirmDelete` it short-circuits to true, letting the
 * action proceed without a prompt.
 *
 * Usage:
 *   const confirmDelete = useConfirmDelete();
 *   if (!await confirmDelete('Delete this workspace?')) return;
 */
export function useConfirmDelete(): (message: string) => Promise<boolean> {
  const confirmDeleteEnabled = usePreferencesStore(
    (s) => s.preferences.ui.confirmDelete,
  );
  return useCallback(
    async (message: string) => {
      if (!confirmDeleteEnabled) return true;
      return confirmAction(message);
    },
    [confirmDeleteEnabled],
  );
}
