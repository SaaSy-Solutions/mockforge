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
vi.mock('../../components/time-travel/TimeTravelWidget', () => ({
  TimeTravelWidget: () => <div>TimeTravelWidget</div>,
}));
vi.mock('../../components/reality/RealitySlider', () => ({
  RealitySlider: () => <div>RealitySlider</div>,
}));
vi.mock('../../components/reality/RealityIndicator', () => ({
  RealityIndicator: () => <div>RealityIndicator</div>,
}));
vi.mock('../../components/dashboard/ServerTable', () => ({
  ServerTable: () => <div>ServerTable</div>,
}));
vi.mock('../../components/dashboard/RequestLog', () => ({
  RequestLog: () => <div>RequestLog</div>,
}));
vi.mock('../../components/metrics/LatencyHistogram', () => ({
  LatencyHistogram: () => <div>LatencyHistogram</div>,
}));

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

    expect(screen.getByText('Dashboard')).toBeInTheDocument();
  });

  it('renders dashboard with data', async () => {
    const mockDashboard = {
      system: {
        uptime_seconds: 3600,
        cpu_usage_percent: 10.5,
        memory_usage_mb: 512,
        active_threads: 4,
        version: '1.0.0',
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
      expect(screen.getByText('System Metrics')).toBeInTheDocument();
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
      expect(screen.getByText('Failed to load dashboard')).toBeInTheDocument();
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
      data: {
        system: {
          uptime_seconds: 3600,
          cpu_usage_percent: 10.5,
          memory_usage_mb: 512,
          active_threads: 4,
          version: '1.0.0',
        },
      },
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
      expect(screen.getByText('Success Responses')).toBeInTheDocument();
      expect(screen.getByText('Client Errors')).toBeInTheDocument();
      expect(screen.getByText('Server Errors')).toBeInTheDocument();
    });
  });
});
