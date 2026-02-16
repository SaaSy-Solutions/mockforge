/**
 * @vitest-environment jsdom
 *
 * Integration tests for state machine components working together
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { ConditionBuilder } from '../ConditionBuilder';
import { VbrEntitySelector } from '../VbrEntitySelector';
import { SubScenarioEditor } from '../SubScenarioEditor';
import * as apiService from '../../../services/api';

// Mock API service
vi.mock('../../../services/api', () => ({
  apiService: {
    getStateMachines: vi.fn(),
    getStateInstances: vi.fn(),
  },
}));

// Mock WebSocket hook
vi.mock('../../../hooks/useWebSocket', () => ({
  useWebSocket: vi.fn(() => ({
    lastMessage: null,
    connected: true,
    sendMessage: vi.fn(),
    connect: vi.fn(),
    disconnect: vi.fn(),
  })),
}));

describe('State Machine Component Integration', () => {
  beforeEach(() => {
    vi.clearAllMocks();
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
  });

  describe('ConditionBuilder and State Machine Flow', () => {
    it('should update condition and trigger callback', async () => {
      const onUpdate = vi.fn();
      const onCancel = vi.fn();

      render(
        <ConditionBuilder
          condition="count > 10"
          onUpdate={onUpdate}
          onCancel={onCancel}
        />
      );

      const input = screen.getByDisplayValue('count > 10');
      fireEvent.change(input, { target: { value: 'status == "active"' } });
      fireEvent.click(screen.getByText('Apply'));

      expect(onUpdate).toHaveBeenCalledWith('status == "active"');
    });

    it('should build visual condition and convert to expression', async () => {
      const onUpdate = vi.fn();

      render(
        <ConditionBuilder
          condition=""
          onUpdate={onUpdate}
          onCancel={vi.fn()}
        />
      );

      fireEvent.click(screen.getByText('Visual'));
      fireEvent.click(screen.getByText('Add Condition'));

      await waitFor(() => {
        const variableInputs = screen.getAllByPlaceholderText('variable');
        expect(variableInputs.length).toBeGreaterThan(0);
      });

      const variableInput = screen.getAllByPlaceholderText('variable')[0];
      const valueInput = screen.getAllByPlaceholderText('value')[0];

      fireEvent.change(variableInput, { target: { value: 'count' } });
      fireEvent.change(valueInput, { target: { value: '10' } });

      fireEvent.click(screen.getByText('Apply'));

      expect(onUpdate).toHaveBeenCalled();
    });
  });

  describe('VbrEntitySelector Integration', () => {
    it('should load and display entities from state machines', async () => {
      const onSelect = vi.fn();
      const onClose = vi.fn();

      render(
        <VbrEntitySelector
          selectedEntity={undefined}
          onSelect={onSelect}
          onClose={onClose}
        />
      );

      await waitFor(() => {
        expect(screen.getByText('User')).toBeInTheDocument();
        expect(screen.getByText('Order')).toBeInTheDocument();
      });
    });

    it('should filter entities by search query', async () => {
      render(
        <VbrEntitySelector
          selectedEntity={undefined}
          onSelect={vi.fn()}
          onClose={vi.fn()}
        />
      );

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

    it('should select entity and call onSelect', async () => {
      const onSelect = vi.fn();

      render(
        <VbrEntitySelector
          selectedEntity={undefined}
          onSelect={onSelect}
          onClose={vi.fn()}
        />
      );

      await waitFor(() => {
        expect(screen.getByText('User')).toBeInTheDocument();
      });

      const entityCard = screen.getByText('User').closest('div[class*="cursor-pointer"]');
      if (entityCard) {
        fireEvent.click(entityCard);
      }

      const selectButton = screen.getByText('Select');
      fireEvent.click(selectButton);

      expect(onSelect).toHaveBeenCalled();
    });
  });

  describe('SubScenarioEditor Integration', () => {
    it('should load available state machines for sub-scenario selection', async () => {
      render(
        <SubScenarioEditor
          subScenarioId={undefined}
          onSave={vi.fn()}
          onCancel={vi.fn()}
        />
      );

      await waitFor(() => {
        expect(screen.getByText('Select a state machine...')).toBeInTheDocument();
      });

      const select = screen.getByRole('combobox');
      expect(select).toBeInTheDocument();
    });

    it('should create sub-scenario with input/output mappings', async () => {
      const onSave = vi.fn();

      render(
        <SubScenarioEditor
          subScenarioId={undefined}
          onSave={onSave}
          onCancel={vi.fn()}
        />
      );

      await waitFor(() => {
        expect(screen.getByPlaceholderText('Sub-scenario name')).toBeInTheDocument();
      });

      const nameInput = screen.getByPlaceholderText('Sub-scenario name');
      fireEvent.change(nameInput, { target: { value: 'Test Sub-Scenario' } });

      const select = screen.getByRole('combobox');
      fireEvent.change(select, { target: { value: 'User' } });

      // Add input mapping
      const addButtons = screen.getAllByText('Add');
      fireEvent.click(addButtons[0]);

      const parentVarInput = screen.getAllByPlaceholderText('Parent variable')[0];
      const subVarInput = screen.getAllByPlaceholderText('Sub-scenario variable')[0];

      fireEvent.change(parentVarInput, { target: { value: 'parentVar' } });
      fireEvent.change(subVarInput, { target: { value: 'subVar' } });

      const saveButton = screen.getByText('Save');
      fireEvent.click(saveButton);

      await waitFor(() => {
        expect(onSave).toHaveBeenCalled();
        const savedConfig = onSave.mock.calls[0][0];
        expect(savedConfig.name).toBe('Test Sub-Scenario');
        expect(savedConfig.state_machine_resource_type).toBe('User');
      });
    });
  });

  describe('Component State Management', () => {
    it('should handle condition builder cancel without updating', () => {
      const onUpdate = vi.fn();
      const onCancel = vi.fn();

      render(
        <ConditionBuilder
          condition="count > 10"
          onUpdate={onUpdate}
          onCancel={onCancel}
        />
      );

      const input = screen.getByDisplayValue('count > 10');
      fireEvent.change(input, { target: { value: 'modified' } });
      fireEvent.click(screen.getByText('Cancel'));

      expect(onCancel).toHaveBeenCalled();
      expect(onUpdate).not.toHaveBeenCalled();
    });

    it('should maintain state when switching between code and visual modes', () => {
      render(
        <ConditionBuilder
          condition="count > 10"
          onUpdate={vi.fn()}
          onCancel={vi.fn()}
        />
      );

      // Start in code mode
      expect(screen.getByDisplayValue('count > 10')).toBeInTheDocument();

      // Switch to visual
      fireEvent.click(screen.getByText('Visual'));
      expect(screen.getByText('Add Condition')).toBeInTheDocument();

      // Switch back to code
      fireEvent.click(screen.getByText('Code'));
      expect(screen.getByDisplayValue('count > 10')).toBeInTheDocument();
    });
  });
});
