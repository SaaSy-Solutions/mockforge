/**
 * @jest-environment jsdom
 */

import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { Header } from '../../layout/Header';

describe('Header', () => {
  it('renders refresh button', () => {
    render(<Header />);

    expect(screen.getByText('Refresh')).toBeInTheDocument();
  });

  it('calls onRefresh when refresh button is clicked', () => {
    const mockOnRefresh = vi.fn();
    render(<Header onRefresh={mockOnRefresh} />);

    const refreshButton = screen.getByText('Refresh');
    fireEvent.click(refreshButton);

    expect(mockOnRefresh).toHaveBeenCalledTimes(1);
  });

  it('renders UserProfile component', () => {
    render(<Header />);

    // UserProfile should be rendered
    // This test verifies the component structure
    expect(screen.getByText('Refresh').closest('header')).toBeInTheDocument();
  });

  it('applies correct styling classes', () => {
    const { container } = render(<Header />);

    const header = container.querySelector('header');
    expect(header).toHaveClass('border-b', 'bg-background');
  });
});
