import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import { ResponsePanel } from '../ResponsePanel';
import { usePlaygroundStore } from '../../../stores/usePlaygroundStore';

// Mock the store
vi.mock('../../../stores/usePlaygroundStore');
vi.mock('sonner', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

describe('ResponsePanel', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders empty state when no response', () => {
    (usePlaygroundStore as any).mockReturnValue({
      currentResponse: null,
      responseError: null,
      responseLoading: false,
    });

    render(<ResponsePanel />);

    expect(screen.getByText(/No Response/i)).toBeInTheDocument();
  });

  it('renders loading state', () => {
    (usePlaygroundStore as any).mockReturnValue({
      currentResponse: null,
      responseError: null,
      responseLoading: true,
    });

    render(<ResponsePanel />);

    expect(screen.getByText(/Executing request/i)).toBeInTheDocument();
  });

  it('renders response with status code and timing', () => {
    (usePlaygroundStore as any).mockReturnValue({
      currentResponse: {
        status_code: 200,
        headers: { 'Content-Type': 'application/json' },
        body: { success: true },
        response_time_ms: 150,
        request_id: 'test-id',
        error: null,
      },
      responseError: null,
      responseLoading: false,
    });

    render(<ResponsePanel />);

    expect(screen.getByText('200')).toBeInTheDocument();
    expect(screen.getByText(/150ms/i)).toBeInTheDocument();
  });

  it('renders error state', () => {
    (usePlaygroundStore as any).mockReturnValue({
      currentResponse: null,
      responseError: 'Request failed',
      responseLoading: false,
    });

    render(<ResponsePanel />);

    expect(screen.getByText(/Error/i)).toBeInTheDocument();
    expect(screen.getByText('Request failed')).toBeInTheDocument();
  });
});
