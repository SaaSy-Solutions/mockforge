import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { HistoryPanel } from '../HistoryPanel';
import { usePlaygroundStore } from '../../../stores/usePlaygroundStore';

// Mock the store
vi.mock('../../../stores/usePlaygroundStore');
vi.mock('sonner', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

describe('HistoryPanel', () => {
  const mockLoadHistory = vi.fn();
  const mockReplayRequest = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();

    (usePlaygroundStore as any).mockReturnValue({
      history: [],
      historyLoading: false,
      historyError: null,
      loadHistory: mockLoadHistory,
      replayRequest: mockReplayRequest,
    });
  });

  it('loads history on mount', () => {
    render(<HistoryPanel />);

    expect(mockLoadHistory).toHaveBeenCalledWith({ limit: 50 });
  });

  it('renders empty state when no history', () => {
    render(<HistoryPanel />);

    expect(screen.getByText(/No requests found/i)).toBeInTheDocument();
  });

  it('renders history entries', () => {
    (usePlaygroundStore as any).mockReturnValue({
      history: [
        {
          id: '1',
          protocol: 'rest',
          method: 'GET',
          path: '/api/users',
          status_code: 200,
          response_time_ms: 150,
          timestamp: new Date().toISOString(),
        },
      ],
      historyLoading: false,
      historyError: null,
      loadHistory: mockLoadHistory,
      replayRequest: mockReplayRequest,
    });

    render(<HistoryPanel />);

    expect(screen.getByText('/api/users')).toBeInTheDocument();
    expect(screen.getByText('200')).toBeInTheDocument();
  });

  it('filters history by search query', () => {
    (usePlaygroundStore as any).mockReturnValue({
      history: [
        {
          id: '1',
          protocol: 'rest',
          method: 'GET',
          path: '/api/users',
          status_code: 200,
          response_time_ms: 150,
          timestamp: new Date().toISOString(),
        },
        {
          id: '2',
          protocol: 'rest',
          method: 'POST',
          path: '/api/posts',
          status_code: 201,
          response_time_ms: 200,
          timestamp: new Date().toISOString(),
        },
      ],
      historyLoading: false,
      historyError: null,
      loadHistory: mockLoadHistory,
      replayRequest: mockReplayRequest,
    });

    render(<HistoryPanel />);

    const searchInput = screen.getByPlaceholderText(/Search requests/i);
    fireEvent.change(searchInput, { target: { value: 'users' } });

    expect(screen.getByText('/api/users')).toBeInTheDocument();
    expect(screen.queryByText('/api/posts')).not.toBeInTheDocument();
  });

  it('replays request when clicked', async () => {
    (usePlaygroundStore as any).mockReturnValue({
      history: [
        {
          id: '1',
          protocol: 'rest',
          method: 'GET',
          path: '/api/users',
          status_code: 200,
          response_time_ms: 150,
          timestamp: new Date().toISOString(),
        },
      ],
      historyLoading: false,
      historyError: null,
      loadHistory: mockLoadHistory,
      replayRequest: mockReplayRequest,
    });

    render(<HistoryPanel />);

    const replayButton = screen.getByRole('button', { name: '' });
    fireEvent.click(replayButton);

    await waitFor(() => {
      expect(mockReplayRequest).toHaveBeenCalledWith('1');
    });
  });
});
