import { create } from 'zustand';
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from './Dialog';
import { Button } from './button';

interface Confirmation { message: string; resolve: (confirmed: boolean) => void }
const useConfirmation = create<{ pending: Confirmation | null }>(() => ({ pending: null }));

/** An app dialog also works when browser-native dialogs are suppressed. */
export function confirmAction(message: string): Promise<boolean> {
  useConfirmation.getState().pending?.resolve(false);
  return new Promise((resolve) => useConfirmation.setState({ pending: { message, resolve } }));
}
export function ConfirmationDialog() {
  const pending = useConfirmation((state) => state.pending);
  const finish = (confirmed: boolean) => {
    useConfirmation.setState({ pending: null });
    pending?.resolve(confirmed);
  };
  return <Dialog open={!!pending} onOpenChange={(open) => { if (!open) finish(false); }}>
    <DialogContent>
      <DialogHeader>
        <DialogTitle id="dialog-title">Confirm action</DialogTitle>
        <DialogDescription id="dialog-description">{pending?.message}</DialogDescription>
      </DialogHeader>
      <DialogFooter>
        <Button variant="outline" autoFocus onClick={() => finish(false)}>Cancel</Button>
        <Button variant="destructive" onClick={() => finish(true)}>Confirm</Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>;
}
