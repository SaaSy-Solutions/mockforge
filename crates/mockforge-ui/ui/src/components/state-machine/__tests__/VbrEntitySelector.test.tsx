/**
 * @vitest-environment jsdom
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { VbrEntitySelector } from '../VbrEntitySelector';
import * as apiService from '../../../services/api';

// Mock the API service
vi.mock('../../../services/api', () => ({
  apiService: {
    getStateMachines: vi.fn(),
  },
}));

// Mock fetch
global.fetch = vi.fn();

describe('VbrEntitySelector', () => {
  const defaultProps = {
    selectedEntity: undefined,
    onSelect: vi.fn(),
    onClose: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(global.fetch).mockResolvedValue({
      ok: false,
    } as Response);
  });

  it('should render VBR entity selector', () => {
    vi.mocked(apiService.apiService.getStateMachines).mockResolvedValue({
      state_machines: [],
      total: 0,
    });

    render(<VbrEntitySelector {...defaultProps} />);
    expect(screen.getByText('Select VBR Entity')).toBeInTheDocument();
  });

  it('should show loading state initially', () => {
    vi.mocked(apiService.apiService.getStateMachines).mockImplementation(
      () => new Promise(() => {}) // Never resolves
    );

    render(<VbrEntitySelector {...defaultProps} />);
    expect(screen.getByText('Loading entities...')).toBeInTheDocument();
  });

  it('should display entities from state machines', async () => {
    vi.mocked(apiService.apiService.getStateMachines).mockResolvedValue({
      state_machines: [
        {
          resource_type: 'User',
          state_count: 3,
          transition_count: 5,
          sub_scenario_count: 0,
          has_visual_layout: true,
        },
      ],
      total: 1,
    });

    render(<VbrEntitySelector {...defaultProps} />);

    await waitFor(() => {
      expect(screen.getByText('User')).toBeInTheDocument();
    });
  });

  it('should filter entities by search query', async () => {
    vi.mocked(apiService.apiService.getStateMachines).mockResolvedValue({
      state_machines: [
        {
          resource_type: 'User',
          state_count: 3,
          transition_count: 5,
          sub_scenario_count: 0,
          has_visual_layout: true,
        },
        {
          resource_type: 'Order',
          state_count: 4,
          transition_count: 6,
          sub_scenario_count: 0,
          has_visual_layout: true,
        },
      ],
      total: 2,
    });

    render(<VbrEntitySelector {...defaultProps} />);

    await waitFor(() => {
      expect(screen.getByText('User')).toBeInTheDocument();
    });

    const searchInput = screen.getByPlaceholderText('Search entities...');
    fireEvent.change(searchInput, { target: { value: 'Order' } });

    await waitFor(() => {
      expect(screen.queryByText('User')).not.toBeInTheDocument();
      expect(screen.getByText('Order')).toBeInTheDocument();
    });
  });

  it('should call onSelect when entity is selected', async () => {
    vi.mocked(apiService.apiService.getStateMachines).mockResolvedValue({
      state_machines: [
        {
          resource_type: 'User',
          state_count: 3,
          transition_count: 5,
          sub_scenario_count: 0,
          has_visual_layout: true,
        },
      ],
      total: 1,
    });

    render(<VbrEntitySelector {...defaultProps} />);

    await waitFor(() => {
      expect(screen.getByText('User')).toBeInTheDocument();
    });

    const entityCard = screen.getByText('User').closest('div[class*="cursor-pointer"]');
    if (entityCard) {
      fireEvent.click(entityCard);
    }

    // Click Select button
    const selectButton = screen.getByText('Select');
    fireEvent.click(selectButton);

    expect(defaultProps.onSelect).toHaveBeenCalled();
  });

  it('should show badge for entities with state machines', async () => {
    vi.mocked(apiService.apiService.getStateMachines).mockResolvedValue({
      state_machines: [
        {
          resource_type: 'User',
          state_count: 3,
          transition_count: 5,
          sub_scenario_count: 0,
          has_visual_layout: true,
        },
      ],
      total: 1,
    });

    render(<VbrEntitySelector {...defaultProps} />);

    await waitFor(() => {
      expect(screen.getByText('Has State Machine')).toBeInTheDocument();
    });
  });

  it('should call onClose when cancel is clicked', () => {
    vi.mocked(apiService.apiService.getStateMachines).mockResolvedValue({
      state_machines: [],
      total: 0,
    });

    render(<VbrEntitySelector {...defaultProps} />);

    const cancelButton = screen.getByText('Cancel');
    fireEvent.click(cancelButton);

    expect(defaultProps.onClose).toHaveBeenCalled();
  });
});
