/**
 * Tests for RealitySlider component
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { RealitySlider } from '../RealitySlider';
import * as useApi from '../../../hooks/useApi';

// Mock the API hooks
vi.mock('../../../hooks/useApi', () => ({
  useRealityLevel: vi.fn(),
  useSetRealityLevel: vi.fn(),
}));

describe('RealitySlider', () => {
  let queryClient: QueryClient;

  beforeEach(() => {
    queryClient = new QueryClient({
      defaultOptions: {
        queries: { retry: false },
        mutations: { retry: false },
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

    (useApi.useSetRealityLevel as any).mockReturnValue({
      mutate: vi.fn(),
      isPending: false,
    });

    const { container } = renderWithProviders(<RealitySlider />);
    expect(container.querySelector('.animate-pulse')).toBeInTheDocument();
  });

  it('renders current reality level', () => {
    (useApi.useRealityLevel as any).mockReturnValue({
      data: {
        level: 3,
        level_name: 'Moderate Realism',
        description: 'Some chaos, moderate latency, full intelligence',
        chaos: {
          enabled: true,
          error_rate: 0.05,
          delay_rate: 0.10,
        },
        latency: {
          base_ms: 125,
          jitter_ms: 75,
        },
        mockai: {
          enabled: true,
        },
      },
      isLoading: false,
    });

    (useApi.useSetRealityLevel as any).mockReturnValue({
      mutate: vi.fn(),
      isPending: false,
    });

    renderWithProviders(<RealitySlider />);
    expect(screen.getAllByText(/Moderate Realism/i).length).toBeGreaterThan(0);
    expect(screen.getByText(/Level 3/i)).toBeInTheDocument();
  });

  it('displays configuration details', () => {
    (useApi.useRealityLevel as any).mockReturnValue({
      data: {
        level: 3,
        level_name: 'Moderate Realism',
        description: 'Some chaos, moderate latency, full intelligence',
        chaos: {
          enabled: true,
          error_rate: 0.05,
          delay_rate: 0.10,
        },
        latency: {
          base_ms: 125,
          jitter_ms: 75,
        },
        mockai: {
          enabled: true,
        },
      },
      isLoading: false,
    });

    (useApi.useSetRealityLevel as any).mockReturnValue({
      mutate: vi.fn(),
      isPending: false,
    });

    renderWithProviders(<RealitySlider />);
    expect(screen.getAllByText(/5% errors/i).length).toBeGreaterThan(0);
    expect(screen.getByText(/125ms/i)).toBeInTheDocument();
    expect(screen.getAllByText(/Enabled/i).length).toBeGreaterThan(0);
  });

  it('renders compact mode', () => {
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

    (useApi.useSetRealityLevel as any).mockReturnValue({
      mutate: vi.fn(),
      isPending: false,
    });

    renderWithProviders(<RealitySlider compact />);
    expect(screen.getByText(/Level 3/i)).toBeInTheDocument();
  });

  it('renders all level indicators', () => {
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

    (useApi.useSetRealityLevel as any).mockReturnValue({
      mutate: vi.fn(),
      isPending: false,
    });

    renderWithProviders(<RealitySlider />);

    // Check that all 5 level indicators are present
    expect(screen.getAllByText('1').length).toBeGreaterThan(0);
    expect(screen.getAllByText('2').length).toBeGreaterThan(0);
    expect(screen.getAllByText('3').length).toBeGreaterThan(0);
    expect(screen.getAllByText('4').length).toBeGreaterThan(0);
    expect(screen.getAllByText('5').length).toBeGreaterThan(0);
  });
});
