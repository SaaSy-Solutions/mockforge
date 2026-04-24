import { useCallback } from 'react';
import { usePreferencesStore } from '../stores/usePreferencesStore';

/**
 * Returns a `(message) => boolean` predicate that callers use in place of
 * `window.confirm` for destructive actions. When the user has disabled
 * `preferences.ui.confirmDelete` it short-circuits to true, letting the
 * action proceed without a prompt.
 *
 * Usage:
 *   const confirmDelete = useConfirmDelete();
 *   if (!confirmDelete('Delete this workspace?')) return;
 */
export function useConfirmDelete(): (message: string) => boolean {
  const confirmDeleteEnabled = usePreferencesStore(
    (s) => s.preferences.ui.confirmDelete,
  );
  return useCallback(
    (message: string) => {
      if (!confirmDeleteEnabled) return true;
      return typeof window === 'undefined' ? true : window.confirm(message);
    },
    [confirmDeleteEnabled],
  );
}
