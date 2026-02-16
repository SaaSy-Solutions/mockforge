/**
 * @jest-environment jsdom
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import ResponseHistory from '../ResponseHistory';
import { apiService } from '../../../services/api';
import type { ResponseHistoryEntry } from '../../../types';

vi.mock('../../../services/api');
vi.mock('sonner', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

const mockHistory: ResponseHistoryEntry[] = [
  {
    executed_at: '2024-01-01T10:00:00Z',
    request_method: 'GET',
    request_path: '/api/users',
    request_headers: { 'Content-Type': 'application/json' },
    request_body: null,
    response_status_code: 200,
    response_headers: { 'Content-Type': 'application/json' },
    response_body: '{"users": []}',
    response_time_ms: 45,
    response_size_bytes: 1024,
    error_message: null,
  },
  {
    executed_at: '2024-01-01T09:00:00Z',
    request_method: 'POST',
    request_path: '/api/posts',
    request_headers: {},
    request_body: '{"title": "Test"}',
    response_status_code: 500,
    response_headers: {},
    response_body: null,
    response_time_ms: 120,
    response_size_bytes: 0,
    error_message: 'Internal server error',
  },
];

describe('ResponseHistory', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    (apiService.getRequestHistory as any) = vi.fn().mockResolvedValue({ history: mockHistory });
  });

  it('renders loading state initially', () => {
    (apiService.getRequestHistory as any) = vi.fn(() => new Promise(() => {}));

    render(<ResponseHistory workspaceId="ws-1" requestId="req-1" requestName="Test Request" />);

    expect(screen.getByText('Response History')).toBeInTheDocument();
    expect(document.querySelector('.animate-spin')).toBeInTheDocument();
  });

  it('loads and displays history on mount', async () => {
    render(<ResponseHistory workspaceId="ws-1" requestId="req-1" requestName="Test Request" />);

    await waitFor(() => {
      expect(apiService.getRequestHistory).toHaveBeenCalledWith('ws-1', 'req-1');
      expect(screen.getByText('2 executions for Test Request')).toBeInTheDocument();
    });
  });

  it('displays history entries', async () => {
    render(<ResponseHistory workspaceId="ws-1" requestId="req-1" requestName="Test Request" />);

    await waitFor(() => {
      expect(screen.getByText('GET')).toBeInTheDocument();
      expect(screen.getByText('/api/users')).toBeInTheDocument();
      expect(screen.getByText('200')).toBeInTheDocument();
      expect(screen.getByText('45ms')).toBeInTheDocument();
    });
  });

  it('shows error entry with error message', async () => {
    render(<ResponseHistory workspaceId="ws-1" requestId="req-1" requestName="Test Request" />);

    await waitFor(() => {
      expect(screen.getByText('500')).toBeInTheDocument();
      expect(screen.getByText('Internal server error')).toBeInTheDocument();
    });
  });

  it('formats file size correctly', async () => {
    render(<ResponseHistory workspaceId="ws-1" requestId="req-1" requestName="Test Request" />);

    await waitFor(() => {
      expect(screen.getByText('1.0KB')).toBeInTheDocument();
      expect(screen.getByText('0B')).toBeInTheDocument();
    });
  });

  it('executes request when execute button clicked', async () => {
    const onExecuteRequest = vi.fn();
    const executionResult = {
      execution: {
        ...mockHistory[0],
        executed_at: '2024-01-01T11:00:00Z',
      },
    };
    (apiService.executeRequest as any) = vi.fn().mockResolvedValue(executionResult);

    render(
      <ResponseHistory
        workspaceId="ws-1"
        requestId="req-1"
        requestName="Test Request"
        onExecuteRequest={onExecuteRequest}
      />
    );

    await waitFor(() => {
      const executeButton = screen.getByText('Execute Request');
      fireEvent.click(executeButton);
    });

    await waitFor(() => {
      expect(apiService.executeRequest).toHaveBeenCalledWith('ws-1', 'req-1');
      expect(onExecuteRequest).toHaveBeenCalled();
    });
  });

  it('disables execute button while executing', async () => {
    let resolveExecution: (value: any) => void;
    (apiService.executeRequest as any) = vi.fn(
      () =>
        new Promise((resolve) => {
          resolveExecution = resolve;
        })
    );

    render(<ResponseHistory workspaceId="ws-1" requestId="req-1" requestName="Test Request" />);

    await waitFor(() => screen.getByText('Execute Request'));

    const executeButton = screen.getByText('Execute Request');
    fireEvent.click(executeButton);

    await waitFor(() => {
      expect(screen.getByText('Executing...')).toBeInTheDocument();
      expect(executeButton).toBeDisabled();
    });

    resolveExecution!({ execution: mockHistory[0] });
  });

  it('shows empty state when no history', async () => {
    (apiService.getRequestHistory as any) = vi.fn().mockResolvedValue({ history: [] });

    render(<ResponseHistory workspaceId="ws-1" requestId="req-1" requestName="Test Request" />);

    await waitFor(() => {
      expect(screen.getByText('No executions yet')).toBeInTheDocument();
      expect(screen.getByText('Execute the request to see history')).toBeInTheDocument();
    });
  });

  it('shows error state when loading fails', async () => {
    (apiService.getRequestHistory as any) = vi.fn().mockRejectedValue(new Error('Failed to load'));

    render(<ResponseHistory workspaceId="ws-1" requestId="req-1" requestName="Test Request" />);

    await waitFor(() => {
      expect(screen.getByText('Failed to load')).toBeInTheDocument();
      expect(screen.getByText('Retry')).toBeInTheDocument();
    });
  });

  it('retries loading when retry button clicked', async () => {
    (apiService.getRequestHistory as any) = vi
      .fn()
      .mockRejectedValueOnce(new Error('Failed'))
      .mockResolvedValueOnce({ history: mockHistory });

    render(<ResponseHistory workspaceId="ws-1" requestId="req-1" requestName="Test Request" />);

    await waitFor(() => screen.getByText('Retry'));

    fireEvent.click(screen.getByText('Retry'));

    await waitFor(() => {
      expect(screen.getByText('2 executions for Test Request')).toBeInTheDocument();
    });
  });

  it('switches between tabs', async () => {
    render(<ResponseHistory workspaceId="ws-1" requestId="req-1" requestName="Test Request" />);

    await waitFor(() => screen.getAllByText('Response'));

    // Switch to Request tab
    fireEvent.click(screen.getAllByText('Request')[1]);
    expect(screen.getByText('Request Body')).toBeInTheDocument();

    // Switch to Headers tab
    fireEvent.click(screen.getAllByText('Headers')[0]);
    expect(screen.getByText('Response Headers')).toBeInTheDocument();
  });

  it('displays request body when available', async () => {
    render(<ResponseHistory workspaceId="ws-1" requestId="req-1" requestName="Test Request" />);

    await waitFor(() => screen.getAllByText('Request'));

    fireEvent.click(screen.getAllByText('Request')[1]);

    expect(screen.getByText('{"title": "Test"}')).toBeInTheDocument();
  });

  it('displays request headers when available', async () => {
    render(<ResponseHistory workspaceId="ws-1" requestId="req-1" requestName="Test Request" />);

    await waitFor(() => screen.getAllByText('Request'));

    fireEvent.click(screen.getAllByText('Request')[0]);

    expect(screen.getByText('Content-Type:')).toBeInTheDocument();
    expect(screen.getByText('application/json')).toBeInTheDocument();
  });

  it('displays response headers', async () => {
    render(<ResponseHistory workspaceId="ws-1" requestId="req-1" requestName="Test Request" />);

    await waitFor(() => screen.getAllByText('Headers'));

    fireEvent.click(screen.getAllByText('Headers')[0]);

    expect(screen.getByText('Response Headers')).toBeInTheDocument();
  });

  it('shows empty response message when response body is null', async () => {
    (apiService.getRequestHistory as any) = vi.fn().mockResolvedValue({
      history: [
        {
          ...mockHistory[0],
          response_body: null,
          error_message: null,
        },
      ],
    });

    render(<ResponseHistory workspaceId="ws-1" requestId="req-1" requestName="Test Request" />);

    await waitFor(() => {
      expect(screen.getByText('1 execution for Test Request')).toBeInTheDocument();
    });

    expect(screen.getByText('(empty response)')).toBeInTheDocument();
  });

  it('shows status icons correctly', async () => {
    render(<ResponseHistory workspaceId="ws-1" requestId="req-1" requestName="Test Request" />);

    await waitFor(() => {
      // 200 should have CheckCircle icon
      const successIcons = document.querySelectorAll('.text-green-500');
      expect(successIcons.length).toBeGreaterThan(0);

      // 500 should have XCircle icon
      const errorIcons = document.querySelectorAll('.text-red-500');
      expect(errorIcons.length).toBeGreaterThan(0);
    });
  });

  it('formats timestamps correctly', async () => {
    render(<ResponseHistory workspaceId="ws-1" requestId="req-1" requestName="Test Request" />);

    await waitFor(() => {
      expect(screen.getAllByText(/\d{4}/).length).toBeGreaterThan(0);
    });
  });

  it('updates history when new execution completes', async () => {
    const { rerender } = render(
      <ResponseHistory workspaceId="ws-1" requestId="req-1" requestName="Test Request" />
    );

    await waitFor(() => screen.getByText('2 executions for Test Request'));

    const newExecution = {
      execution: {
        ...mockHistory[0],
        executed_at: '2024-01-01T11:00:00Z',
      },
    };
    (apiService.executeRequest as any) = vi.fn().mockResolvedValue(newExecution);

    fireEvent.click(screen.getByText('Execute Request'));

    await waitFor(() => {
      expect(screen.getByText('3 executions for Test Request')).toBeInTheDocument();
    });
  });
});
