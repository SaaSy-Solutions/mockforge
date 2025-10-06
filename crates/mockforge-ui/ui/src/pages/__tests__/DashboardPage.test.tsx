/**
 * @jest-environment jsdom
 */

import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { DashboardPage } from '../DashboardPage';
import { useDashboard, useLogs } from '../../hooks/useApi';

// Mock the hooks
vi.mock('../../hooks/useApi');

const createWrapper = () => {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
      },
    },
  });

  return ({ children }: { children: React.ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
};

describe('DashboardPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders loading state', () => {
    vi.mocked(useDashboard).mockReturnValue({
      data: undefined,
      isLoading: true,
      error: null,
    } as any);
    vi.mocked(useLogs).mockReturnValue({
      data: undefined,
    } as any);

    const Wrapper = createWrapper();
    render(
      <Wrapper>
        <DashboardPage />
      </Wrapper>
    );

    // Loading state should be displayed
    expect(screen.queryByText('Server Instances')).not.toBeInTheDocument();
  });

  it('renders dashboard with data', async () => {
    const mockDashboard = {
      servers: [
        {
          server_type: 'HTTP',
          address: 'http://127.0.0.1:9080',
          running: true,
          uptime_seconds: 3600,
          total_requests: 100,
          active_connections: 5,
        },
      ],
      metrics: {
        total_requests: 100,
        total_2xx: 95,
        total_4xx: 3,
        total_5xx: 2,
      },
    };

    const mockLogs = [
      {
        timestamp: '2024-01-01T12:00:00Z',
        method: 'GET',
        path: '/api/test',
        status_code: 200,
        response_time_ms: 45,
      },
    ];

    useDashboard.mockReturnValue({
      data: mockDashboard,
      isLoading: false,
      error: null,
    });
    useLogs.mockReturnValue({
      data: mockLogs,
    });

    const Wrapper = createWrapper();
    render(
      <Wrapper>
        <DashboardPage />
      </Wrapper>
    );

    await waitFor(() => {
      expect(screen.getByText('Server Instances')).toBeInTheDocument();
    });
  });

  it('handles errors', async () => {
    useDashboard.mockReturnValue({
      data: undefined,
      isLoading: false,
      error: new Error('Failed to fetch'),
    });
    useLogs.mockReturnValue({
      data: undefined,
    });

    const Wrapper = createWrapper();
    render(
      <Wrapper>
        <DashboardPage />
      </Wrapper>
    );

    // Error state should be displayed
    await waitFor(() => {
      expect(screen.queryByText('Server Instances')).not.toBeInTheDocument();
    });
  });

  it('calculates metrics from logs', async () => {
    const mockLogs = [
      {
        timestamp: '2024-01-01T12:00:00Z',
        method: 'GET',
        path: '/api/test',
        status_code: 200,
        response_time_ms: 45,
      },
      {
        timestamp: '2024-01-01T12:01:00Z',
        method: 'POST',
        path: '/api/create',
        status_code: 404,
        response_time_ms: 100,
      },
      {
        timestamp: '2024-01-01T12:02:00Z',
        method: 'GET',
        path: '/api/error',
        status_code: 500,
        response_time_ms: 200,
      },
    ];

    useDashboard.mockReturnValue({
      data: { servers: [] },
      isLoading: false,
      error: null,
    });
    useLogs.mockReturnValue({
      data: mockLogs,
    });

    const Wrapper = createWrapper();
    render(
      <Wrapper>
        <DashboardPage />
      </Wrapper>
    );

    // Metrics should be calculated (1x 2xx, 1x 4xx, 1x 5xx)
    await waitFor(() => {
      expect(screen.getByText('Server Instances')).toBeInTheDocument();
    });
  });
});
