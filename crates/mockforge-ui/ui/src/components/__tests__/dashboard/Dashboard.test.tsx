/**
 * @jest-environment jsdom
 */

import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { MetricCard } from '../../dashboard/MetricCard';
import { StatCard } from '../../dashboard/StatCard';
import { ServerTable } from '../../dashboard/ServerTable';
import { RequestLog } from '../../dashboard/RequestLog';
import { Activity, Users, Zap } from 'lucide-react';

// Mock dependencies
vi.mock('../../../hooks/useApi', () => ({
  useDashboard: vi.fn(),
  useLogs: vi.fn(),
  useClearLogs: vi.fn(),
}));

vi.mock('../../../hooks/useErrorHandling', () => ({
  useApiErrorHandling: vi.fn(),
}));

vi.mock('../../../stores/usePreferencesStore', () => ({
  usePreferencesStore: vi.fn(),
}));

vi.mock('../../ui/ToastProvider', () => ({
  useErrorToast: vi.fn(() => vi.fn()),
}));

describe('MetricCard', () => {
  it('renders title and value', () => {
    render(<MetricCard title="Total Requests" value={1234} />);

    expect(screen.getByText('Total Requests')).toBeInTheDocument();
    expect(screen.getByText('1234')).toBeInTheDocument();
  });

  it('renders subtitle when provided', () => {
    render(<MetricCard title="Total Requests" value={1234} subtitle="requests/sec" />);

    expect(screen.getByText('requests/sec')).toBeInTheDocument();
  });

  it('renders icon when provided', () => {
    render(
      <MetricCard
        title="Total Requests"
        value={1234}
        icon={<Activity data-testid="activity-icon" />}
      />
    );

    expect(screen.getByTestId('activity-icon')).toBeInTheDocument();
  });

  it('renders up trend indicator', () => {
    render(
      <MetricCard
        title="Total Requests"
        value={1234}
        trend="up"
        trendValue="+12%"
      />
    );

    expect(screen.getByText('+12%')).toBeInTheDocument();
    expect(screen.getByText('vs last hour')).toBeInTheDocument();
  });

  it('renders down trend indicator', () => {
    render(
      <MetricCard
        title="Total Requests"
        value={1234}
        trend="down"
        trendValue="-5%"
      />
    );

    expect(screen.getByText('-5%')).toBeInTheDocument();
    expect(screen.getByText('vs last hour')).toBeInTheDocument();
  });

  it('renders neutral trend indicator', () => {
    render(
      <MetricCard
        title="Total Requests"
        value={1234}
        trend="neutral"
        trendValue="0%"
      />
    );

    expect(screen.getByText('0%')).toBeInTheDocument();
  });

  it('accepts custom className', () => {
    const { container } = render(
      <MetricCard title="Total Requests" value={1234} className="custom-class" />
    );

    expect(container.querySelector('.custom-class')).toBeInTheDocument();
  });

  it('handles string values', () => {
    render(<MetricCard title="Status" value="Running" />);

    expect(screen.getByText('Running')).toBeInTheDocument();
  });
});

describe('StatCard', () => {
  it('renders title and value', () => {
    render(<StatCard title="Active Users" value={42} />);

    expect(screen.getByText(/Active Users/i)).toBeInTheDocument();
    expect(screen.getByText('42')).toBeInTheDocument();
  });

  it('renders subtitle when provided', () => {
    render(<StatCard title="Active Users" value={42} subtitle="online" />);

    expect(screen.getByText('online')).toBeInTheDocument();
  });

  it('renders icon when provided', () => {
    render(
      <StatCard
        title="Active Users"
        value={42}
        icon={<Users data-testid="users-icon" />}
      />
    );

    expect(screen.getByTestId('users-icon')).toBeInTheDocument();
  });

  it('renders up trend indicator', () => {
    render(
      <StatCard
        title="Active Users"
        value={42}
        trend="up"
        trendValue="+15%"
      />
    );

    expect(screen.getByText('+15%')).toBeInTheDocument();
    expect(screen.getByText('vs last hour')).toBeInTheDocument();
  });

  it('renders down trend indicator', () => {
    render(
      <StatCard
        title="Active Users"
        value={42}
        trend="down"
        trendValue="-10%"
      />
    );

    expect(screen.getByText('-10%')).toBeInTheDocument();
  });

  it('applies custom className', () => {
    const { container } = render(
      <StatCard title="Active Users" value={42} className="stat-custom" />
    );

    expect(container.querySelector('.stat-custom')).toBeInTheDocument();
  });

  it('handles string values', () => {
    render(<StatCard title="Server Status" value="Healthy" />);

    expect(screen.getByText('Healthy')).toBeInTheDocument();
  });
});

