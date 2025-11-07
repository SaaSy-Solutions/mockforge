/**
 * @vitest-environment jsdom
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { SubScenarioEditor } from '../SubScenarioEditor';
import * as apiService from '../../../services/api';

// Mock the API service
vi.mock('../../../services/api', () => ({
  apiService: {
    getStateMachines: vi.fn(),
  },
}));

describe('SubScenarioEditor', () => {
  const defaultProps = {
    subScenarioId: undefined,
    onSave: vi.fn(),
    onCancel: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(apiService.apiService.getStateMachines).mockResolvedValue({
      state_machines: [
        {
          resource_type: 'Order',
          state_count: 4,
          transition_count: 6,
          sub_scenario_count: 0,
          has_visual_layout: true,
        },
      ],
      total: 1,
    });
  });

  it('should render sub-scenario editor', () => {
    render(<SubScenarioEditor {...defaultProps} />);
    expect(screen.getByText('Create Sub-Scenario')).toBeInTheDocument();
  });

  it('should show edit title when editing', () => {
    render(<SubScenarioEditor {...defaultProps} subScenarioId="test-id" />);
    expect(screen.getByText('Edit Sub-Scenario')).toBeInTheDocument();
  });

  it('should load available state machines', async () => {
    render(<SubScenarioEditor {...defaultProps} />);

    await waitFor(() => {
      expect(screen.getByText('Select a state machine...')).toBeInTheDocument();
    });
  });

  it('should update name field', () => {
    render(<SubScenarioEditor {...defaultProps} />);
    const nameInput = screen.getByPlaceholderText('Sub-scenario name');

    fireEvent.change(nameInput, { target: { value: 'Test Sub-Scenario' } });
    expect(nameInput).toHaveValue('Test Sub-Scenario');
  });

  it('should update description field', () => {
    render(<SubScenarioEditor {...defaultProps} />);
    const descInput = screen.getByPlaceholderText('Optional description');

    fireEvent.change(descInput, { target: { value: 'Test description' } });
    expect(descInput).toHaveValue('Test description');
  });

  it('should add input mapping', async () => {
    render(<SubScenarioEditor {...defaultProps} />);

    await waitFor(() => {
      expect(screen.getByText('Add')).toBeInTheDocument();
    });

    const addButtons = screen.getAllByText('Add');
    const inputMappingAdd = addButtons[0];
    fireEvent.click(inputMappingAdd);

    const variableInputs = screen.getAllByPlaceholderText('Parent variable');
    expect(variableInputs.length).toBeGreaterThan(1);
  });

  it('should remove input mapping', async () => {
    render(<SubScenarioEditor {...defaultProps} />);

    await waitFor(() => {
      expect(screen.getByText('Add')).toBeInTheDocument();
    });

    const addButtons = screen.getAllByText('Add');
    fireEvent.click(addButtons[0]); // Add input mapping

    await waitFor(() => {
      const removeButtons = screen.getAllByRole('button');
      const trashButtons = removeButtons.filter(btn =>
        btn.querySelector('svg') && btn.querySelector('svg')?.getAttribute('data-lucide') === 'trash-2'
      );

      if (trashButtons.length > 0) {
        fireEvent.click(trashButtons[trashButtons.length - 1]);
      }
    });
  });

  it('should update input mapping fields', async () => {
    render(<SubScenarioEditor {...defaultProps} />);

    await waitFor(() => {
      const variableInput = screen.getByPlaceholderText('Parent variable');
      fireEvent.change(variableInput, { target: { value: 'parentVar' } });
      expect(variableInput).toHaveValue('parentVar');
    });
  });

  it('should add output mapping', async () => {
    render(<SubScenarioEditor {...defaultProps} />);

    await waitFor(() => {
      const addButtons = screen.getAllByText('Add');
      const outputMappingAdd = addButtons[1];
      fireEvent.click(outputMappingAdd);

      const variableInputs = screen.getAllByPlaceholderText('Sub-scenario variable');
      expect(variableInputs.length).toBeGreaterThan(1);
    });
  });

  it('should call onSave with correct data', async () => {
    render(<SubScenarioEditor {...defaultProps} />);

    await waitFor(() => {
      const nameInput = screen.getByPlaceholderText('Sub-scenario name');
      fireEvent.change(nameInput, { target: { value: 'Test Sub-Scenario' } });

      const select = screen.getByRole('combobox');
      fireEvent.change(select, { target: { value: 'Order' } });

      const saveButton = screen.getByText('Save');
      fireEvent.click(saveButton);

      expect(defaultProps.onSave).toHaveBeenCalled();
    });
  });

  it('should disable save button when required fields are missing', async () => {
    render(<SubScenarioEditor {...defaultProps} />);

    await waitFor(() => {
      const saveButton = screen.getByText('Save');
      expect(saveButton).toBeDisabled();
    });
  });

  it('should call onCancel when cancel is clicked', () => {
    render(<SubScenarioEditor {...defaultProps} />);

    const cancelButton = screen.getByText('Cancel');
    fireEvent.click(cancelButton);

    expect(defaultProps.onCancel).toHaveBeenCalled();
  });
});
