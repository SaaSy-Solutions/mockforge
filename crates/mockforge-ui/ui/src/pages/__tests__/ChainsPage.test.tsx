/**
 * @jest-environment jsdom
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { ChainsPage } from '../ChainsPage';
import { apiService } from '../../services/api';
import type { ChainSummary, ChainDefinition } from '../../types/chains';

vi.mock('../../services/api');
vi.mock('sonner', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

describe('ChainsPage', () => {
  const mockChains: ChainSummary[] = [
    {
      id: 'chain-1',
      name: 'Test Chain 1',
      description: 'Test chain description',
      enabled: true,
      linkCount: 3,
      tags: ['test', 'example'],
    },
    {
      id: 'chain-2',
      name: 'Test Chain 2',
      description: 'Another test chain',
      enabled: false,
      linkCount: 2,
      tags: [],
    },
  ];

  const mockChainDetails: ChainDefinition = {
    id: 'chain-1',
    name: 'Test Chain 1',
    description: 'Test chain description',
    config: {
      enabled: true,
      maxChainLength: 10,
      globalTimeoutSecs: 60,
      enableParallelExecution: false,
    },
    links: [
      {
        request: {
          id: 'step1',
          method: 'GET',
          url: 'https://api.example.com/data',
          headers: { 'Content-Type': 'application/json' },
        },
        storeAs: 'step1_response',
        dependsOn: [],
      },
    ],
    variables: { base_url: 'https://api.example.com' },
    tags: ['test', 'example'],
  };

  beforeEach(() => {
    vi.clearAllMocks();
    (apiService.listChains as any) = vi.fn().mockResolvedValue({ chains: mockChains });
  });

  it('renders loading state initially', () => {
    (apiService.listChains as any) = vi.fn(() => new Promise(() => {}));
    render(<ChainsPage />);
    expect(screen.getByText('Loading chains...')).toBeInTheDocument();
  });

  it('fetches and displays chains', async () => {
    render(<ChainsPage />);

    await waitFor(() => {
      expect(screen.getByText('Test Chain 1')).toBeInTheDocument();
      expect(screen.getByText('Test Chain 2')).toBeInTheDocument();
    });
  });

  it('displays chain metadata correctly', async () => {
    render(<ChainsPage />);

    await waitFor(() => {
      expect(screen.getByText('Test chain description')).toBeInTheDocument();
      expect(screen.getByText('3')).toBeInTheDocument(); // link count
      expect(screen.getByText('test')).toBeInTheDocument(); // tag
    });
  });

  it('shows enabled/disabled badges', async () => {
    render(<ChainsPage />);

    await waitFor(() => {
      const enabledBadges = screen.getAllByText('Enabled');
      const disabledBadges = screen.getAllByText('Disabled');
      expect(enabledBadges.length).toBeGreaterThan(0);
      expect(disabledBadges.length).toBeGreaterThan(0);
    });
  });

  it('displays empty state when no chains exist', async () => {
    (apiService.listChains as any) = vi.fn().mockResolvedValue({ chains: [] });
    render(<ChainsPage />);

    await waitFor(() => {
      expect(screen.getByText('No Chains Found')).toBeInTheDocument();
      expect(screen.getByText(/Create your first request chain/)).toBeInTheDocument();
    });
  });

  it('handles error state gracefully', async () => {
    const errorMessage = 'Failed to load chains';
    (apiService.listChains as any) = vi.fn().mockRejectedValue(new Error(errorMessage));
    render(<ChainsPage />);

    await waitFor(() => {
      expect(screen.getByText(/Failed to load chains/)).toBeInTheDocument();
    });
  });

  it('opens create chain dialog', async () => {
    render(<ChainsPage />);

    await waitFor(() => {
      const createButton = screen.getByText('Create Chain');
      fireEvent.click(createButton);
    });

    expect(screen.getByText('Create a new request chain using YAML definition.')).toBeInTheDocument();
  });

  it('shows default YAML in create dialog', async () => {
    render(<ChainsPage />);

    await waitFor(() => {
      fireEvent.click(screen.getByText('Create Chain'));
    });

    const textarea = screen.getByPlaceholderText('Enter YAML chain definition...');
    expect((textarea as HTMLTextAreaElement).value).toContain('id: my-chain');
  });

  it('loads example YAML when button is clicked', async () => {
    render(<ChainsPage />);

    await waitFor(() => {
      fireEvent.click(screen.getByText('Create Chain'));
    });

    const loadExampleButton = screen.getByText('Load Example');
    fireEvent.click(loadExampleButton);

    const textarea = screen.getByPlaceholderText('Enter YAML chain definition...');
    expect((textarea as HTMLTextAreaElement).value).toContain('User Management Workflow');
  });

  it('creates a new chain successfully', async () => {
    const createResponse = { id: 'new-chain' };
    (apiService.createChain as any) = vi.fn().mockResolvedValue(createResponse);
    (apiService.listChains as any) = vi
      .fn()
      .mockResolvedValueOnce({ chains: mockChains })
      .mockResolvedValueOnce({
        chains: [...mockChains, { id: 'new-chain', name: 'New Chain', linkCount: 1, enabled: true }],
      });

    render(<ChainsPage />);

    await waitFor(() => {
      fireEvent.click(screen.getByText('Create Chain'));
    });

    const createButton = screen.getAllByText('Create Chain').pop()!;
    fireEvent.click(createButton);

    await waitFor(() => {
      expect(apiService.createChain).toHaveBeenCalled();
    });
  });

  it('opens delete confirmation dialog', async () => {
    render(<ChainsPage />);

    await waitFor(() => {
      const deleteButtons = screen.getAllByRole('button', { name: /Delete/ });
      fireEvent.click(deleteButtons[0]);
    });

    expect(screen.getByText(/Are you sure you want to delete the chain/)).toBeInTheDocument();
  });

  it('deletes a chain successfully', async () => {
    (apiService.deleteChain as any) = vi.fn().mockResolvedValue({});
    render(<ChainsPage />);

    await waitFor(() => {
      const deleteButtons = screen.getAllByRole('button', { name: /Delete/ });
      fireEvent.click(deleteButtons[0]);
    });

    const confirmButton = screen.getAllByRole('button', { name: 'Delete' }).at(-1)!;
    fireEvent.click(confirmButton);

    await waitFor(() => {
      expect(apiService.deleteChain).toHaveBeenCalledWith('chain-1');
    });
  });

  it('views chain details', async () => {
    (apiService.getChain as any) = vi.fn().mockResolvedValue(mockChainDetails);
    render(<ChainsPage />);

    await waitFor(() => {
      const viewButtons = screen.getAllByText('View');
      fireEvent.click(viewButtons[0]);
    });

    await waitFor(() => {
      expect(apiService.getChain).toHaveBeenCalledWith('chain-1');
      expect(screen.getByText('Request Links (1)')).toBeInTheDocument();
    });
  });

  it('executes a chain', async () => {
    const executionResult = { status: 'success', data: { result: 'ok' } };
    (apiService.executeChain as any) = vi.fn().mockResolvedValue(executionResult);
    render(<ChainsPage />);

    await waitFor(() => {
      const executeButtons = screen.getAllByText('Execute');
      fireEvent.click(executeButtons[0]);
    });

    await waitFor(() => {
      expect(apiService.executeChain).toHaveBeenCalledWith('chain-1');
      expect(screen.getByText(/Execution Result/)).toBeInTheDocument();
    });
  });

  it('disables execute button for disabled chains', async () => {
    render(<ChainsPage />);

    await waitFor(() => {
      const executeButtons = screen.getAllByText('Execute');
      // Second chain is disabled
      expect(executeButtons[1]).toBeDisabled();
    });
  });

  it('displays chain configuration correctly', async () => {
    (apiService.getChain as any) = vi.fn().mockResolvedValue(mockChainDetails);
    render(<ChainsPage />);

    await waitFor(() => {
      fireEvent.click(screen.getAllByText('View')[0]);
    });

    await waitFor(() => {
      expect(screen.getByText('Global Timeout:')).toBeInTheDocument();
      expect(screen.getByText('60s')).toBeInTheDocument();
      expect(screen.getByText('Parallel Execution:')).toBeInTheDocument();
      expect(screen.getAllByText('Disabled').length).toBeGreaterThan(0);
    });
  });

  it('handles chain execution errors', async () => {
    (apiService.executeChain as any) = vi.fn().mockRejectedValue(new Error('Execution failed'));
    render(<ChainsPage />);

    await waitFor(() => {
      fireEvent.click(screen.getAllByText('Execute')[0]);
    });

    await waitFor(() => {
      expect(screen.getByText(/Error: Execution failed/)).toBeInTheDocument();
    });
  });

  it('handles API not available error', async () => {
    const error = new Error('not valid JSON');
    (apiService.listChains as any) = vi.fn().mockRejectedValue(error);
    render(<ChainsPage />);

    await waitFor(() => {
      expect(
        screen.getByText(/Chain API is not available. The backend may not be running with chain support enabled./)
      ).toBeInTheDocument();
    });
  });

  it('re-executes chain from result dialog', async () => {
    const executionResult = { status: 'success', data: { result: 'ok' } };
    (apiService.executeChain as any) = vi.fn().mockResolvedValue(executionResult);
    render(<ChainsPage />);

    await waitFor(() => {
      fireEvent.click(screen.getAllByText('Execute')[0]);
    });

    await waitFor(() => {
      const executeAgainButton = screen.getByText('Execute Again');
      fireEvent.click(executeAgainButton);
    });

    await waitFor(() => {
      expect(apiService.executeChain).toHaveBeenCalledTimes(2);
    });
  });
});
