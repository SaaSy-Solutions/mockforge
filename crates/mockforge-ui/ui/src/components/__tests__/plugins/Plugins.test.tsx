/**
 * @jest-environment jsdom
 */

import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { PluginList } from '../../plugins/PluginList';

// Mock fetch
global.fetch = vi.fn();

describe('PluginList', () => {
  const mockOnSelectPlugin = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders loading state', () => {
    (global.fetch as any).mockImplementationOnce(() =>
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

    (global.fetch as any).mockResolvedValueOnce({
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
    (global.fetch as any).mockRejectedValueOnce(new Error('Failed to fetch'));

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

    (global.fetch as any).mockResolvedValueOnce({
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
      expect(global.fetch).toHaveBeenCalledWith(
        expect.stringContaining('type=response')
      );
      expect(global.fetch).toHaveBeenCalledWith(
        expect.stringContaining('status=enabled')
      );
    });
  });
});
