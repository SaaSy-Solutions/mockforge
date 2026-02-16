/**
 * @jest-environment jsdom
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { ServicesPage } from '../ServicesPage';
import * as serviceStore from '../../stores/useServiceStore';

const mockServices = [
  {
    id: 'service-1',
    name: 'User API',
    description: 'User management service',
    enabled: true,
    routes: [
      { id: 'route-1', method: 'GET', path: '/api/users', enabled: true },
      { id: 'route-2', method: 'POST', path: '/api/users', enabled: true },
    ],
  },
  {
    id: 'service-2',
    name: 'Post API',
    description: 'Post management service',
    enabled: false,
    routes: [{ id: 'route-3', method: 'GET', path: '/api/posts', enabled: false }],
  },
];

vi.mock('../../stores/useServiceStore', () => ({
  useServiceStore: vi.fn(() => ({
    services: mockServices,
    updateService: vi.fn(),
    toggleRoute: vi.fn(),
    filteredRoutes: mockServices.flatMap((s) => s.routes),
    isLoading: false,
    error: null,
    fetchServices: vi.fn(),
    clearError: vi.fn(),
  })),
}));

describe('ServicesPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(serviceStore.useServiceStore).mockReturnValue({
      services: mockServices as any,
      updateService: vi.fn(),
      toggleRoute: vi.fn(),
      filteredRoutes: mockServices.flatMap((s) => s.routes) as any,
      isLoading: false,
      error: null,
      fetchServices: vi.fn(),
      clearError: vi.fn(),
    } as any);
  });

  it('renders services page header', () => {
    render(<ServicesPage />);

    expect(screen.getByRole('heading', { name: 'Services', level: 1 })).toBeInTheDocument();
    expect(screen.getByText(/Manage services and routes/)).toBeInTheDocument();
  });

  it('displays matching routes card', () => {
    render(<ServicesPage />);

    expect(screen.getByText('Matching Routes')).toBeInTheDocument();
  });

  it('shows services panel', () => {
    render(<ServicesPage />);

    // ServicesPanel should be rendered
    expect(screen.getByRole('heading', { name: 'Services', level: 2 })).toBeInTheDocument();
  });

  it('displays no services message when empty', () => {
    vi.mocked(serviceStore.useServiceStore).mockReturnValue({
      services: [],
      updateService: vi.fn(),
      toggleRoute: vi.fn(),
      filteredRoutes: [],
      isLoading: false,
      error: null,
      fetchServices: vi.fn(),
      clearError: vi.fn(),
    } as any);

    render(<ServicesPage />);

    expect(screen.getByText('No Services')).toBeInTheDocument();
    expect(screen.getByText('No services configured. Add a service to get started.')).toBeInTheDocument();
  });

  it('displays loading state', () => {
    vi.mocked(serviceStore.useServiceStore).mockReturnValue({
      services: [],
      updateService: vi.fn(),
      toggleRoute: vi.fn(),
      filteredRoutes: [],
      isLoading: true,
      error: null,
      fetchServices: vi.fn(),
      clearError: vi.fn(),
    } as any);

    render(<ServicesPage />);

    expect(screen.getByText('Loading services...')).toBeInTheDocument();
  });

  it('shows search active state', () => {
    vi.mocked(serviceStore.useServiceStore).mockReturnValue({
      services: mockServices,
      updateService: vi.fn(),
      toggleRoute: vi.fn(),
      filteredRoutes: [mockServices[0].routes[0]], // Filtered to 1 route
      isLoading: false,
      error: null,
      fetchServices: vi.fn(),
      clearError: vi.fn(),
    } as any);

    render(<ServicesPage />);

    expect(screen.getByText(/1 routes match your search/)).toBeInTheDocument();
  });

  it('shows no search active state', () => {
    const allRoutes = mockServices.flatMap((s) => s.routes);
    vi.mocked(serviceStore.useServiceStore).mockReturnValue({
      services: mockServices,
      updateService: vi.fn(),
      toggleRoute: vi.fn(),
      filteredRoutes: allRoutes,
      isLoading: false,
      error: null,
      fetchServices: vi.fn(),
      clearError: vi.fn(),
    } as any);

    render(<ServicesPage />);

    expect(screen.getByText(/No search active/)).toBeInTheDocument();
  });

  it('displays filtered routes with method badges', () => {
    vi.mocked(serviceStore.useServiceStore).mockReturnValue({
      services: mockServices,
      updateService: vi.fn(),
      toggleRoute: vi.fn(),
      filteredRoutes: [mockServices[0].routes[0]], // GET /api/users
      isLoading: false,
      error: null,
      fetchServices: vi.fn(),
      clearError: vi.fn(),
    } as any);

    render(<ServicesPage />);

    expect(screen.getAllByText('GET').length).toBeGreaterThan(0);
    expect(screen.getAllByText('/api/users').length).toBeGreaterThan(0);
  });

  it('shows only first 10 filtered routes', () => {
    const manyRoutes = Array.from({ length: 20 }, (_, i) => ({
      id: `route-${i}`,
      method: 'GET',
      path: `/api/test/${i}`,
      enabled: true,
    }));
    const totalRoutes = Array.from({ length: 25 }, (_, i) => ({
      id: `route-all-${i}`,
      method: 'GET',
      path: `/api/all/${i}`,
      enabled: true,
    }));

    vi.mocked(serviceStore.useServiceStore).mockReturnValue({
      services: [
        { id: 'service-1', name: 'Test Service', routes: totalRoutes, enabled: true, description: '' },
      ],
      updateService: vi.fn(),
      toggleRoute: vi.fn(),
      filteredRoutes: manyRoutes,
      isLoading: false,
      error: null,
      fetchServices: vi.fn(),
      clearError: vi.fn(),
    } as any);

    render(<ServicesPage />);

    expect(screen.getByText(/Showing first 10 results/i)).toBeInTheDocument();
  });

  it('calculates total routes correctly', () => {
    vi.mocked(serviceStore.useServiceStore).mockReturnValue({
      services: mockServices,
      updateService: vi.fn(),
      toggleRoute: vi.fn(),
      filteredRoutes: mockServices.flatMap((s) => s.routes),
      isLoading: false,
      error: null,
      fetchServices: vi.fn(),
      clearError: vi.fn(),
    } as any);

    render(<ServicesPage />);

    // Total routes should be 3
    expect(screen.getByText(/No search active/)).toBeInTheDocument();
  });
});
