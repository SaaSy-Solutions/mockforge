/**
 * Tests for RealityIndicator component
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { RealityIndicator } from '../RealityIndicator';
import * as useApi from '../../../hooks/useApi';

// Mock the API hooks
vi.mock('../../../hooks/useApi', () => ({
  useRealityLevel: vi.fn(),
}));

describe('RealityIndicator', () => {
  let queryClient: QueryClient;

  beforeEach(() => {
    queryClient = new QueryClient({
      defaultOptions: {
        queries: { retry: false },
      },
    });

    vi.clearAllMocks();
  });

  const renderWithProviders = (component: React.ReactElement) => {
    return render(
      <QueryClientProvider client={queryClient}>
        {component}
      </QueryClientProvider>
    );
  };

  it('renders loading state', () => {
    (useApi.useRealityLevel as any).mockReturnValue({
      data: undefined,
      isLoading: true,
    });

    renderWithProviders(<RealityIndicator />);
    const loadingBadge = document.querySelector('.animate-pulse');
    expect(loadingBadge).toBeInTheDocument();
  });

  it('displays level number', () => {
    (useApi.useRealityLevel as any).mockReturnValue({
      data: {
        level: 3,
        level_name: 'Moderate Realism',
        description: 'Some chaos, moderate latency, full intelligence',
        chaos: { enabled: true, error_rate: 0.05, delay_rate: 0.10 },
        latency: { base_ms: 125, jitter_ms: 75 },
        mockai: { enabled: true },
      },
      isLoading: false,
    });

    renderWithProviders(<RealityIndicator />);
    expect(screen.getByText(/L3/i)).toBeInTheDocument();
  });

  it('displays minimal variant', () => {
    (useApi.useRealityLevel as any).mockReturnValue({
      data: {
        level: 3,
        level_name: 'Moderate Realism',
        description: 'Some chaos, moderate latency, full intelligence',
        chaos: { enabled: true, error_rate: 0.05, delay_rate: 0.10 },
        latency: { base_ms: 125, jitter_ms: 75 },
        mockai: { enabled: true },
      },
      isLoading: false,
    });

    renderWithProviders(<RealityIndicator variant="minimal" />);
    expect(screen.getByText('3')).toBeInTheDocument();
  });

  it('displays level name when showLabel is true', () => {
    (useApi.useRealityLevel as any).mockReturnValue({
      data: {
        level: 3,
        level_name: 'Moderate Realism',
        description: 'Some chaos, moderate latency, full intelligence',
        chaos: { enabled: true, error_rate: 0.05, delay_rate: 0.10 },
        latency: { base_ms: 125, jitter_ms: 75 },
        mockai: { enabled: true },
      },
      isLoading: false,
    });

    renderWithProviders(<RealityIndicator showLabel />);
    expect(screen.getAllByText(/Moderate Realism/i).length).toBeGreaterThan(0);
  });
});
