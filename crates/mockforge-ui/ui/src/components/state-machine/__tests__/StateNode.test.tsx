/**
 * @vitest-environment jsdom
 */

import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { StateNode } from '../StateNode';
import type { NodeProps } from 'react-flow-renderer';

describe('StateNode', () => {
  const defaultProps: NodeProps = {
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
  };

  it('should render state node with label', () => {
    render(<StateNode {...defaultProps} />);
    expect(screen.getByText('Test State')).toBeInTheDocument();
    expect(screen.getByText('test-state')).toBeInTheDocument();
  });

  it('should show initial badge when isInitial is true', () => {
    render(
      <StateNode
        {...defaultProps}
        data={{ ...defaultProps.data, isInitial: true }}
      />
    );
    expect(screen.getByText('Initial')).toBeInTheDocument();
  });

  it('should show final badge when isFinal is true', () => {
    render(
      <StateNode
        {...defaultProps}
        data={{ ...defaultProps.data, isFinal: true }}
      />
    );
    expect(screen.getByText('Final')).toBeInTheDocument();
  });

  it('should enter edit mode on double click', () => {
    render(<StateNode {...defaultProps} />);
    const labelElement = screen.getByText('Test State');

    fireEvent.doubleClick(labelElement);

    const input = screen.getByDisplayValue('Test State');
    expect(input).toBeInTheDocument();
  });

  it('should update label on input change', () => {
    render(<StateNode {...defaultProps} />);
    const labelElement = screen.getByText('Test State');

    fireEvent.doubleClick(labelElement);

    const input = screen.getByDisplayValue('Test State');
    fireEvent.change(input, { target: { value: 'Updated State' } });

    expect(input).toHaveValue('Updated State');
  });

  it('should exit edit mode on blur', () => {
    render(<StateNode {...defaultProps} />);
    const labelElement = screen.getByText('Test State');

    fireEvent.doubleClick(labelElement);

    const input = screen.getByDisplayValue('Test State');
    fireEvent.blur(input);

    expect(screen.getByText('Test State')).toBeInTheDocument();
    expect(input).not.toBeInTheDocument();
  });

  it('should exit edit mode on Enter key', () => {
    render(<StateNode {...defaultProps} />);
    const labelElement = screen.getByText('Test State');

    fireEvent.doubleClick(labelElement);

    const input = screen.getByDisplayValue('Test State');
    fireEvent.keyDown(input, { key: 'Enter' });

    expect(screen.getByText('Test State')).toBeInTheDocument();
  });

  it('should apply selected styling when selected', () => {
    const { container } = render(
      <StateNode {...defaultProps} selected={true} />
    );

    const nodeElement = container.querySelector('.border-blue-500');
    expect(nodeElement).toBeInTheDocument();
  });

  it('should apply initial state styling', () => {
    const { container } = render(
      <StateNode
        {...defaultProps}
        data={{ ...defaultProps.data, isInitial: true }}
      />
    );

    const nodeElement = container.querySelector('.border-green-500');
    expect(nodeElement).toBeInTheDocument();
  });

  it('should apply final state styling', () => {
    const { container } = render(
      <StateNode
        {...defaultProps}
        data={{ ...defaultProps.data, isFinal: true }}
      />
    );

    const nodeElement = container.querySelector('.border-purple-500');
    expect(nodeElement).toBeInTheDocument();
  });
});
