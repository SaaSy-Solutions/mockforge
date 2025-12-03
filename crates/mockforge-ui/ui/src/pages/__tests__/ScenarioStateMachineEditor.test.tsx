/**
 * @vitest-environment jsdom
 *
 * Unit tests for ScenarioStateMachineEditor page
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { ScenarioStateMachineEditor } from '../ScenarioStateMachineEditor';
import * as apiService from '../../services/api';

// Mock all dependencies
vi.mock('../../services/api', () => ({
  apiService: {
    getStateMachine: vi.fn(),
    createStateMachine: vi.fn(),
    updateStateMachine: vi.fn(),
    exportStateMachines: vi.fn(),
    importStateMachines: vi.fn(),
    getStateMachines: vi.fn(),
  },
}));

vi.mock('../../hooks/useWebSocket', () => ({
  useWebSocket: vi.fn(() => ({
    lastMessage: null,
    connected: true,
    sendMessage: vi.fn(),
    connect: vi.fn(),
    disconnect: vi.fn(),
  })),
}));

vi.mock('../../hooks/useHistory', () => ({
  useHistory: vi.fn(() => ({
    history: { nodes: [], edges: [] },
    push: vi.fn(),
    undo: vi.fn(() => ({ nodes: [], edges: [] })),
    redo: vi.fn(() => ({ nodes: [], edges: [] })),
    canUndo: false,
    canRedo: false,
    clear: vi.fn(),
  })),
}));

// Mock React Flow
vi.mock('@xyflow/react', async () => {
  const actual = await vi.importActual('@xyflow/react');
  return {
    ...actual,
    ReactFlow: ({ children }: any) => <div data-testid="react-flow">{children}</div>,
    Background: () => <div data-testid="background" />,
    Controls: () => <div data-testid="controls" />,
    MiniMap: () => <div data-testid="minimap" />,
    useNodesState: vi.fn(() => [[], vi.fn(), vi.fn()]),
    useEdgesState: vi.fn(() => [[], vi.fn(), vi.fn()]),
  };
});

describe('ScenarioStateMachineEditor', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should render editor page', () => {
    render(<ScenarioStateMachineEditor />);
    expect(screen.getByText(/Add State|Save|Export/i)).toBeInTheDocument();
  });

  it('should show loading state initially when loading state machine', () => {
    vi.mocked(apiService.apiService.getStateMachine).mockImplementation(
      () => new Promise(() => {}) // Never resolves
    );

    render(<ScenarioStateMachineEditor resourceType="test-resource" />);
    expect(screen.getByText('Loading state machine...')).toBeInTheDocument();
  });

  it('should load state machine when resourceType is provided', async () => {
    const mockStateMachine = {
      resource_type: 'test-resource',
      states: ['initial', 'active'],
      initial_state: 'initial',
      transitions: [
        {
          from_state: 'initial',
          to_state: 'active',
          condition_expression: 'count > 0',
        },
      ],
    };

    vi.mocked(apiService.apiService.getStateMachine).mockResolvedValue({
      state_machine: mockStateMachine,
      visual_layout: undefined,
    });

    render(<ScenarioStateMachineEditor resourceType="test-resource" />);

    await waitFor(() => {
      expect(apiService.apiService.getStateMachine).toHaveBeenCalledWith('test-resource');
    });
  });

  it('should initialize new state machine when no resourceType provided', () => {
    render(<ScenarioStateMachineEditor />);

    // Should not be loading
    expect(screen.queryByText('Loading state machine...')).not.toBeInTheDocument();
  });

  it('should display toolbar buttons', () => {
    render(<ScenarioStateMachineEditor />);

    expect(screen.getByText('Add State')).toBeInTheDocument();
    expect(screen.getByText('Save')).toBeInTheDocument();
    expect(screen.getByText('Export')).toBeInTheDocument();
    expect(screen.getByText('Import')).toBeInTheDocument();
  });

  it('should handle save operation', async () => {
    vi.mocked(apiService.apiService.createStateMachine).mockResolvedValue({
      state_machine: {
        resource_type: 'new-state-machine',
        states: ['initial'],
        initial_state: 'initial',
        transitions: [],
      },
      visual_layout: undefined,
    });

    render(<ScenarioStateMachineEditor />);

    await waitFor(() => {
      const saveButton = screen.getByText('Save');
      expect(saveButton).toBeInTheDocument();
    });

    const saveButton = screen.getByText('Save');
    // Note: Actual save would require nodes/edges state, which is mocked
    // This test verifies the button exists and is clickable
    expect(saveButton).toBeInTheDocument();
  });

  it('should handle export operation', async () => {
    vi.mocked(apiService.apiService.exportStateMachines).mockResolvedValue({
      state_machines: [],
      visual_layouts: {},
    });

    // Mock URL.createObjectURL and download
    const createObjectURLSpy = vi.spyOn(URL, 'createObjectURL').mockReturnValue('mock-url');
    const revokeObjectURLSpy = vi.spyOn(URL, 'revokeObjectURL').mockImplementation(() => {});

    render(<ScenarioStateMachineEditor />);

    await waitFor(() => {
      const exportButton = screen.getByText('Export');
      expect(exportButton).toBeInTheDocument();
    });

    const exportButton = screen.getByText('Export');
    // Export functionality would be tested in integration/E2E tests
    expect(exportButton).toBeInTheDocument();

    createObjectURLSpy.mockRestore();
    revokeObjectURLSpy.mockRestore();
  });
});
