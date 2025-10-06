/**
 * @jest-environment jsdom
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { SyncConfirmationDialog } from '../SyncConfirmationDialog';

const mockChanges = [
  {
    change_type: 'created',
    path: '/new-file.json',
    description: 'New file created',
    requires_confirmation: true,
  },
  {
    change_type: 'modified',
    path: '/existing-file.json',
    description: 'File modified',
    requires_confirmation: true,
  },
  {
    change_type: 'deleted',
    path: '/old-file.json',
    description: 'File deleted',
    requires_confirmation: false,
  },
];

describe('SyncConfirmationDialog', () => {
  const onConfirm = vi.fn();
  const onOpenChange = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders dialog when open', () => {
    render(
      <SyncConfirmationDialog
        open={true}
        onOpenChange={onOpenChange}
        workspaceId="ws-1"
        changes={mockChanges}
        onConfirm={onConfirm}
      />
    );

    expect(screen.getByText('Confirm Directory Sync Changes')).toBeInTheDocument();
  });

  it('does not render when closed', () => {
    render(
      <SyncConfirmationDialog
        open={false}
        onOpenChange={onOpenChange}
        workspaceId="ws-1"
        changes={mockChanges}
        onConfirm={onConfirm}
      />
    );

    expect(screen.queryByText('Confirm Directory Sync Changes')).not.toBeInTheDocument();
  });

  it('displays workspace id in description', () => {
    render(
      <SyncConfirmationDialog
        open={true}
        onOpenChange={onOpenChange}
        workspaceId="my-workspace"
        changes={mockChanges}
        onConfirm={onConfirm}
      />
    );

    expect(screen.getByText(/workspace "my-workspace"/)).toBeInTheDocument();
  });

  it('shows changes requiring confirmation', () => {
    render(
      <SyncConfirmationDialog
        open={true}
        onOpenChange={onOpenChange}
        workspaceId="ws-1"
        changes={mockChanges}
        onConfirm={onConfirm}
      />
    );

    expect(screen.getByText('Changes requiring confirmation:')).toBeInTheDocument();
    expect(screen.getByText('/new-file.json')).toBeInTheDocument();
    expect(screen.getByText('/existing-file.json')).toBeInTheDocument();
  });

  it('shows auto-applied changes', () => {
    render(
      <SyncConfirmationDialog
        open={true}
        onOpenChange={onOpenChange}
        workspaceId="ws-1"
        changes={mockChanges}
        onConfirm={onConfirm}
      />
    );

    expect(screen.getByText('Changes that will be applied automatically:')).toBeInTheDocument();
    expect(screen.getByText('/old-file.json')).toBeInTheDocument();
  });

  it('displays change types with badges', () => {
    render(
      <SyncConfirmationDialog
        open={true}
        onOpenChange={onOpenChange}
        workspaceId="ws-1"
        changes={mockChanges}
        onConfirm={onConfirm}
      />
    );

    expect(screen.getByText('created')).toBeInTheDocument();
    expect(screen.getByText('modified')).toBeInTheDocument();
    expect(screen.getByText('deleted')).toBeInTheDocument();
  });

  it('shows change descriptions', () => {
    render(
      <SyncConfirmationDialog
        open={true}
        onOpenChange={onOpenChange}
        workspaceId="ws-1"
        changes={mockChanges}
        onConfirm={onConfirm}
      />
    );

    expect(screen.getByText('New file created')).toBeInTheDocument();
    expect(screen.getByText('File modified')).toBeInTheDocument();
    expect(screen.getByText('File deleted')).toBeInTheDocument();
  });

  it('shows apply all checkbox when there are confirmable changes', () => {
    render(
      <SyncConfirmationDialog
        open={true}
        onOpenChange={onOpenChange}
        workspaceId="ws-1"
        changes={mockChanges}
        onConfirm={onConfirm}
      />
    );

    expect(screen.getByLabelText('Apply all changes including those requiring confirmation')).toBeInTheDocument();
  });

  it('toggles apply all checkbox', () => {
    render(
      <SyncConfirmationDialog
        open={true}
        onOpenChange={onOpenChange}
        workspaceId="ws-1"
        changes={mockChanges}
        onConfirm={onConfirm}
      />
    );

    const checkbox = screen.getByRole('checkbox');
    expect(checkbox).not.toBeChecked();

    fireEvent.click(checkbox);
    expect(checkbox).toBeChecked();
  });

  it('confirms changes with apply all flag', async () => {
    render(
      <SyncConfirmationDialog
        open={true}
        onOpenChange={onOpenChange}
        workspaceId="ws-1"
        changes={mockChanges}
        onConfirm={onConfirm}
      />
    );

    const checkbox = screen.getByRole('checkbox');
    fireEvent.click(checkbox);

    const confirmButton = screen.getByText('Confirm Changes');
    fireEvent.click(confirmButton);

    await waitFor(() => {
      expect(onConfirm).toHaveBeenCalledWith(true);
    });
  });

  it('confirms changes without apply all flag', async () => {
    render(
      <SyncConfirmationDialog
        open={true}
        onOpenChange={onOpenChange}
        workspaceId="ws-1"
        changes={mockChanges}
        onConfirm={onConfirm}
      />
    );

    const confirmButton = screen.getByText('Confirm Changes');
    fireEvent.click(confirmButton);

    await waitFor(() => {
      expect(onConfirm).toHaveBeenCalledWith(false);
    });
  });

  it('closes dialog on cancel', () => {
    render(
      <SyncConfirmationDialog
        open={true}
        onOpenChange={onOpenChange}
        workspaceId="ws-1"
        changes={mockChanges}
        onConfirm={onConfirm}
      />
    );

    fireEvent.click(screen.getByText('Cancel'));

    expect(onOpenChange).toHaveBeenCalledWith(false);
  });

  it('closes dialog after confirmation', async () => {
    const mockOnConfirm = vi.fn().mockResolvedValue(undefined);
    render(
      <SyncConfirmationDialog
        open={true}
        onOpenChange={onOpenChange}
        workspaceId="ws-1"
        changes={mockChanges}
        onConfirm={mockOnConfirm}
      />
    );

    fireEvent.click(screen.getByText('Confirm Changes'));

    await waitFor(() => {
      expect(onOpenChange).toHaveBeenCalledWith(false);
    });
  });

  it('disables buttons while loading', () => {
    render(
      <SyncConfirmationDialog
        open={true}
        onOpenChange={onOpenChange}
        workspaceId="ws-1"
        changes={mockChanges}
        onConfirm={onConfirm}
        loading={true}
      />
    );

    expect(screen.getByText('Cancel')).toBeDisabled();
    expect(screen.getByText('Applying...')).toBeDisabled();
  });

  it('shows empty state when no changes', () => {
    render(
      <SyncConfirmationDialog
        open={true}
        onOpenChange={onOpenChange}
        workspaceId="ws-1"
        changes={[]}
        onConfirm={onConfirm}
      />
    );

    expect(screen.getByText('No changes detected.')).toBeInTheDocument();
    expect(screen.getByText('Confirm Changes')).toBeDisabled();
  });

  it('displays correct icons for change types', () => {
    render(
      <SyncConfirmationDialog
        open={true}
        onOpenChange={onOpenChange}
        workspaceId="ws-1"
        changes={mockChanges}
        onConfirm={onConfirm}
      />
    );

    const greenIcons = document.querySelectorAll('.text-green-500');
    expect(greenIcons.length).toBeGreaterThan(0); // created

    const blueIcons = document.querySelectorAll('.text-blue-500');
    expect(blueIcons.length).toBeGreaterThan(0); // modified

    const redIcons = document.querySelectorAll('.text-red-500');
    expect(redIcons.length).toBeGreaterThan(0); // deleted
  });

  it('hides apply all checkbox when no confirmable changes', () => {
    const autoChanges = [
      {
        change_type: 'created',
        path: '/auto-file.json',
        description: 'Auto applied',
        requires_confirmation: false,
      },
    ];

    render(
      <SyncConfirmationDialog
        open={true}
        onOpenChange={onOpenChange}
        workspaceId="ws-1"
        changes={autoChanges}
        onConfirm={onConfirm}
      />
    );

    expect(
      screen.queryByLabelText('Apply all changes including those requiring confirmation')
    ).not.toBeInTheDocument();
  });

  it('handles confirmation error gracefully', async () => {
    const mockOnConfirm = vi.fn().mockRejectedValue(new Error('Sync failed'));
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    render(
      <SyncConfirmationDialog
        open={true}
        onOpenChange={onOpenChange}
        workspaceId="ws-1"
        changes={mockChanges}
        onConfirm={mockOnConfirm}
      />
    );

    fireEvent.click(screen.getByText('Confirm Changes'));

    await waitFor(() => {
      expect(consoleSpy).toHaveBeenCalled();
    });

    consoleSpy.mockRestore();
  });
});
