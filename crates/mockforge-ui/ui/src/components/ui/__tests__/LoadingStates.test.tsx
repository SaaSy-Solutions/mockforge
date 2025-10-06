/**
 * @jest-environment jsdom
 */

import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import {
  Spinner,
  LoadingState,
  EmptyState,
  ErrorState,
  SuccessState,
  DashboardLoading,
  TableLoading,
} from '../LoadingStates';

describe('Spinner', () => {
  it('renders with default size', () => {
    const { container } = render(<Spinner />);
    expect(container.querySelector('.h-6')).toBeInTheDocument();
  });

  it('renders with small size', () => {
    const { container } = render(<Spinner size="sm" />);
    expect(container.querySelector('.h-4')).toBeInTheDocument();
  });

  it('renders with large size', () => {
    const { container } = render(<Spinner size="lg" />);
    expect(container.querySelector('.h-8')).toBeInTheDocument();
  });

  it('has loading role and aria-label', () => {
    render(<Spinner />);
    const spinner = screen.getByRole('status');
    expect(spinner).toHaveAttribute('aria-label', 'Loading');
  });

  it('applies custom className', () => {
    const { container } = render(<Spinner className="custom-spinner" />);
    expect(container.firstChild).toHaveClass('custom-spinner');
  });

  it('applies correct color variants', () => {
    const { container, rerender } = render(<Spinner color="primary" />);
    expect(container.firstChild).toHaveClass('text-primary');

    rerender(<Spinner color="brand" />);
    expect(container.firstChild).toHaveClass('text-brand');

    rerender(<Spinner color="muted" />);
    expect(container.firstChild).toHaveClass('text-secondary');
  });
});

describe('LoadingState', () => {
  it('renders with default props', () => {
    render(<LoadingState />);
    expect(screen.getByText('Loading...')).toBeInTheDocument();
  });

  it('renders custom title and description', () => {
    render(<LoadingState title="Loading data" description="Please wait..." />);
    expect(screen.getByText('Loading data')).toBeInTheDocument();
    expect(screen.getByText('Please wait...')).toBeInTheDocument();
  });

  it('renders spinner variant', () => {
    render(<LoadingState variant="spinner" />);
    expect(screen.getByRole('status')).toBeInTheDocument();
  });

  it('renders skeleton variant', () => {
    const { container } = render(<LoadingState variant="skeleton" />);
    expect(container.querySelector('.animate-pulse')).toBeInTheDocument();
  });

  it('renders pulse variant', () => {
    render(<LoadingState variant="pulse" title="Processing" />);
    expect(screen.getByText('Processing')).toBeInTheDocument();
    expect(document.querySelector('.pulse-subtle')).toBeInTheDocument();
  });

  it('applies size variants', () => {
    const { container } = render(<LoadingState size="lg" />);
    expect(container.firstChild).toHaveClass('py-16');
  });
});

describe('EmptyState', () => {
  it('renders with required title', () => {
    render(<EmptyState title="No data found" />);
    expect(screen.getByText('No data found')).toBeInTheDocument();
  });

  it('renders description when provided', () => {
    render(<EmptyState title="No data" description="Try adjusting your filters" />);
    expect(screen.getByText('Try adjusting your filters')).toBeInTheDocument();
  });

  it('renders action button when provided', () => {
    const onClick = vi.fn();
    render(
      <EmptyState
        title="No data"
        action={{ label: 'Create New', onClick }}
      />
    );

    const button = screen.getByText('Create New');
    fireEvent.click(button);
    expect(onClick).toHaveBeenCalled();
  });

  it('renders custom icon', () => {
    const CustomIcon = <div data-testid="custom-icon">Custom</div>;
    render(<EmptyState title="No data" icon={CustomIcon} />);
    expect(screen.getByTestId('custom-icon')).toBeInTheDocument();
  });

  it('renders default icon when not provided', () => {
    const { container } = render(<EmptyState title="No data" />);
    expect(container.querySelector('svg')).toBeInTheDocument();
  });

  it('applies action button variant', () => {
    render(
      <EmptyState
        title="No data"
        action={{ label: 'Create', onClick: vi.fn(), variant: 'secondary' }}
      />
    );
    const button = screen.getByText('Create');
    expect(button).toHaveClass('variant-secondary');
  });
});

describe('ErrorState', () => {
  it('renders with default error message', () => {
    render(<ErrorState />);
    expect(screen.getByText('Something went wrong')).toBeInTheDocument();
  });

  it('renders custom title and description', () => {
    render(<ErrorState title="Error occurred" description="Check your connection" />);
    expect(screen.getByText('Error occurred')).toBeInTheDocument();
    expect(screen.getByText('Check your connection')).toBeInTheDocument();
  });

  it('displays error message from Error object', () => {
    const error = new Error('Network timeout');
    render(<ErrorState error={error} />);
    expect(screen.getByText('Network timeout')).toBeInTheDocument();
  });

  it('displays error message from string', () => {
    render(<ErrorState error="Invalid request" />);
    expect(screen.getByText('Invalid request')).toBeInTheDocument();
  });

  it('renders retry button', () => {
    const retry = vi.fn();
    render(<ErrorState retry={retry} />);

    const retryButton = screen.getByText('Try Again');
    fireEvent.click(retryButton);
    expect(retry).toHaveBeenCalled();
  });

  it('does not render retry button when not provided', () => {
    render(<ErrorState />);
    expect(screen.queryByText('Try Again')).not.toBeInTheDocument();
  });
});

describe('SuccessState', () => {
  it('renders with title', () => {
    render(<SuccessState title="Operation completed" />);
    expect(screen.getByText('Operation completed')).toBeInTheDocument();
  });

  it('renders description', () => {
    render(<SuccessState title="Success" description="Your changes have been saved" />);
    expect(screen.getByText('Your changes have been saved')).toBeInTheDocument();
  });

  it('renders action button', () => {
    const onClick = vi.fn();
    render(
      <SuccessState
        title="Success"
        action={{ label: 'Continue', onClick }}
      />
    );

    const button = screen.getByText('Continue');
    fireEvent.click(button);
    expect(onClick).toHaveBeenCalled();
  });

  it('does not render action when not provided', () => {
    render(<SuccessState title="Success" />);
    expect(screen.queryByRole('button')).not.toBeInTheDocument();
  });
});

describe('DashboardLoading', () => {
  it('renders skeleton metric cards', () => {
    const { container } = render(<DashboardLoading />);
    const skeletons = container.querySelectorAll('.animate-pulse');
    expect(skeletons.length).toBeGreaterThan(0);
  });

  it('renders with stagger animation delays', () => {
    const { container } = render(<DashboardLoading />);
    expect(container.querySelector('.animate-delay-75')).toBeInTheDocument();
    expect(container.querySelector('.animate-delay-150')).toBeInTheDocument();
  });
});

describe('TableLoading', () => {
  it('renders with default rows and columns', () => {
    render(<TableLoading />);
    const { container } = document;
    expect(container).toBeTruthy();
  });

  it('renders with custom rows and columns', () => {
    render(<TableLoading rows={10} cols={6} />);
    const { container } = document;
    expect(container).toBeTruthy();
  });
});
