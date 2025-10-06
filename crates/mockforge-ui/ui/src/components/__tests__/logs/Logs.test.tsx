/**
 * @jest-environment jsdom
 */

import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import { LogEntry } from '../../logs/LogEntry';

const mockLog = {
  id: '1',
  timestamp: '2024-01-01T12:00:00Z',
  method: 'GET',
  path: '/api/users',
  statusCode: 200,
  duration: 45,
  size: 1024,
};

describe('LogEntry', () => {
  it('renders log information', () => {
    render(<LogEntry log={mockLog} />);

    expect(screen.getByText('GET')).toBeInTheDocument();
    expect(screen.getByText('/api/users')).toBeInTheDocument();
  });

  it('applies correct status color for 2xx codes', () => {
    const { container } = render(<LogEntry log={mockLog} />);

    const statusElement = container.querySelector('.text-green-600');
    expect(statusElement).toBeInTheDocument();
  });

  it('applies correct status color for 4xx codes', () => {
    const log404 = { ...mockLog, statusCode: 404 };
    const { container } = render(<LogEntry log={log404} />);

    const statusElement = container.querySelector('.text-yellow-600');
    expect(statusElement).toBeInTheDocument();
  });

  it('applies correct status color for 5xx codes', () => {
    const log500 = { ...mockLog, statusCode: 500 };
    const { container } = render(<LogEntry log={log500} />);

    const statusElement = container.querySelector('.text-red-600');
    expect(statusElement).toBeInTheDocument();
  });

  it('applies correct method color for GET requests', () => {
    const { container } = render(<LogEntry log={mockLog} />);

    const methodElement = container.querySelector('.text-green-700');
    expect(methodElement).toBeInTheDocument();
  });

  it('applies correct method color for POST requests', () => {
    const postLog = { ...mockLog, method: 'POST' };
    const { container } = render(<LogEntry log={postLog} />);

    const methodElement = container.querySelector('.text-blue-700');
    expect(methodElement).toBeInTheDocument();
  });

  it('applies correct method color for DELETE requests', () => {
    const deleteLog = { ...mockLog, method: 'DELETE' };
    const { container } = render(<LogEntry log={deleteLog} />);

    const methodElement = container.querySelector('.text-red-700');
    expect(methodElement).toBeInTheDocument();
  });

  it('highlights selected log entry', () => {
    const { container } = render(<LogEntry log={mockLog} isSelected={true} />);

    const logEntry = container.firstChild;
    expect(logEntry).toHaveClass('bg-accent');
  });

  it('calls onSelect when clicked', () => {
    const mockOnSelect = vi.fn();
    render(<LogEntry log={mockLog} onSelect={mockOnSelect} />);

    const logEntry = screen.getByText('GET').closest('div');
    fireEvent.click(logEntry!);

    expect(mockOnSelect).toHaveBeenCalledWith(mockLog);
  });

  it('formats timestamp correctly', () => {
    render(<LogEntry log={mockLog} />);

    // Should display time in locale format
    const timeRegex = /\d{1,2}:\d{2}:\d{2}/;
    expect(screen.getByText(timeRegex)).toBeInTheDocument();
  });
});
