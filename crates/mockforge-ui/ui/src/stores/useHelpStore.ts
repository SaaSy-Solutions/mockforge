import { create } from 'zustand';

interface HelpStore {
  isOpen: boolean;
  open: () => void;
  close: () => void;
  setOpen: (open: boolean) => void;
}

/** Global visibility of the Help & Support modal.
 *
 * Shared so the avatar menu, the Shift+? shortcut, and anything else can
 * open the same dialog without each owning its own state. The modal itself
 * is mounted once in {@link AppShell}.
 */
export const useHelpStore = create<HelpStore>()((set) => ({
  isOpen: false,
  open: () => set({ isOpen: true }),
  close: () => set({ isOpen: false }),
  setOpen: (open) => set({ isOpen: open }),
}));
