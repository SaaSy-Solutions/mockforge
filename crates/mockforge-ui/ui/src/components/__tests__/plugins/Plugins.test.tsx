/**
 * @jest-environment jsdom
 */

import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { PluginList } from '../../plugins/PluginList';

// PluginList calls `authenticatedFetch` from `@/utils/apiClient`, which
// captures `globalThis.fetch` at module load — patching `global.fetch` from
// the test never reaches the component. Mock the wrapper directly so we can
// drive responses per-test.
const authenticatedFetchMock = vi.fn();
vi.mock('@/utils/apiClient', () => ({
  authenticatedFetch: (...args: unknown[]) => authenticatedFetchMock(...args),
}));

// PluginList still uses raw `fetch` for enable/disable/reload calls.
global.fetch = vi.fn();

describe('PluginList', () => {
  const mockOnSelectPlugin = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    authenticatedFetchMock.mockReset();
  });

  it('renders loading state', () => {
    authenticatedFetchMock.mockImplementationOnce(() =>
      new Promise(() => {}) // Never resolves
    );

    render(
      <PluginList
        filterType=""
        filterStatus=""
        onSelectPlugin={mockOnSelectPlugin}
      />
    );

    // Should show loading indicator
    expect(screen.queryByText('No plugins found')).not.toBeInTheDocument();
  });

  it('renders plugin list', async () => {
    const mockPlugins = {
      success: true,
      data: {
        plugins: [
          {
            id: 'plugin-1',
            name: 'Test Plugin',
            version: '1.0.0',
            types: ['response'],
            status: 'enabled',
            healthy: true,
            description: 'A test plugin',
            author: 'Test Author',
          },
        ],
      },
    };

    authenticatedFetchMock.mockResolvedValueOnce({
      json: async () => mockPlugins,
    });

    render(
      <PluginList
        filterType=""
        filterStatus=""
        onSelectPlugin={mockOnSelectPlugin}
      />
    );

    await waitFor(() => {
      expect(screen.getByText('Test Plugin')).toBeInTheDocument();
    });
  });

  it('handles errors', async () => {
    authenticatedFetchMock.mockRejectedValueOnce(new Error('Failed to fetch'));

    render(
      <PluginList
        filterType=""
        filterStatus=""
        onSelectPlugin={mockOnSelectPlugin}
      />
    );

    await waitFor(() => {
      expect(screen.getByText(/Failed to fetch plugins/i)).toBeInTheDocument();
    });
  });

  it('applies filters', async () => {
    const mockPlugins = {
      success: true,
      data: { plugins: [] },
    };

    authenticatedFetchMock.mockResolvedValueOnce({
      json: async () => mockPlugins,
    });

    render(
      <PluginList
        filterType="response"
        filterStatus="enabled"
        onSelectPlugin={mockOnSelectPlugin}
      />
    );

    await waitFor(() => {
      expect(authenticatedFetchMock).toHaveBeenCalledWith(
        expect.stringContaining('type=response')
      );
      expect(authenticatedFetchMock).toHaveBeenCalledWith(
        expect.stringContaining('status=enabled')
      );
    });
  });
});