describe('ServerTable', () => {
  const { useDashboard } = await import('../../../hooks/useApi');
  const { usePreferencesStore } = await import('../../../stores/usePreferencesStore');

  const mockUseDashboard = vi.mocked(useDashboard);
  const mockUsePreferencesStore = vi.mocked(usePreferencesStore);

  beforeEach(() => {
    vi.clearAllMocks();

    // Default mock for preferences store
    mockUsePreferencesStore.mockReturnValue({
      preferences: {
        ui: {
          serverTableDensity: 'comfortable',
        },
      },
      updateUI: vi.fn(),
    } as any);
  });

  it('renders loading state', () => {
    mockUseDashboard.mockReturnValue({
      data: undefined,
      isLoading: true,
      error: null,
      refetch: vi.fn(),
    } as any);

    render(<ServerTable />);

    expect(screen.getByText('Server Instances')).toBeInTheDocument();
    expect(screen.getByText('Running MockForge services')).toBeInTheDocument();
  });

  it('renders error state', () => {
    mockUseDashboard.mockReturnValue({
      data: undefined,
      isLoading: false,
      error: new Error('Failed to fetch'),
      refetch: vi.fn(),
    } as any);

    render(<ServerTable />);

    expect(screen.getByText('Failed to load server data')).toBeInTheDocument();
  });

  it('renders empty state when no servers', () => {
    mockUseDashboard.mockReturnValue({
      data: { servers: [] },
      isLoading: false,
      error: null,
      refetch: vi.fn(),
    } as any);

    render(<ServerTable />);

    expect(screen.getByText('No servers running')).toBeInTheDocument();
  });

  it('renders server list with data', () => {
    mockUseDashboard.mockReturnValue({
      data: {
        servers: [
          {
            server_type: 'HTTP',
            address: 'http://127.0.0.1:9080',
            running: true,
            uptime_seconds: 3600,
            total_requests: 1234,
            active_connections: 5,
          },
          {
            server_type: 'WebSocket',
            address: 'ws://127.0.0.1:9081',
            running: true,
            uptime_seconds: 1800,
            total_requests: 567,
            active_connections: 2,
          },
        ],
      },
      isLoading: false,
      error: null,
      refetch: vi.fn(),
    } as any);

    render(<ServerTable />);

    expect(screen.getByText('HTTP')).toBeInTheDocument();
    expect(screen.getByText('WebSocket')).toBeInTheDocument();
    expect(screen.getByText('1,234')).toBeInTheDocument();
    expect(screen.getByText('567')).toBeInTheDocument();
  });

  it('displays uptime correctly', () => {
    mockUseDashboard.mockReturnValue({
      data: {
        servers: [
          {
            server_type: 'HTTP',
            address: 'http://127.0.0.1:9080',
            running: true,
            uptime_seconds: 7260, // 2 hours 1 minute
            total_requests: 100,
            active_connections: 1,
          },
        ],
      },
      isLoading: false,
      error: null,
      refetch: vi.fn(),
    } as any);

    render(<ServerTable />);

    expect(screen.getByText('2h 1m')).toBeInTheDocument();
  });

  it('handles refresh button click', async () => {
    const mockRefetch = vi.fn();
    mockUseDashboard.mockReturnValue({
      data: {
        servers: [
          {
            server_type: 'HTTP',
            address: 'http://127.0.0.1:9080',
            running: true,
            uptime_seconds: 100,
            total_requests: 10,
            active_connections: 1,
          },
        ],
      },
      isLoading: false,
      error: null,
      refetch: mockRefetch,
    } as any);

    render(<ServerTable />);

    const refreshButton = screen.getByText('Refresh').closest('button');
    fireEvent.click(refreshButton!);

    await waitFor(() => {
      expect(mockRefetch).toHaveBeenCalled();
    });
  });

  it('toggles between comfortable and compact view', () => {
    const mockUpdateUI = vi.fn();
    mockUsePreferencesStore.mockReturnValue({
      preferences: {
        ui: {
          serverTableDensity: 'comfortable',
        },
      },
      updateUI: mockUpdateUI,
    } as any);

    mockUseDashboard.mockReturnValue({
      data: {
        servers: [
          {
            server_type: 'HTTP',
            address: 'http://127.0.0.1:9080',
            running: true,
            uptime_seconds: 100,
            total_requests: 10,
            active_connections: 1,
          },
        ],
      },
      isLoading: false,
      error: null,
      refetch: vi.fn(),
    } as any);

    render(<ServerTable />);

    const compactButton = screen.getByText('Compact');
    fireEvent.click(compactButton);

    expect(mockUpdateUI).toHaveBeenCalledWith({ serverTableDensity: 'compact' });
  });

  it('shows warning for server without address', () => {
    mockUseDashboard.mockReturnValue({
      data: {
        servers: [
          {
            server_type: 'HTTP',
            address: undefined,
            running: true,
            uptime_seconds: 100,
            total_requests: 10,
            active_connections: 1,
          },
        ],
      },
      isLoading: false,
      error: null,
      refetch: vi.fn(),
    } as any);

    render(<ServerTable />);

    expect(screen.getByText(/Address not configured/i)).toBeInTheDocument();
  });
});

