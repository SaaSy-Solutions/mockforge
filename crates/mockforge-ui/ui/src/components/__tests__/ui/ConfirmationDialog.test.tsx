import { act, render, screen, fireEvent } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { ConfirmationDialog, confirmAction } from '../../ui/ConfirmationDialog';
describe('in-app destructive confirmation', () => {
  it('requires confirmation and supports cancellation when native dialogs are suppressed', async () => {
    render(<ConfirmationDialog />);
    let decision!: Promise<boolean>;
    act(() => { decision = confirmAction('Delete QA-TEST Service?'); });
    expect(screen.getByRole('dialog')).toBeVisible();
    expect(screen.getByText('Delete QA-TEST Service?')).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    expect(await decision).toBe(false);
    act(() => { decision = confirmAction('Delete QA-TEST Service?'); });
    fireEvent.click(screen.getByRole('button', { name: 'Confirm' }));
    expect(await decision).toBe(true);
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });
});
