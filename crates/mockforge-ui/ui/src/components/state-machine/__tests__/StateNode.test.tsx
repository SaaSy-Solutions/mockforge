/**
 * @vitest-environment jsdom
 */

import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { StateNode } from '../StateNode';
import { ReactFlowProvider, type NodeProps } from '@xyflow/react';

describe('StateNode', () => {
  const getDefaultProps = (): NodeProps => ({
    id: 'test-node',
    type: 'state',
    position: { x: 0, y: 0 },
    data: {
      label: 'Test State',
      state: 'test-state',
      isInitial: false,
      isFinal: false,
    },
    selected: false,
  });

  const renderWithProvider = (ui: JSX.Element) =>
    render(<ReactFlowProvider>{ui}</ReactFlowProvider>);

  it('should render state node with label', () => {
    const defaultProps = getDefaultProps();
    renderWithProvider(<StateNode {...defaultProps} />);
    expect(screen.getByText('Test State')).toBeInTheDocument();
    expect(screen.getByText('test-state')).toBeInTheDocument();
  });

  it('should show initial badge when isInitial is true', () => {
    const defaultProps = getDefaultProps();
    renderWithProvider(
      <StateNode
        {...defaultProps}
        data={{ ...defaultProps.data, isInitial: true }}
      />
    );
    expect(screen.getByText('Initial')).toBeInTheDocument();
  });

  it('should show final badge when isFinal is true', () => {
    const defaultProps = getDefaultProps();
    renderWithProvider(
      <StateNode
        {...defaultProps}
        data={{ ...defaultProps.data, isFinal: true }}
      />
    );
    expect(screen.getByText('Final')).toBeInTheDocument();
  });

  it('should enter edit mode on double click', () => {
    const defaultProps = getDefaultProps();
    renderWithProvider(<StateNode {...defaultProps} />);
    const labelElement = screen.getByText('Test State');

    fireEvent.doubleClick(labelElement);

    const input = screen.getByDisplayValue('Test State');
    expect(input).toBeInTheDocument();
  });

  it('should update label on input change', () => {
    const defaultProps = getDefaultProps();
    renderWithProvider(<StateNode {...defaultProps} />);
    const labelElement = screen.getByText('Test State');

    fireEvent.doubleClick(labelElement);

    const input = screen.getByDisplayValue('Test State');
    fireEvent.change(input, { target: { value: 'Updated State' } });

    expect(input).toHaveValue('Updated State');
  });

  it('should exit edit mode on blur', () => {
    const defaultProps = getDefaultProps();
    renderWithProvider(<StateNode {...defaultProps} />);
    const labelElement = screen.getByText('Test State');

    fireEvent.doubleClick(labelElement);

    const input = screen.getByDisplayValue('Test State');
    fireEvent.blur(input);

    expect(screen.getByText('Test State')).toBeInTheDocument();
    expect(input).not.toBeInTheDocument();
  });

  it('should exit edit mode on Enter key', () => {
    const defaultProps = getDefaultProps();
    renderWithProvider(<StateNode {...defaultProps} />);
    const labelElement = screen.getByText('Test State');

    fireEvent.doubleClick(labelElement);

    const input = screen.getByDisplayValue('Test State');
    fireEvent.keyDown(input, { key: 'Enter' });

    expect(screen.getByText('Test State')).toBeInTheDocument();
  });

  it('should apply selected styling when selected', () => {
    const defaultProps = getDefaultProps();
    const { container } = renderWithProvider(
      <StateNode {...defaultProps} selected={true} />
    );

    const nodeElement = container.querySelector('.border-blue-500');
    expect(nodeElement).toBeInTheDocument();
  });

  it('should apply initial state styling', () => {
    const defaultProps = getDefaultProps();
    const { container } = renderWithProvider(
      <StateNode
        {...defaultProps}
        data={{ ...defaultProps.data, isInitial: true }}
      />
    );

    const nodeElement = container.querySelector('.border-green-500');
    expect(nodeElement).toBeInTheDocument();
  });

  it('should apply final state styling', () => {
    const defaultProps = getDefaultProps();
    const { container } = renderWithProvider(
      <StateNode
        {...defaultProps}
        data={{ ...defaultProps.data, isFinal: true }}
      />
    );

    const nodeElement = container.querySelector('.border-purple-500');
    expect(nodeElement).toBeInTheDocument();
  });
});
