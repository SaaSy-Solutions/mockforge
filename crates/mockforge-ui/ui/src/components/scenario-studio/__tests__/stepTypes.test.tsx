/**
 * @vitest-environment jsdom
 *
 * Regression: Scenario Studio registered its API-call node as `apiCall` while
 * steps (and the backend's serde snake_case StepType) use `api_call`, so
 * API-call steps fell back to the default xyflow node and the properties
 * panel never opened.
 */

import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { ReactFlow, ReactFlowProvider, type Node } from '@xyflow/react';
import { scenarioNodeTypes, normalizeStepType, STEP_TYPES } from '../stepTypes';
import { FlowPropertiesPanel } from '../FlowPropertiesPanel';

const apiCallStep: Node = {
  id: 'step-1',
  type: 'api_call',
  position: { x: 0, y: 0 },
  data: { id: 'step-1', name: 'Fetch users', method: 'POST', endpoint: '/api/users' },
};

describe('Scenario Studio step types', () => {
  it('registers a custom node for every backend step_type', () => {
    for (const stepType of ['api_call', 'condition', 'delay', 'loop', 'parallel']) {
      expect(scenarioNodeTypes).toHaveProperty(stepType);
    }
    expect(Object.keys(scenarioNodeTypes).sort()).toEqual([...STEP_TYPES].sort());
  });

  it('renders an api_call step with the custom ApiCallNode', () => {
    const { container } = render(
      <div style={{ width: 800, height: 600 }}>
        <ReactFlowProvider>
          <ReactFlow nodes={[apiCallStep]} edges={[]} nodeTypes={scenarioNodeTypes} />
        </ReactFlowProvider>
      </div>
    );

    const node = container.querySelector('.react-flow__node');
    expect(node).not.toBeNull();
    expect(node).toHaveClass('react-flow__node-api_call');
    expect(node).not.toHaveClass('react-flow__node-default');
    expect(screen.getByText('/api/users')).toBeInTheDocument();
    expect(screen.getByText('POST')).toBeInTheDocument();
  });

  it('opens the API Call properties panel for an api_call step', () => {
    render(<FlowPropertiesPanel selectedNode={apiCallStep} onUpdate={vi.fn()} onClose={vi.fn()} />);
    expect(screen.getByText('API Call Properties')).toBeInTheDocument();
  });

  it('normalizes the legacy camelCase apiCall type to api_call', () => {
    expect(normalizeStepType('apiCall')).toBe('api_call');
    expect(normalizeStepType('api_call')).toBe('api_call');
    expect(normalizeStepType('delay')).toBe('delay');
  });
});
