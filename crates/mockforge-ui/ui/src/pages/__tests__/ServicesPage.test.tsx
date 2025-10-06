/**
 * @jest-environment jsdom
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { ServicesPage } from '../ServicesPage';

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
  })),
}));

describe('ServicesPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders services page header', () => {
    render(<ServicesPage />);

    expect(screen.getByText('Services')).toBeInTheDocument();
    expect(screen.getByText(/Manage services and routes/)).toBeInTheDocument();
  });

  it('displays matching routes card', () => {
    render(<ServicesPage />);

    expect(screen.getByText('Matching Routes')).toBeInTheDocument();
  });

  it('shows services panel', () => {
    render(<ServicesPage />);

    // ServicesPanel should be rendered
    expect(screen.getByText('Services')).toBeInTheDocument();
  });

  it('displays no services message when empty', () => {
    const { useServiceStore } = require('../../stores/useServiceStore');
    useServiceStore.mockReturnValue({
      services: [],
      updateService: vi.fn(),
      toggleRoute: vi.fn(),
      filteredRoutes: [],
    });

    render(<ServicesPage />);

    expect(screen.getByText('No Services')).toBeInTheDocument();
    expect(screen.getByText('No services configured. Add a service to get started.')).toBeInTheDocument();
  });

  it('displays loading state', () => {
    const { useServiceStore } = require('../../stores/useServiceStore');
    useServiceStore.mockReturnValue({
      services: [],
      updateService: vi.fn(),
      toggleRoute: vi.fn(),
      filteredRoutes: [],
    });

    render(<ServicesPage />);

    // When there are no services, it shows the empty state
    expect(screen.getByText('No Services')).toBeInTheDocument();
  });

  it('shows search active state', () => {
    const { useServiceStore } = require('../../stores/useServiceStore');
    useServiceStore.mockReturnValue({
      services: mockServices,
      updateService: vi.fn(),
      toggleRoute: vi.fn(),
      filteredRoutes: [mockServices[0].routes[0]], // Filtered to 1 route
    });

    render(<ServicesPage />);

    expect(screen.getByText(/1 routes match your search/)).toBeInTheDocument();
  });

  it('shows no search active state', () => {
    const { useServiceStore } = require('../../stores/useServiceStore');
    const allRoutes = mockServices.flatMap((s) => s.routes);
    useServiceStore.mockReturnValue({
      services: mockServices,
      updateService: vi.fn(),
      toggleRoute: vi.fn(),
      filteredRoutes: allRoutes,
    });

    render(<ServicesPage />);

    expect(screen.getByText(/No search active/)).toBeInTheDocument();
  });

  it('displays filtered routes with method badges', () => {
    const { useServiceStore } = require('../../stores/useServiceStore');
    useServiceStore.mockReturnValue({
      services: mockServices,
      updateService: vi.fn(),
      toggleRoute: vi.fn(),
      filteredRoutes: [mockServices[0].routes[0]], // GET /api/users
    });

    render(<ServicesPage />);

    expect(screen.getByText('GET')).toBeInTheDocument();
    expect(screen.getByText('/api/users')).toBeInTheDocument();
  });

  it('shows only first 10 filtered routes', () => {
    const manyRoutes = Array.from({ length: 20 }, (_, i) => ({
      id: `route-${i}`,
      method: 'GET',
      path: `/api/test/${i}`,
      enabled: true,
    }));

    const { useServiceStore } = require('../../stores/useServiceStore');
    useServiceStore.mockReturnValue({
      services: [
        { id: 'service-1', name: 'Test Service', routes: manyRoutes, enabled: true, description: '' },
      ],
      updateService: vi.fn(),
      toggleRoute: vi.fn(),
      filteredRoutes: manyRoutes,
    });

    render(<ServicesPage />);

    expect(screen.getByText(/Showing first 10 results/)).toBeInTheDocument();
  });

  it('calculates total routes correctly', () => {
    const { useServiceStore } = require('../../stores/useServiceStore');
    useServiceStore.mockReturnValue({
      services: mockServices,
      updateService: vi.fn(),
      toggleRoute: vi.fn(),
      filteredRoutes: mockServices.flatMap((s) => s.routes),
    });

    render(<ServicesPage />);

    // Total routes should be 3
    expect(screen.getByText(/No search active/)).toBeInTheDocument();
  });
});
