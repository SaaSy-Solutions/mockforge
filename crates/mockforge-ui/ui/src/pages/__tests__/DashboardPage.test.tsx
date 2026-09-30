/**
 * @jest-environment jsdom
 */

import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { DashboardPage } from '../DashboardPage';
import { useDashboard, useLogs } from '../../hooks/useApi';
import type { DashboardData, RequestLog, SystemInfo } from '../../types';

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

// DashboardPage captures `const isCloud = isCloudMode()` at module load,
// so per-test env stubbing is too late. Mock the module to keep the page
// in local mode for the existing tests.
vi.mock('../../utils/cloudMode', () => ({
  isCloudMode: () => false,
  getCloudApiBase: () => '',
}));

type LogFields = Pick<RequestLog, 'timestamp' | 'method' | 'path' | 'status_code' | 'response_time_ms'>;

const toRequestLogs = (entries: LogFields[]): RequestLog[] =>
  entries.map((entry, i) => ({ id: String(i + 1), headers: {}, response_size_bytes: 0, ...entry }));

const makeDashboard = (system: Omit<SystemInfo, 'total_routes' | 'total_fixtures'>): DashboardData => ({
  server_info: { version: system.version, build_time: '', git_sha: '', api_enabled: true, admin_port: 9080 },
  system_info: { os: 'linux', arch: 'x86_64', uptime: system.uptime_seconds, memory_usage: system.memory_usage_mb },
  metrics: { total_requests: 0, active_requests: 0, average_response_time: 0, error_rate: 0 },
  servers: [],
  recent_logs: [],
  system: { ...system, total_routes: 0, total_fixtures: 0 },
});

describe('DashboardPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders loading state', () => {
    vi.mocked(useDashboard, { partial: true }).mockReturnValue({
      data: undefined,
      isLoading: true,
      error: null,
    });
    vi.mocked(useLogs, { partial: true }).mockReturnValue({
      data: undefined,
    });

    const Wrapper = createWrapper();
    render(
      <Wrapper>
        <DashboardPage />
      </Wrapper>
    );

    expect(screen.getByText('Dashboard')).toBeInTheDocument();
  });

  it('renders dashboard with data', async () => {
    const mockDashboard = makeDashboard({
      uptime_seconds: 3600,
      cpu_usage_percent: 10.5,
      memory_usage_mb: 512,
      active_threads: 4,
      version: '1.0.0',
    });

    const mockLogs = toRequestLogs([
      {
        timestamp: '2024-01-01T12:00:00Z',
        method: 'GET',
        path: '/api/test',
        status_code: 200,
        response_time_ms: 45,
      },
    ]);

    vi.mocked(useDashboard, { partial: true }).mockReturnValue({
      data: mockDashboard,
      isLoading: false,
      error: null,
    });
    vi.mocked(useLogs, { partial: true }).mockReturnValue({
      data: mockLogs,
    });

    const Wrapper = createWrapper();
    render(
      <Wrapper>
        <DashboardPage />
      </Wrapper>
    );

    await waitFor(() => {
      expect(screen.getByRole('heading', { name: 'System' })).toBeInTheDocument();
      expect(screen.getByText('1.0.0')).toBeInTheDocument();
      expect(screen.getByText('Error rate')).toBeInTheDocument();
    });
  });

  it('handles errors', async () => {
    vi.mocked(useDashboard, { partial: true }).mockReturnValue({
      data: undefined,
      isLoading: false,
      error: new Error('Failed to fetch'),
    });
    vi.mocked(useLogs, { partial: true }).mockReturnValue({
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
    const mockLogs = toRequestLogs([
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
    ]);

    vi.mocked(useDashboard, { partial: true }).mockReturnValue({
      data: makeDashboard({
        uptime_seconds: 3600,
        cpu_usage_percent: 10.5,
        memory_usage_mb: 512,
        active_threads: 4,
        version: '1.0.0',
      }),
      isLoading: false,
      error: null,
    });
    vi.mocked(useLogs, { partial: true }).mockReturnValue({
      data: mockLogs,
    });

    const Wrapper = createWrapper();
    render(
      <Wrapper>
        <DashboardPage />
      </Wrapper>
    );

    await waitFor(() => {
      expect(screen.getByText('2xx Success')).toBeInTheDocument();
      expect(screen.getByText('4xx Client errors')).toBeInTheDocument();
      expect(screen.getByText('5xx Server errors')).toBeInTheDocument();
      // One of each → an even three-way split in the stacked bar.
      expect(
        screen.getByRole('img', {
          name: '2xx Success 33.3%, 4xx Client errors 33.3%, 5xx Server errors 33.3%',
        }),
      ).toBeInTheDocument();
    });
  });
});
