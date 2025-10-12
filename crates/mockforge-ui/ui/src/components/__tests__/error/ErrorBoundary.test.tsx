/**
 * @jest-environment jsdom
 */

import React from 'react';
import { render, screen } from '@testing-library/react';
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { ErrorBoundary } from '../../error/ErrorBoundary';
import * as errorReporting from '../../../services/errorReporting';

// Mock component that throws an error
const ThrowError = ({ shouldThrow = false }) => {
  if (shouldThrow) {
    throw new Error('Test error message');
  }
  return <div data-testid="working-component">Working Component</div>;
};

// Mock the error reporting service
vi.mock('../../../services/errorReporting', () => ({
  reportError: vi.fn(),
  clearErrors: vi.fn(),
}));

describe('ErrorBoundary', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Suppress console.error for these tests
    vi.spyOn(console, 'error').mockImplementation(() => {});
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders children when there are no errors', () => {
    render(
      <ErrorBoundary>
        <ThrowError shouldThrow={false} />
      </ErrorBoundary>
    );

    expect(screen.getByTestId('working-component')).toBeInTheDocument();
  });

  it('renders error fallback UI when child component throws', () => {
    render(
      <ErrorBoundary>
        <ThrowError shouldThrow={true} />
      </ErrorBoundary>
    );

    expect(screen.queryByTestId('working-component')).not.toBeInTheDocument();
    expect(screen.getByTestId('error-boundary-fallback')).toBeInTheDocument();
    expect(screen.getByText(/something went wrong/i)).toBeInTheDocument();
  });

  it('displays error message in development mode', () => {
    const originalEnv = process.env.NODE_ENV;
    process.env.NODE_ENV = 'development';

    render(
      <ErrorBoundary>
        <ThrowError shouldThrow={true} />
      </ErrorBoundary>
    );

    expect(screen.getByText(/test error message/i)).toBeInTheDocument();

    process.env.NODE_ENV = originalEnv;
  });

  it('shows error details in development mode', () => {
    render(
      <ErrorBoundary>
        <ThrowError shouldThrow={true} />
      </ErrorBoundary>
    );

    // In development mode (which is the test environment), error details should be visible
    expect(screen.getByText(/test error message/i)).toBeInTheDocument();
    expect(screen.getByText(/component stack/i)).toBeInTheDocument();
  });

  it('reports error to error tracking service', () => {
    render(
      <ErrorBoundary>
        <ThrowError shouldThrow={true} />
      </ErrorBoundary>
    );

    expect(errorReporting.reportError).toHaveBeenCalledTimes(1);
    expect(errorReporting.reportError).toHaveBeenCalledWith(
      expect.objectContaining({
        message: 'Test error message',
        stack: expect.any(String),
      }),
      expect.objectContaining({
        componentStack: expect.any(String),
      })
    );
  });

  it('provides retry functionality', () => {
    const { rerender } = render(
      <ErrorBoundary>
        <ThrowError shouldThrow={true} />
      </ErrorBoundary>
    );

    const retryButton = screen.getByRole('button', { name: /try again/i });
    expect(retryButton).toBeInTheDocument();

    // Click retry button
    retryButton.click();

    // Re-render with working component
    rerender(
      <ErrorBoundary>
        <ThrowError shouldThrow={false} />
      </ErrorBoundary>
    );

    expect(screen.getByTestId('working-component')).toBeInTheDocument();
    expect(screen.queryByTestId('error-boundary-fallback')).not.toBeInTheDocument();
  });

  it('handles multiple consecutive errors', () => {
    const { rerender } = render(
      <ErrorBoundary>
        <ThrowError shouldThrow={false} />
      </ErrorBoundary>
    );

    // First error
    rerender(
      <ErrorBoundary>
        <ThrowError shouldThrow={true} />
      </ErrorBoundary>
    );

    expect(screen.getByTestId('error-boundary-fallback')).toBeInTheDocument();

    // Reset and cause another error
    const retryButton = screen.getByRole('button', { name: /try again/i });
    retryButton.click();

    rerender(
      <ErrorBoundary>
        <ThrowError shouldThrow={true} />
      </ErrorBoundary>
    );

    expect(screen.getByTestId('error-boundary-fallback')).toBeInTheDocument();
    expect(errorReporting.reportError).toHaveBeenCalledTimes(2);
  });

  it('renders custom fallback component when provided', () => {
    const CustomFallback = ({ error, resetError }) => (
      <div data-testid="custom-fallback">
        <h1>Custom Error UI</h1>
        <p>{error.message}</p>
        <button onClick={resetError}>Reset</button>
      </div>
    );

    render(
      <ErrorBoundary fallback={CustomFallback}>
        <ThrowError shouldThrow={true} />
      </ErrorBoundary>
    );

    expect(screen.getByTestId('custom-fallback')).toBeInTheDocument();
    expect(screen.getByText('Custom Error UI')).toBeInTheDocument();
    expect(screen.queryByTestId('error-boundary-fallback')).not.toBeInTheDocument();
  });

  it('handles errors during error reporting gracefully', () => {
    vi.mocked(errorReporting.reportError).mockImplementation(() => {
      throw new Error('Error reporting failed');
    });

    // Should not crash even if error reporting fails
    render(
      <ErrorBoundary>
        <ThrowError shouldThrow={true} />
      </ErrorBoundary>
    );

    expect(screen.getByTestId('error-boundary-fallback')).toBeInTheDocument();
  });

  it('provides error context information', () => {
    render(
      <ErrorBoundary>
        <ThrowError shouldThrow={true} />
      </ErrorBoundary>
    );

    expect(errorReporting.reportError).toHaveBeenCalledWith(
      expect.any(Error),
      expect.objectContaining({
        componentStack: expect.stringContaining('ThrowError'),
      })
    );
  });
});
