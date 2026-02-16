/**
 * @jest-environment jsdom
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { MetricsPage } from '../MetricsPage';
import * as apiHooks from '../../hooks/useApi';

const mockMetrics = {
  requests_by_endpoint: {
    'GET /api/users': 100,
    'POST /api/posts': 50,
    'DELETE /api/users/1': 10,
  },
  response_time_percentiles: {
    p50: 45,
    p95: 120,
    p99: 250,
  },
  error_rate_by_endpoint: {
    'GET /api/users': 0.01,
    'POST /api/posts': 0.05,
    'DELETE /api/users/1': 0.15,
  },
  memory_usage_over_time: [
    ['2024-01-01T10:00:00Z', 512],
    ['2024-01-01T10:01:00Z', 520],
  ],
  cpu_usage_over_time: [
    ['2024-01-01T10:00:00Z', 25],
    ['2024-01-01T10:01:00Z', 30],
  ],
};

vi.mock('../../hooks/useApi', () => ({
  useMetrics: vi.fn(() => ({
    data: mockMetrics,
    isLoading: false,
    error: null,
  })),
}));

vi.mock('../../components/metrics/PerformanceDashboard', () => ({
  PerformanceDashboard: () => <div data-testid="performance-dashboard" />,
}));

describe('MetricsPage', () => {
  const mockUseMetrics = vi.mocked(apiHooks.useMetrics);

  const createWrapper = () => {
    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    return ({ children }: { children: React.ReactNode }) => (
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    );
  };

  beforeEach(() => {
    vi.clearAllMocks();
    mockUseMetrics.mockReturnValue({
      data: mockMetrics,
      isLoading: false,
      error: null,
    } as any);
  });

  it('renders loading state', () => {
    mockUseMetrics.mockReturnValue({ data: null, isLoading: true, error: null } as any);

    render(<MetricsPage />, { wrapper: createWrapper() });
    expect(screen.getByText('Loading metrics...')).toBeInTheDocument();
  });

  it('displays key performance indicators', () => {
    render(<MetricsPage />, { wrapper: createWrapper() });

    expect(screen.getByText('Total Requests')).toBeInTheDocument();
    expect(screen.getByText('Avg Response Time')).toBeInTheDocument();
    expect(screen.getAllByText('Error Rate').length).toBeGreaterThan(0);
    expect(screen.getByText('Active Endpoints')).toBeInTheDocument();
  });

  it('calculates total requests correctly', () => {
    render(<MetricsPage />, { wrapper: createWrapper() });

    // 100 + 50 + 10 = 160
    expect(screen.getByText('160')).toBeInTheDocument();
  });

  it('calculates average response time correctly', () => {
    render(<MetricsPage />, { wrapper: createWrapper() });

    // Average of p50, p95, p99 = (45 + 120 + 250) / 3 = 138ms
    expect(screen.getByText('138ms')).toBeInTheDocument();
  });

  it('calculates average error rate correctly', () => {
    render(<MetricsPage />, { wrapper: createWrapper() });

    // Average of 1%, 5%, 15% = 7%
    expect(screen.getByText('7.0%')).toBeInTheDocument();
  });

  it('displays active endpoints count', () => {
    render(<MetricsPage />, { wrapper: createWrapper() });

    // 3 endpoints
    expect(screen.getByText('3')).toBeInTheDocument();
  });

  it('renders request distribution chart', () => {
    render(<MetricsPage />, { wrapper: createWrapper() });

    expect(screen.getByText('Requests by Endpoint')).toBeInTheDocument();
    expect(screen.getAllByText('/api/users').length).toBeGreaterThan(0);
    expect(screen.getAllByText('/api/posts').length).toBeGreaterThan(0);
  });

  it('renders response time percentiles chart', () => {
    render(<MetricsPage />, { wrapper: createWrapper() });

    expect(screen.getByText('Response Time Percentiles (ms)')).toBeInTheDocument();
    expect(screen.getByText('P50')).toBeInTheDocument();
    expect(screen.getByText('P95')).toBeInTheDocument();
    expect(screen.getByText('P99')).toBeInTheDocument();
  });

  it('displays error rate analysis', () => {
    render(<MetricsPage />, { wrapper: createWrapper() });

    expect(screen.getByText('Error Rates by Endpoint (%)')).toBeInTheDocument();
  });

  it('renders memory usage chart', () => {
    render(<MetricsPage />, { wrapper: createWrapper() });

    expect(screen.getByText('Memory Usage (MB)')).toBeInTheDocument();
  });

  it('renders CPU usage chart', () => {
    render(<MetricsPage />, { wrapper: createWrapper() });

    expect(screen.getByText('CPU Usage (%)')).toBeInTheDocument();
  });

  it('displays endpoint performance table', () => {
    render(<MetricsPage />, { wrapper: createWrapper() });

    expect(screen.getByText('GET /api/users')).toBeInTheDocument();
    expect(screen.getByText('POST /api/posts')).toBeInTheDocument();
    expect(screen.getByText('DELETE /api/users/1')).toBeInTheDocument();
  });

  it('shows error rate badges with correct variants', () => {
    render(<MetricsPage />, { wrapper: createWrapper() });

    // Different error rates should have different badge variants
    expect(screen.getByText('1.0%')).toBeInTheDocument(); // Low error rate - green
    expect(screen.getByText('5.0%')).toBeInTheDocument(); // Medium error rate - yellow
    expect(screen.getByText('15.0%')).toBeInTheDocument(); // High error rate - red
  });

  it('shows health status badges', () => {
    render(<MetricsPage />, { wrapper: createWrapper() });

    expect(screen.getAllByText('Issues').length).toBeGreaterThan(0);
  });

  it('handles error state', () => {
    mockUseMetrics.mockReturnValue({
      data: null,
      isLoading: false,
      error: new Error('Failed to load metrics'),
    } as any);

    render(<MetricsPage />, { wrapper: createWrapper() });

    expect(screen.getAllByText('Failed to load metrics').length).toBeGreaterThan(0);
  });

  it('displays warning when no metrics available', () => {
    mockUseMetrics.mockReturnValue({ data: null, isLoading: false, error: null } as any);

    render(<MetricsPage />, { wrapper: createWrapper() });

    expect(screen.getByText('No metrics available')).toBeInTheDocument();
  });

  it('shows empty state when no endpoint data', () => {
    mockUseMetrics.mockReturnValue({
      data: {
        requests_by_endpoint: {},
        response_time_percentiles: { p50: 0, p95: 0, p99: 0 },
        error_rate_by_endpoint: {},
        memory_usage_over_time: [],
        cpu_usage_over_time: [],
      },
      isLoading: false,
      error: null,
    } as any);

    render(<MetricsPage />, { wrapper: createWrapper() });

    expect(screen.getByText('No request data')).toBeInTheDocument();
  });

  it('shows empty state when no error data', () => {
    mockUseMetrics.mockReturnValue({
      data: {
        requests_by_endpoint: { 'GET /api/test': 10 },
        response_time_percentiles: { p50: 50, p95: 100, p99: 200 },
        error_rate_by_endpoint: {},
        memory_usage_over_time: [],
        cpu_usage_over_time: [],
      },
      isLoading: false,
      error: null,
    } as any);

    render(<MetricsPage />, { wrapper: createWrapper() });

    expect(screen.getByText('No error data')).toBeInTheDocument();
  });

  it('shows empty state when no time series data', () => {
    mockUseMetrics.mockReturnValue({
      data: {
        requests_by_endpoint: { 'GET /api/test': 10 },
        response_time_percentiles: { p50: 50, p95: 100, p99: 200 },
        error_rate_by_endpoint: { 'GET /api/test': 0.01 },
        memory_usage_over_time: [],
        cpu_usage_over_time: [],
      },
      isLoading: false,
      error: null,
    } as any);

    render(<MetricsPage />, { wrapper: createWrapper() });

    expect(screen.getByText('No time series data')).toBeInTheDocument();
  });

  it('handles string values in metrics data', () => {
    mockUseMetrics.mockReturnValue({
      data: {
        requests_by_endpoint: { 'GET /api/test': '100' }, // String instead of number
        response_time_percentiles: { p50: '50', p95: '100', p99: '200' },
        error_rate_by_endpoint: { 'GET /api/test': '0.01' },
        memory_usage_over_time: [],
        cpu_usage_over_time: [],
      },
      isLoading: false,
      error: null,
    } as any);

    render(<MetricsPage />, { wrapper: createWrapper() });

    expect(screen.getAllByText('100').length).toBeGreaterThan(0);
    expect(screen.getAllByText('117ms').length).toBeGreaterThan(0); // Average of 50, 100, 200
  });

  it('formats time series timestamps', () => {
    render(<MetricsPage />, { wrapper: createWrapper() });

    // Time series charts should show formatted timestamps
    expect(screen.getAllByText(/:/).length).toBeGreaterThan(0); // Time format with colons
  });
});
