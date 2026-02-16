/**
 * @jest-environment jsdom
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { SyncStatusIndicator } from '../SyncStatusIndicator';
import type { SyncStatus } from '../../../types';

describe('SyncStatusIndicator', () => {
  const baseStatus: SyncStatus = {
    enabled: true,
    status: 'idle',
    target_directory: '/path/to/sync',
    sync_direction: 'Manual',
    realtime_monitoring: false,
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders sync status', () => {
    render(<SyncStatusIndicator status={baseStatus} />);

    expect(screen.getByText('Idle')).toBeInTheDocument();
  });

  it('shows syncing status with spinner', () => {
    const status = { ...baseStatus, status: 'syncing' };
    render(<SyncStatusIndicator status={status} />);

    expect(screen.getByText('Syncing...')).toBeInTheDocument();
    const spinner = document.querySelector('.animate-spin');
    expect(spinner).toBeInTheDocument();
  });

  it('shows success status', () => {
    const status = { ...baseStatus, status: 'success' };
    render(<SyncStatusIndicator status={status} />);

    expect(screen.getByText('Synced')).toBeInTheDocument();
  });

  it('shows error status', () => {
    const status = { ...baseStatus, status: 'error' };
    render(<SyncStatusIndicator status={status} />);

    expect(screen.getByText('Sync Error')).toBeInTheDocument();
  });

  it('displays target directory', () => {
    render(<SyncStatusIndicator status={baseStatus} />);

    expect(screen.getByText('sync')).toBeInTheDocument(); // Shows last part of path
  });

  it('formats last sync time - just now', () => {
    const status = { ...baseStatus, last_sync: new Date().toISOString() };
    render(<SyncStatusIndicator status={status} />);

    expect(screen.getByText(/Just now/)).toBeInTheDocument();
  });

  it('formats last sync time - minutes ago', () => {
    const fiveMinutesAgo = new Date(Date.now() - 5 * 60 * 1000).toISOString();
    const status = { ...baseStatus, last_sync: fiveMinutesAgo };
    render(<SyncStatusIndicator status={status} />);

    expect(screen.getByText(/5m ago/)).toBeInTheDocument();
  });

  it('formats last sync time - hours ago', () => {
    const twoHoursAgo = new Date(Date.now() - 2 * 60 * 60 * 1000).toISOString();
    const status = { ...baseStatus, last_sync: twoHoursAgo };
    render(<SyncStatusIndicator status={status} />);

    expect(screen.getByText(/2h ago/)).toBeInTheDocument();
  });

  it('formats last sync time - days ago', () => {
    const threeDaysAgo = new Date(Date.now() - 3 * 24 * 60 * 60 * 1000).toISOString();
    const status = { ...baseStatus, last_sync: threeDaysAgo };
    render(<SyncStatusIndicator status={status} />);

    expect(screen.getByText(/3d ago/)).toBeInTheDocument();
  });

  it('shows "Never" when no last sync', () => {
    render(<SyncStatusIndicator status={baseStatus} />);

    expect(screen.queryByText(/Last sync:/)).not.toBeInTheDocument();
  });

  it('shows live indicator for bidirectional realtime sync', () => {
    const status = {
      ...baseStatus,
      sync_direction: 'Bidirectional' as const,
      realtime_monitoring: true,
    };
    render(<SyncStatusIndicator status={status} />);

    expect(screen.getByText('Live')).toBeInTheDocument();
    const pulse = document.querySelector('.animate-pulse');
    expect(pulse).toBeInTheDocument();
  });

  it('shows sync now button for manual sync', () => {
    const onSyncNow = vi.fn();
    render(<SyncStatusIndicator status={baseStatus} onSyncNow={onSyncNow} />);

    const syncButton = screen.getByRole('button', { name: '' });
    fireEvent.click(syncButton);

    expect(onSyncNow).toHaveBeenCalled();
  });

  it('disables sync now button while syncing', () => {
    const onSyncNow = vi.fn();
    const status = { ...baseStatus, status: 'syncing' };
    render(<SyncStatusIndicator status={status} onSyncNow={onSyncNow} />);

    const syncButton = screen.getByRole('button', { name: '' });
    expect(syncButton).toBeDisabled();
  });

  it('disables sync now button when loading', () => {
    const onSyncNow = vi.fn();
    render(<SyncStatusIndicator status={baseStatus} onSyncNow={onSyncNow} loading={true} />);

    const syncButton = screen.getByRole('button', { name: '' });
    expect(syncButton).toBeDisabled();
  });

  it('shows stop sync button while syncing', () => {
    const onStopSync = vi.fn();
    const status = { ...baseStatus, status: 'syncing' };
    render(<SyncStatusIndicator status={status} onStopSync={onStopSync} />);

    const stopButton = screen.getByRole('button', { name: '' });
    fireEvent.click(stopButton);

    expect(onStopSync).toHaveBeenCalled();
  });

  it('hides sync now button for bidirectional sync', () => {
    const onSyncNow = vi.fn();
    const status = { ...baseStatus, sync_direction: 'Bidirectional' as const };
    render(<SyncStatusIndicator status={status} onSyncNow={onSyncNow} />);

    const buttons = screen.queryAllByRole('button');
    expect(buttons.length).toBe(0);
  });

  it('displays correct badge variant for status', () => {
    const { rerender } = render(<SyncStatusIndicator status={{ ...baseStatus, status: 'syncing' }} />);
    let badge = screen.getByText('Syncing...').closest('.inline-flex');
    expect(badge).toHaveClass('bg-blue-100');

    rerender(<SyncStatusIndicator status={{ ...baseStatus, status: 'success' }} />);
    badge = screen.getByText('Synced').closest('.inline-flex');
    expect(badge).toHaveClass('bg-success/15');

    rerender(<SyncStatusIndicator status={{ ...baseStatus, status: 'error' }} />);
    badge = screen.getByText('Sync Error').closest('.inline-flex');
    expect(badge).toHaveClass('bg-danger/15');
  });

  it('shows tooltip for target directory', () => {
    render(<SyncStatusIndicator status={baseStatus} />);

    expect(screen.getByText('Syncing to: /path/to/sync')).toBeInTheDocument();
  });
});
