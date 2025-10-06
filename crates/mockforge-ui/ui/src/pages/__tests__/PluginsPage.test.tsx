/**
 * @jest-environment jsdom
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { PluginsPage } from '../PluginsPage';

vi.mock('../../services/api', () => ({
  pluginsApi: {
    reloadAllPlugins: vi.fn().mockResolvedValue({ message: 'Plugins reloaded' }),
  },
}));

vi.mock('../../components/plugins/PluginList', () => ({
  PluginList: ({ filterType, filterStatus }: { filterType: string; filterStatus: string }) => (
    <div data-testid="plugin-list">
      Plugin List - Type: {filterType || 'all'} - Status: {filterStatus || 'all'}
    </div>
  ),
}));

vi.mock('../../components/plugins/PluginStatus', () => ({
  PluginStatus: () => <div>Plugin Status Component</div>,
}));

vi.mock('../../components/plugins/PluginDetails', () => ({
  PluginDetails: ({ pluginId, onClose }: { pluginId: string; onClose: () => void }) => (
    <div>
      Plugin Details: {pluginId}
      <button onClick={onClose}>Close</button>
    </div>
  ),
}));

vi.mock('../../components/plugins/InstallPluginModal', () => ({
  InstallPluginModal: ({ onClose }: { onClose: () => void }) => (
    <div>
      Install Plugin Modal
      <button onClick={onClose}>Cancel</button>
    </div>
  ),
}));

vi.mock('sonner', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

describe('PluginsPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    global.open = vi.fn();
  });

  it('renders plugins page header', () => {
    render(<PluginsPage />);

    expect(screen.getByText('Plugin Management')).toBeInTheDocument();
    expect(screen.getByText(/Manage authentication, template, response, and datasource plugins/)).toBeInTheDocument();
  });

  it('displays install and reload buttons', () => {
    render(<PluginsPage />);

    expect(screen.getByText('Install Plugin')).toBeInTheDocument();
    expect(screen.getByText('Reload All')).toBeInTheDocument();
  });

  it('opens install plugin modal', () => {
    render(<PluginsPage />);

    const installButton = screen.getByText('Install Plugin');
    fireEvent.click(installButton);

    expect(screen.getByText('Install Plugin Modal')).toBeInTheDocument();
  });

  it('closes install plugin modal', () => {
    render(<PluginsPage />);

    fireEvent.click(screen.getByText('Install Plugin'));
    expect(screen.getByText('Install Plugin Modal')).toBeInTheDocument();

    const cancelButton = screen.getByText('Cancel');
    fireEvent.click(cancelButton);

    expect(screen.queryByText('Install Plugin Modal')).not.toBeInTheDocument();
  });

  it('reloads all plugins', async () => {

    render(<PluginsPage />);

    const reloadButton = screen.getByText('Reload All');
    fireEvent.click(reloadButton);

    await waitFor(() => {
      expect(pluginsApi.reloadAllPlugins).toHaveBeenCalled();
    });
  });

  it('disables reload button during reload', async () => {
    let resolveReload: () => void;
    pluginsApi.reloadAllPlugins.mockReturnValue(
      new Promise((resolve) => {
        resolveReload = resolve;
      })
    );

    render(<PluginsPage />);

    const reloadButton = screen.getByText('Reload All');
    fireEvent.click(reloadButton);

    expect(reloadButton).toBeDisabled();

    resolveReload!();
    await waitFor(() => {
      expect(reloadButton).not.toBeDisabled();
    });
  });

  it('displays error when reload fails', async () => {
    pluginsApi.reloadAllPlugins.mockRejectedValue(new Error('Reload failed'));

    render(<PluginsPage />);

    fireEvent.click(screen.getByText('Reload All'));

    await waitFor(() => {
      expect(screen.getByText(/Reload failed/)).toBeInTheDocument();
    });
  });

  it('filters plugins by search query', () => {
    render(<PluginsPage />);

    const searchInput = screen.getByPlaceholderText('Search plugins by name or description...');
    fireEvent.change(searchInput, { target: { value: 'auth' } });

    expect(searchInput).toHaveValue('auth');
  });

  it('filters plugins by type', () => {
    render(<PluginsPage />);

    const typeInput = screen.getByPlaceholderText('Filter by type');
    fireEvent.change(typeInput, { target: { value: 'authentication' } });

    expect(typeInput).toHaveValue('authentication');
    expect(screen.getByTestId('plugin-list')).toHaveTextContent('Type: authentication');
  });

  it('filters plugins by status', () => {
    render(<PluginsPage />);

    const statusInput = screen.getByPlaceholderText('Filter by status');
    fireEvent.change(statusInput, { target: { value: 'active' } });

    expect(statusInput).toHaveValue('active');
    expect(screen.getByTestId('plugin-list')).toHaveTextContent('Status: active');
  });

  it('displays plugin type datalist options', () => {
    render(<PluginsPage />);

    const typeDatalist = document.getElementById('plugin-types');
    expect(typeDatalist).toBeInTheDocument();
    expect(typeDatalist?.querySelectorAll('option')).toHaveLength(4);
  });

  it('displays plugin status datalist options', () => {
    render(<PluginsPage />);

    const statusDatalist = document.getElementById('plugin-statuses');
    expect(statusDatalist).toBeInTheDocument();
    expect(statusDatalist?.querySelectorAll('option')).toHaveLength(4);
  });

  it('switches to installed plugins tab', () => {
    render(<PluginsPage />);

    const installedTab = screen.getByText('Installed Plugins');
    fireEvent.click(installedTab);

    expect(screen.getByTestId('plugin-list')).toBeInTheDocument();
  });

  it('switches to status tab', () => {
    render(<PluginsPage />);

    const statusTab = screen.getByText('System Status');
    fireEvent.click(statusTab);

    expect(screen.getByText('Plugin Status Component')).toBeInTheDocument();
  });

  it('switches to marketplace tab', () => {
    render(<PluginsPage />);

    const marketplaceTab = screen.getByText('Marketplace');
    fireEvent.click(marketplaceTab);

    expect(screen.getByText('Plugin Marketplace')).toBeInTheDocument();
    expect(screen.getByText('Browse and install plugins from the official marketplace')).toBeInTheDocument();
  });

  it('opens marketplace in new tab', () => {
    render(<PluginsPage />);

    fireEvent.click(screen.getByText('Marketplace'));

    const browseButton = screen.getByText('Browse Marketplace');
    fireEvent.click(browseButton);

    expect(global.open).toHaveBeenCalledWith('/plugins/marketplace', '_blank');
  });

  it('re-renders plugin list when reload key changes', async () => {

    render(<PluginsPage />);

    const initialList = screen.getByTestId('plugin-list');

    fireEvent.click(screen.getByText('Reload All'));

    await waitFor(() => {
      expect(pluginsApi.reloadAllPlugins).toHaveBeenCalled();
    });

    // Plugin list should re-render with new key
    expect(screen.getByTestId('plugin-list')).toBeInTheDocument();
  });
});
