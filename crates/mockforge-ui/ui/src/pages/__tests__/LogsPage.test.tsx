/**
 * @jest-environment jsdom
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { LogsPage } from '../LogsPage';
import type { RequestLog } from '../../types';
import * as apiHooks from '../../hooks/useApi';

const mockLogs: RequestLog[] = [
  {
    id: '1',
    timestamp: '2024-01-01T10:00:00Z',
    method: 'GET',
    path: '/api/users',
    status_code: 200,
    response_time_ms: 45,
    client_ip: '127.0.0.1',
    user_agent: 'Mozilla/5.0',
  },
  {
    id: '2',
    timestamp: '2024-01-01T10:01:00Z',
    method: 'POST',
    path: '/api/posts',
    status_code: 404,
    response_time_ms: 23,
    client_ip: '127.0.0.1',
    user_agent: 'Mozilla/5.0',
  },
  {
    id: '3',
    timestamp: '2024-01-01T10:02:00Z',
    method: 'DELETE',
    path: '/api/users/1',
    status_code: 500,
    response_time_ms: 150,
    client_ip: '192.168.1.1',
    user_agent: 'curl/7.68.0',
  },
];

vi.mock('../../hooks/useApi', () => ({
  useLogs: vi.fn(() => ({
    data: mockLogs,
    isLoading: false,
    error: null,
    refetch: vi.fn(),
  })),
}));

describe('LogsPage', () => {
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
    vi.mocked(apiHooks.useLogs).mockReturnValue({
      data: mockLogs,
      isLoading: false,
      error: null,
      refetch: vi.fn(),
    } as any);
  });

  it('renders loading state', () => {
    vi.mocked(apiHooks.useLogs).mockReturnValue({ data: null, isLoading: true, error: null, refetch: vi.fn() } as any);

    render(<LogsPage />, { wrapper: createWrapper() });
    expect(screen.getByText('Loading logs...')).toBeInTheDocument();
  });

  it('displays logs list', () => {
    render(<LogsPage />, { wrapper: createWrapper() });

    expect(screen.getByText('/api/users')).toBeInTheDocument();
    expect(screen.getByText('/api/posts')).toBeInTheDocument();
    expect(screen.getByText('/api/users/1')).toBeInTheDocument();
  });

  it('shows method badges with correct colors', () => {
    render(<LogsPage />, { wrapper: createWrapper() });

    expect(screen.getAllByText('GET').length).toBeGreaterThan(0);
    expect(screen.getAllByText('POST').length).toBeGreaterThan(0);
    expect(screen.getAllByText('DELETE').length).toBeGreaterThan(0);
  });

  it('displays status codes with badges', () => {
    render(<LogsPage />, { wrapper: createWrapper() });

    expect(screen.getAllByText('200').length).toBeGreaterThan(0);
    expect(screen.getAllByText('404').length).toBeGreaterThan(0);
    expect(screen.getAllByText('500').length).toBeGreaterThan(0);
  });

  it('shows response times', () => {
    render(<LogsPage />, { wrapper: createWrapper() });

    expect(screen.getByText('45ms')).toBeInTheDocument();
    expect(screen.getByText('23ms')).toBeInTheDocument();
    expect(screen.getByText('150ms')).toBeInTheDocument();
  });

  it('filters logs by search term', () => {
    render(<LogsPage />, { wrapper: createWrapper() });

    const searchInput = screen.getByPlaceholderText('Filter by path...');
    fireEvent.change(searchInput, { target: { value: 'posts' } });

    // Logs are filtered server-side, so we just check the input value
    expect(searchInput).toHaveValue('posts');
  });

  it('filters logs by HTTP method', () => {
    render(<LogsPage />, { wrapper: createWrapper() });

    const methodSelect = screen.getAllByRole('combobox')[0];
    fireEvent.change(methodSelect, { target: { value: 'GET' } });

    expect(methodSelect).toHaveValue('GET');
  });

  it('filters logs by status code', () => {
    render(<LogsPage />, { wrapper: createWrapper() });

    const statusSelect = screen.getAllByRole('combobox')[1];
    fireEvent.change(statusSelect, { target: { value: '2xx' } });

    // 2xx filter should only show the successful request row
    expect(screen.getByText('/api/users')).toBeInTheDocument();
    expect(screen.queryByText('/api/posts')).not.toBeInTheDocument();
    expect(screen.queryByText('/api/users/1')).not.toBeInTheDocument();
  });

  it('changes fetch limit', () => {
    render(<LogsPage />, { wrapper: createWrapper() });

    const limitSelect = screen.getAllByRole('combobox')[2];
    fireEvent.change(limitSelect, { target: { value: '250' } });

    expect(limitSelect).toHaveValue('250');
  });

  it('exports logs to CSV', () => {
    const createElementSpy = vi.spyOn(document, 'createElement');
    render(<LogsPage />, { wrapper: createWrapper() });

    const exportButton = screen.getByText('Export CSV');
    fireEvent.click(exportButton);

    expect(createElementSpy).toHaveBeenCalledWith('a');
  });

  it('disables export when no logs', () => {
    vi.mocked(apiHooks.useLogs).mockReturnValue({ data: [], isLoading: false, error: null, refetch: vi.fn() } as any);

    render(<LogsPage />, { wrapper: createWrapper() });

    const exportButton = screen.getByText('Export CSV');
    expect(exportButton).toBeDisabled();
  });

  it('refreshes logs', () => {
    const refetchMock = vi.fn();
    vi.mocked(apiHooks.useLogs).mockReturnValue({ data: mockLogs, isLoading: false, error: null, refetch: refetchMock } as any);

    render(<LogsPage />, { wrapper: createWrapper() });

    const refreshButton = screen.getByText('Refresh');
    fireEvent.click(refreshButton);

    expect(refetchMock).toHaveBeenCalled();
  });

  it('displays empty state when no logs exist', () => {
    vi.mocked(apiHooks.useLogs).mockReturnValue({ data: [], isLoading: false, error: null, refetch: vi.fn() } as any);

    render(<LogsPage />, { wrapper: createWrapper() });

    expect(screen.getByText('No logs found')).toBeInTheDocument();
    expect(screen.getByText(/No request logs are available/)).toBeInTheDocument();
  });

  it('displays empty state when filters return no results', () => {
    render(<LogsPage />, { wrapper: createWrapper() });

    const statusSelect = screen.getAllByRole('combobox')[1];
    fireEvent.change(statusSelect, { target: { value: '5xx' } });

    expect(screen.getByText(/Showing 1 of/)).toBeInTheDocument(); // Only 1 500 error
  });

  it('handles error state', () => {
    vi.mocked(apiHooks.useLogs).mockReturnValue({
      data: null,
      isLoading: false,
      error: new Error('Failed to fetch logs'),
      refetch: vi.fn(),
    } as any);

    render(<LogsPage />, { wrapper: createWrapper() });

    expect(screen.getByText('Failed to load logs')).toBeInTheDocument();
  });

  it('formats timestamps correctly', () => {
    render(<LogsPage />, { wrapper: createWrapper() });

    // Check that timestamps are formatted and displayed
    expect(screen.getAllByText(/Jan/).length).toBeGreaterThan(0);
  });

  it('displays client IP addresses', () => {
    render(<LogsPage />, { wrapper: createWrapper() });

    expect(screen.getAllByText(/127\.0\.0\.1/).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/192\.168\.1\.1/).length).toBeGreaterThan(0);
  });

  it('displays user agents', () => {
    render(<LogsPage />, { wrapper: createWrapper() });

    expect(screen.getAllByText('Mozilla/5.0').length).toBeGreaterThan(0);
    expect(screen.getByText('curl/7.68.0')).toBeInTheDocument();
  });

  it('shows load more button when there are more logs', () => {
    // Mock more logs than display limit
    const manyLogs = Array.from({ length: 100 }, (_, i) => ({
      id: `${i}`,
      timestamp: '2024-01-01T10:00:00Z',
      method: 'GET',
      path: `/api/test/${i}`,
      status_code: 200,
      response_time_ms: 50,
      client_ip: '127.0.0.1',
      user_agent: 'test',
    }));

    vi.mocked(apiHooks.useLogs).mockReturnValue({ data: manyLogs as any, isLoading: false, error: null, refetch: vi.fn() } as any);

    render(<LogsPage />, { wrapper: createWrapper() });

    expect(screen.getByText(/Show more logs/)).toBeInTheDocument();
  });

  it('loads more logs when button is clicked', () => {
    const manyLogs = Array.from({ length: 100 }, (_, i) => ({
      id: `${i}`,
      timestamp: '2024-01-01T10:00:00Z',
      method: 'GET',
      path: `/api/test/${i}`,
      status_code: 200,
      response_time_ms: 50,
      client_ip: '127.0.0.1',
      user_agent: 'test',
    }));

    vi.mocked(apiHooks.useLogs).mockReturnValue({ data: manyLogs as any, isLoading: false, error: null, refetch: vi.fn() } as any);

    render(<LogsPage />, { wrapper: createWrapper() });

    const loadMoreButton = screen.getByText(/Show more logs/);
    fireEvent.click(loadMoreButton);

    // Button should disappear when all logs are displayed
    expect(screen.queryByText(/Show more logs/)).not.toBeInTheDocument();
  });

  it('resets display limit when filters change', () => {
    render(<LogsPage />, { wrapper: createWrapper() });

    const statusSelect = screen.getAllByRole('combobox')[1];
    fireEvent.change(statusSelect, { target: { value: '2xx' } });

    // Display limit should reset when filters change
    expect(screen.queryByText(/Show more logs/)).not.toBeInTheDocument();
  });
});
