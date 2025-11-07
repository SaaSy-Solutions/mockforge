/**
 * @vitest-environment jsdom
 */

import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { ConditionBuilder } from '../ConditionBuilder';

describe('ConditionBuilder', () => {
  const defaultProps = {
    condition: 'count > 10',
    onUpdate: vi.fn(),
    onCancel: vi.fn(),
  };

  it('should render condition builder', () => {
    render(<ConditionBuilder {...defaultProps} />);
    expect(screen.getByText('Code')).toBeInTheDocument();
    expect(screen.getByText('Visual')).toBeInTheDocument();
  });

  it('should display initial condition in code mode', () => {
    render(<ConditionBuilder {...defaultProps} />);
    const input = screen.getByDisplayValue('count > 10');
    expect(input).toBeInTheDocument();
  });

  it('should update condition in code mode', () => {
    render(<ConditionBuilder {...defaultProps} />);
    const input = screen.getByDisplayValue('count > 10');

    fireEvent.change(input, { target: { value: 'status == "active"' } });
    expect(input).toHaveValue('status == "active"');
  });

  it('should call onUpdate when Apply is clicked in code mode', () => {
    render(<ConditionBuilder {...defaultProps} />);
    const input = screen.getByDisplayValue('count > 10');

    fireEvent.change(input, { target: { value: 'status == "active"' } });
    fireEvent.click(screen.getByText('Apply'));

    expect(defaultProps.onUpdate).toHaveBeenCalledWith('status == "active"');
  });

  it('should call onCancel when Cancel is clicked', () => {
    render(<ConditionBuilder {...defaultProps} />);
    fireEvent.click(screen.getByText('Cancel'));

    expect(defaultProps.onCancel).toHaveBeenCalled();
  });

  it('should switch to visual mode', () => {
    render(<ConditionBuilder {...defaultProps} />);
    fireEvent.click(screen.getByText('Visual'));

    expect(screen.getByText('Add Condition')).toBeInTheDocument();
  });

  it('should add visual condition', () => {
    render(<ConditionBuilder {...defaultProps} />);
    fireEvent.click(screen.getByText('Visual'));
    fireEvent.click(screen.getByText('Add Condition'));

    const variableInputs = screen.getAllByPlaceholderText('variable');
    expect(variableInputs.length).toBeGreaterThan(1);
  });

  it('should remove visual condition', async () => {
    render(<ConditionBuilder {...defaultProps} />);
    fireEvent.click(screen.getByText('Visual'));
    fireEvent.click(screen.getByText('Add Condition'));

    await waitFor(() => {
      const removeButtons = screen.getAllByRole('button');
      const trashButtons = removeButtons.filter(btn =>
        btn.querySelector('svg') && btn.querySelector('svg')?.getAttribute('data-lucide') === 'trash-2'
      );

      if (trashButtons.length > 0) {
        fireEvent.click(trashButtons[trashButtons.length - 1]);
      }
    });

    await waitFor(() => {
      const variableInputs = screen.getAllByPlaceholderText('variable');
      expect(variableInputs.length).toBe(1);
    });
  });

  it('should update visual condition fields', () => {
    render(<ConditionBuilder {...defaultProps} />);
    fireEvent.click(screen.getByText('Visual'));

    const variableInput = screen.getByPlaceholderText('variable');
    fireEvent.change(variableInput, { target: { value: 'count' } });

    expect(variableInput).toHaveValue('count');
  });

  it('should convert visual conditions to expression', () => {
    render(<ConditionBuilder {...defaultProps} />);
    fireEvent.click(screen.getByText('Visual'));

    const variableInput = screen.getByPlaceholderText('variable');
    const valueInput = screen.getByPlaceholderText('value');

    fireEvent.change(variableInput, { target: { value: 'count' } });
    fireEvent.change(valueInput, { target: { value: '10' } });

    fireEvent.click(screen.getByText('Apply'));

    expect(defaultProps.onUpdate).toHaveBeenCalled();
  });
});
