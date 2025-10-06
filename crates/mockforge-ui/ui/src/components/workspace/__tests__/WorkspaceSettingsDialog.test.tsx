/**
 * @jest-environment jsdom
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { WorkspaceSettingsDialog } from '../WorkspaceSettingsDialog';
import type { SyncConfig } from '../../../types';

describe('WorkspaceSettingsDialog', () => {
  const mockConfig: SyncConfig = {
    enabled: false,
    target_directory: '',
    directory_structure: 'Nested',
    sync_direction: 'Manual',
    include_metadata: true,
    realtime_monitoring: false,
    filename_pattern: '{name}',
    exclude_pattern: '',
    force_overwrite: false,
  };

  const onSave = vi.fn();
  const onOpenChange = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders dialog when open', () => {
    render(
      <WorkspaceSettingsDialog
        open={true}
        onOpenChange={onOpenChange}
        workspaceId="ws-1"
        workspaceName="Test Workspace"
        onSave={onSave}
      />
    );

    expect(screen.getByText('Workspace Settings - Test Workspace')).toBeInTheDocument();
  });

  it('does not render when closed', () => {
    render(
      <WorkspaceSettingsDialog
        open={false}
        onOpenChange={onOpenChange}
        workspaceId="ws-1"
        workspaceName="Test Workspace"
        onSave={onSave}
      />
    );

    expect(screen.queryByText('Workspace Settings - Test Workspace')).not.toBeInTheDocument();
  });

  it('displays sync configuration section', () => {
    render(
      <WorkspaceSettingsDialog
        open={true}
        onOpenChange={onOpenChange}
        workspaceId="ws-1"
        workspaceName="Test Workspace"
        onSave={onSave}
      />
    );

    expect(screen.getByText('Directory Synchronization')).toBeInTheDocument();
    expect(screen.getByText('Enable Directory Sync')).toBeInTheDocument();
  });

  it('toggles sync enabled', () => {
    render(
      <WorkspaceSettingsDialog
        open={true}
        onOpenChange={onOpenChange}
        workspaceId="ws-1"
        workspaceName="Test Workspace"
        onSave={onSave}
      />
    );

    const toggle = screen.getByRole('switch', { name: /Enable Directory Sync/ });
    expect(toggle).not.toBeChecked();

    fireEvent.click(toggle);
    expect(toggle).toBeChecked();
  });

  it('shows sync options when enabled', () => {
    render(
      <WorkspaceSettingsDialog
        open={true}
        onOpenChange={onOpenChange}
        workspaceId="ws-1"
        workspaceName="Test Workspace"
        currentConfig={{ ...mockConfig, enabled: true }}
        onSave={onSave}
      />
    );

    expect(screen.getByLabelText('Target Directory')).toBeInTheDocument();
    expect(screen.getByLabelText('Sync Direction')).toBeInTheDocument();
    expect(screen.getByLabelText('Directory Structure')).toBeInTheDocument();
  });

  it('hides sync options when disabled', () => {
    render(
      <WorkspaceSettingsDialog
        open={true}
        onOpenChange={onOpenChange}
        workspaceId="ws-1"
        workspaceName="Test Workspace"
        currentConfig={mockConfig}
        onSave={onSave}
      />
    );

    expect(screen.queryByLabelText('Target Directory')).not.toBeInTheDocument();
  });

  it('updates target directory', () => {
    render(
      <WorkspaceSettingsDialog
        open={true}
        onOpenChange={onOpenChange}
        workspaceId="ws-1"
        workspaceName="Test Workspace"
        currentConfig={{ ...mockConfig, enabled: true }}
        onSave={onSave}
      />
    );

    const input = screen.getByPlaceholderText('/path/to/sync/directory');
    fireEvent.change(input, { target: { value: '/new/path' } });

    expect(input).toHaveValue('/new/path');
  });

  it('selects sync direction', () => {
    render(
      <WorkspaceSettingsDialog
        open={true}
        onOpenChange={onOpenChange}
        workspaceId="ws-1"
        workspaceName="Test Workspace"
        currentConfig={{ ...mockConfig, enabled: true }}
        onSave={onSave}
      />
    );

    const select = screen.getByRole('combobox', { name: /Sync Direction/ });
    fireEvent.change(select, { target: { value: 'Bidirectional' } });

    expect(select).toHaveValue('Bidirectional');
  });

  it('selects directory structure', () => {
    render(
      <WorkspaceSettingsDialog
        open={true}
        onOpenChange={onOpenChange}
        workspaceId="ws-1"
        workspaceName="Test Workspace"
        currentConfig={{ ...mockConfig, enabled: true }}
        onSave={onSave}
      />
    );

    const select = screen.getByRole('combobox', { name: /Directory Structure/ });
    fireEvent.change(select, { target: { value: 'Flat' } });

    expect(select).toHaveValue('Flat');
  });

  it('updates filename pattern', () => {
    render(
      <WorkspaceSettingsDialog
        open={true}
        onOpenChange={onOpenChange}
        workspaceId="ws-1"
        workspaceName="Test Workspace"
        currentConfig={{ ...mockConfig, enabled: true }}
        onSave={onSave}
      />
    );

    const input = screen.getByPlaceholderText('{name}');
    fireEvent.change(input, { target: { value: '{id}_{name}' } });

    expect(input).toHaveValue('{id}_{name}');
  });

  it('toggles realtime monitoring', () => {
    render(
      <WorkspaceSettingsDialog
        open={true}
        onOpenChange={onOpenChange}
        workspaceId="ws-1"
        workspaceName="Test Workspace"
        currentConfig={{ ...mockConfig, enabled: true }}
        onSave={onSave}
      />
    );

    const toggle = screen.getByRole('switch', { name: /Real-time Monitoring/ });
    expect(toggle).not.toBeChecked();

    fireEvent.click(toggle);
    expect(toggle).toBeChecked();
  });

  it('toggles include metadata', () => {
    render(
      <WorkspaceSettingsDialog
        open={true}
        onOpenChange={onOpenChange}
        workspaceId="ws-1"
        workspaceName="Test Workspace"
        currentConfig={{ ...mockConfig, enabled: true }}
        onSave={onSave}
      />
    );

    const toggle = screen.getByRole('switch', { name: /Include Metadata/ });
    expect(toggle).toBeChecked(); // Default is true

    fireEvent.click(toggle);
    expect(toggle).not.toBeChecked();
  });

  it('toggles force overwrite', () => {
    render(
      <WorkspaceSettingsDialog
        open={true}
        onOpenChange={onOpenChange}
        workspaceId="ws-1"
        workspaceName="Test Workspace"
        currentConfig={{ ...mockConfig, enabled: true }}
        onSave={onSave}
      />
    );

    const toggle = screen.getByRole('switch', { name: /Force Overwrite/ });
    expect(toggle).not.toBeChecked();

    fireEvent.click(toggle);
    expect(toggle).toBeChecked();
  });

  it('updates exclude pattern', () => {
    render(
      <WorkspaceSettingsDialog
        open={true}
        onOpenChange={onOpenChange}
        workspaceId="ws-1"
        workspaceName="Test Workspace"
        currentConfig={{ ...mockConfig, enabled: true }}
        onSave={onSave}
      />
    );

    const input = screen.getByPlaceholderText('*.tmp,*.log');
    fireEvent.change(input, { target: { value: '*.bak,*.old' } });

    expect(input).toHaveValue('*.bak,*.old');
  });

  it('saves configuration', async () => {
    render(
      <WorkspaceSettingsDialog
        open={true}
        onOpenChange={onOpenChange}
        workspaceId="ws-1"
        workspaceName="Test Workspace"
        currentConfig={{ ...mockConfig, enabled: true, target_directory: '/path' }}
        onSave={onSave}
      />
    );

    const saveButton = screen.getByText('Save Settings');
    fireEvent.click(saveButton);

    await waitFor(() => {
      expect(onSave).toHaveBeenCalledWith(
        expect.objectContaining({
          enabled: true,
          target_directory: '/path',
        })
      );
    });
  });

  it('closes dialog after save', async () => {
    const mockOnSave = vi.fn().mockResolvedValue(undefined);
    render(
      <WorkspaceSettingsDialog
        open={true}
        onOpenChange={onOpenChange}
        workspaceId="ws-1"
        workspaceName="Test Workspace"
        currentConfig={mockConfig}
        onSave={mockOnSave}
      />
    );

    fireEvent.click(screen.getByText('Save Settings'));

    await waitFor(() => {
      expect(onOpenChange).toHaveBeenCalledWith(false);
    });
  });

  it('cancels and closes dialog', () => {
    render(
      <WorkspaceSettingsDialog
        open={true}
        onOpenChange={onOpenChange}
        workspaceId="ws-1"
        workspaceName="Test Workspace"
        onSave={onSave}
      />
    );

    fireEvent.click(screen.getByText('Cancel'));

    expect(onOpenChange).toHaveBeenCalledWith(false);
    expect(onSave).not.toHaveBeenCalled();
  });

  it('disables buttons while loading', () => {
    render(
      <WorkspaceSettingsDialog
        open={true}
        onOpenChange={onOpenChange}
        workspaceId="ws-1"
        workspaceName="Test Workspace"
        onSave={onSave}
        loading={true}
      />
    );

    expect(screen.getByText('Cancel')).toBeDisabled();
    expect(screen.getByText('Saving...')).toBeDisabled();
  });

  it('loads current config on mount', () => {
    const currentConfig: SyncConfig = {
      enabled: true,
      target_directory: '/current/path',
      directory_structure: 'Flat',
      sync_direction: 'Bidirectional',
      include_metadata: false,
      realtime_monitoring: true,
      filename_pattern: '{id}',
      exclude_pattern: '*.tmp',
      force_overwrite: true,
    };

    render(
      <WorkspaceSettingsDialog
        open={true}
        onOpenChange={onOpenChange}
        workspaceId="ws-1"
        workspaceName="Test Workspace"
        currentConfig={currentConfig}
        onSave={onSave}
      />
    );

    expect(screen.getByPlaceholderText('/path/to/sync/directory')).toHaveValue('/current/path');
    expect(screen.getByPlaceholderText('{name}')).toHaveValue('{id}');
    expect(screen.getByPlaceholderText('*.tmp,*.log')).toHaveValue('*.tmp');
  });

  it('handles save error gracefully', async () => {
    const mockOnSave = vi.fn().mockRejectedValue(new Error('Save failed'));
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    render(
      <WorkspaceSettingsDialog
        open={true}
        onOpenChange={onOpenChange}
        workspaceId="ws-1"
        workspaceName="Test Workspace"
        onSave={mockOnSave}
      />
    );

    fireEvent.click(screen.getByText('Save Settings'));

    await waitFor(() => {
      expect(consoleSpy).toHaveBeenCalled();
    });

    consoleSpy.mockRestore();
  });

  it('shows all sync direction options', () => {
    render(
      <WorkspaceSettingsDialog
        open={true}
        onOpenChange={onOpenChange}
        workspaceId="ws-1"
        workspaceName="Test Workspace"
        currentConfig={{ ...mockConfig, enabled: true }}
        onSave={onSave}
      />
    );

    const select = screen.getByRole('combobox', { name: /Sync Direction/ });
    const options = select.querySelectorAll('option');

    expect(options).toHaveLength(3);
    expect(options[0]).toHaveValue('Manual');
    expect(options[1]).toHaveValue('WorkspaceToDirectory');
    expect(options[2]).toHaveValue('Bidirectional');
  });

  it('shows all directory structure options', () => {
    render(
      <WorkspaceSettingsDialog
        open={true}
        onOpenChange={onOpenChange}
        workspaceId="ws-1"
        workspaceName="Test Workspace"
        currentConfig={{ ...mockConfig, enabled: true }}
        onSave={onSave}
      />
    );

    const select = screen.getByRole('combobox', { name: /Directory Structure/ });
    const options = select.querySelectorAll('option');

    expect(options).toHaveLength(3);
    expect(options[0]).toHaveValue('Flat');
    expect(options[1]).toHaveValue('Nested');
    expect(options[2]).toHaveValue('Grouped');
  });
});
