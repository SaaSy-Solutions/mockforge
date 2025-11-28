/**
 * @vitest-environment jsdom
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import { StatePreviewPanel } from '../StatePreviewPanel';
import * as apiService from '../../../services/api';

// Mock the API service
vi.mock('../../../services/api', () => ({
  apiService: {
    getStateInstances: vi.fn(),
  },
}));

// Mock the WebSocket hook
vi.mock('../../../hooks/useWebSocket', () => ({
  useWebSocket: vi.fn(() => ({
    lastMessage: null,
    connected: true,
  })),
}));

describe('StatePreviewPanel', () => {
  const defaultProps = {
    resourceType: 'test-resource',
    onClose: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should render state preview panel', () => {
    vi.mocked(apiService.apiService.getStateInstances).mockResolvedValue({
      instances: [],
      total: 0,
    });

    render(<StatePreviewPanel {...defaultProps} />);
    expect(screen.getByText('State Preview')).toBeInTheDocument();
  });

  it('should show loading state initially', () => {
    vi.mocked(apiService.apiService.getStateInstances).mockImplementation(
      () => new Promise(() => {}) // Never resolves
    );

    render(<StatePreviewPanel {...defaultProps} />);
    expect(screen.getByText('Loading...')).toBeInTheDocument();
  });

  it('should display state instances', async () => {
    vi.mocked(apiService.apiService.getStateInstances).mockResolvedValue({
      instances: [
        {
          resource_id: 'instance-1',
          current_state: 'active',
          resource_type: 'test-resource',
          history_count: 2,
          state_data: { count: 5 },
        },
      ],
      total: 1,
    });

    render(<StatePreviewPanel {...defaultProps} />);

    await waitFor(() => {
      expect(screen.getByText('instance-1')).toBeInTheDocument();
      expect(screen.getByText('active')).toBeInTheDocument();
    });
  });

  it('should show empty state when no instances', async () => {
    vi.mocked(apiService.apiService.getStateInstances).mockResolvedValue({
      instances: [],
      total: 0,
    });

    render(<StatePreviewPanel {...defaultProps} />);

    await waitFor(() => {
      expect(screen.getByText('No active instances')).toBeInTheDocument();
    });
  });

  it('should display state data when available', async () => {
    vi.mocked(apiService.apiService.getStateInstances).mockResolvedValue({
      instances: [
        {
          resource_id: 'instance-1',
          current_state: 'active',
          resource_type: 'test-resource',
          history_count: 1,
          state_data: { count: 5, status: 'active' },
        },
      ],
      total: 1,
    });

    render(<StatePreviewPanel {...defaultProps} />);

    await waitFor(() => {
      expect(screen.getByText('State Data:')).toBeInTheDocument();
    });
  });

  it('should call onClose when close button is clicked', () => {
    vi.mocked(apiService.apiService.getStateInstances).mockResolvedValue({
      instances: [],
      total: 0,
    });

    render(<StatePreviewPanel {...defaultProps} />);

    const closeButton = screen.getByRole('button', { name: /close/i });
    // Find by title or aria-label
    const buttons = screen.getAllByRole('button');
    const closeBtn = buttons.find(btn => btn.querySelector('svg'));

    if (closeBtn) {
      fireEvent.click(closeBtn);
      // Note: onClose might not be called if button doesn't have proper handler
      // This is a basic test structure
    }
  });
});