describe('RequestLog', () => {
  const { useLogs, useClearLogs } = await import('../../../hooks/useApi');
  const { useApiErrorHandling } = await import('../../../hooks/useErrorHandling');

  const mockUseLogs = vi.mocked(useLogs);
  const mockUseClearLogs = vi.mocked(useClearLogs);
  const mockUseApiErrorHandling = vi.mocked(useApiErrorHandling);

  beforeEach(() => {
    vi.clearAllMocks();

    // Default mock for error handling
    mockUseApiErrorHandling.mockReturnValue({
      handleApiError: vi.fn(),
      retry: vi.fn(),
      clearError: vi.fn(),
      errorState: { error: null },
      canRetry: false,
    } as any);

    // Default mock for clear logs mutation
    mockUseClearLogs.mockReturnValue({
      mutateAsync: vi.fn(),
      isPending: false,
    } as any);
  });

  it('renders loading state', () => {
    mockUseLogs.mockReturnValue({
      data: undefined,
      isLoading: true,
      error: null,
      refetch: vi.fn(),
    } as any);

    render(<RequestLog />);

    expect(screen.getByText('Recent Requests')).toBeInTheDocument();
  });

  it('renders request logs', () => {
    mockUseLogs.mockReturnValue({
      data: [
        {
          id: '1',
          timestamp: '2024-01-01T12:00:00Z',
          method: 'GET',
          path: '/api/users',
          status_code: 200,
          response_time_ms: 45,
        },
        {
          id: '2',
          timestamp: '2024-01-01T12:01:00Z',
          method: 'POST',
          path: '/api/users',
          status_code: 201,
          response_time_ms: 123,
        },
      ],
      isLoading: false,
      error: null,
      refetch: vi.fn(),
    } as any);

    render(<RequestLog />);

    expect(screen.getByText('GET')).toBeInTheDocument();
    expect(screen.getByText('POST')).toBeInTheDocument();
    expect(screen.getAllByText('/api/users')).toHaveLength(2);
  });

  it('filters logs by status family', async () => {
    mockUseLogs.mockReturnValue({
      data: [
        {
          id: '1',
          timestamp: '2024-01-01T12:00:00Z',
          method: 'GET',
          path: '/api/users',
          status_code: 200,
          response_time_ms: 45,
        },
        {
          id: '2',
          timestamp: '2024-01-01T12:01:00Z',
          method: 'GET',
          path: '/api/error',
          status_code: 500,
          response_time_ms: 100,
        },
      ],
      isLoading: false,
      error: null,
      refetch: vi.fn(),
    } as any);

    render(<RequestLog />);

    // Click on 5xx filter
    const fiveXxButton = screen.getByText('5XX');
    fireEvent.click(fiveXxButton);

    await waitFor(() => {
      expect(screen.queryByText('/api/users')).not.toBeInTheDocument();
      expect(screen.getByText('/api/error')).toBeInTheDocument();
    });
  });

  it('filters logs by method', async () => {
    mockUseLogs.mockReturnValue({
      data: [
        {
          id: '1',
          timestamp: '2024-01-01T12:00:00Z',
          method: 'GET',
          path: '/api/users',
          status_code: 200,
          response_time_ms: 45,
        },
        {
          id: '2',
          timestamp: '2024-01-01T12:01:00Z',
          method: 'POST',
          path: '/api/users',
          status_code: 201,
          response_time_ms: 100,
        },
      ],
      isLoading: false,
      error: null,
      refetch: vi.fn(),
    } as any);

    render(<RequestLog />);

    const postButton = screen.getByText('POST');
    fireEvent.click(postButton);

    await waitFor(() => {
      expect(screen.getAllByText('POST')).toHaveLength(2); // One in button, one in table
    });
  });

  it('searches logs by path', async () => {
    vi.useFakeTimers();

    mockUseLogs.mockReturnValue({
      data: [
        {
          id: '1',
          timestamp: '2024-01-01T12:00:00Z',
          method: 'GET',
          path: '/api/users',
          status_code: 200,
          response_time_ms: 45,
        },
        {
          id: '2',
          timestamp: '2024-01-01T12:01:00Z',
          method: 'GET',
          path: '/api/products',
          status_code: 200,
          response_time_ms: 100,
        },
      ],
      isLoading: false,
      error: null,
      refetch: vi.fn(),
    } as any);

    render(<RequestLog />);

    const searchInput = screen.getByPlaceholderText(/Search path/i);
    fireEvent.change(searchInput, { target: { value: 'products' } });

    // Wait for debounce
    vi.advanceTimersByTime(300);

    await waitFor(() => {
      expect(screen.queryByText('/api/users')).not.toBeInTheDocument();
      expect(screen.getByText('/api/products')).toBeInTheDocument();
    });

    vi.useRealTimers();
  });

  it('clears logs when clear button is clicked', async () => {
    const mockMutateAsync = vi.fn();
    mockUseClearLogs.mockReturnValue({
      mutateAsync: mockMutateAsync,
      isPending: false,
    } as any);

    mockUseLogs.mockReturnValue({
      data: [
        {
          id: '1',
          timestamp: '2024-01-01T12:00:00Z',
          method: 'GET',
          path: '/api/users',
          status_code: 200,
          response_time_ms: 45,
        },
      ],
      isLoading: false,
      error: null,
      refetch: vi.fn(),
    } as any);

    render(<RequestLog />);

    const clearButton = screen.getByText('Clear');
    fireEvent.click(clearButton);

    await waitFor(() => {
      expect(mockMutateAsync).toHaveBeenCalled();
    });
  });

  it('shows empty message when no logs match filters', () => {
    mockUseLogs.mockReturnValue({
      data: [],
      isLoading: false,
      error: null,
      refetch: vi.fn(),
    } as any);

    render(<RequestLog />);

    expect(screen.getByText('No requests found')).toBeInTheDocument();
  });
});
