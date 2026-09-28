/**
 * @jest-environment jsdom
 */

import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { ServiceToggleCard } from '../../services/ServiceToggleCard';
import type { ServiceInfo } from '../../../types';

describe('ServiceToggleCard', () => {
  const mockService: ServiceInfo = {
    id: 'service-1',
    name: 'Test Service',
    baseUrl: 'http://localhost:3000',
    enabled: true,
    createdAt: '2024-01-01T00:00:00Z',
    updatedAt: '2024-01-01T00:00:00Z',
    routes: [
      {
        id: 'route-1',
        method: 'GET',
        path: '/api/test',
        statusCode: 200,
        enabled: true,
      },
      {
        id: 'route-2',
        method: 'POST',
        path: '/api/create',
        statusCode: 200,
        enabled: false,
      },
    ],
  };

  it('renders service name', () => {
    render(
      <ServiceToggleCard
        service={mockService}
        onToggleService={vi.fn()}
        onToggleRoute={vi.fn()}
        onToggleExpanded={vi.fn()}
      />
    );

    expect(screen.getByText('Test Service')).toBeInTheDocument();
  });

  it('shows enabled/disabled state', () => {
    render(
      <ServiceToggleCard
        service={mockService}
        onToggleService={vi.fn()}
        onToggleRoute={vi.fn()}
        onToggleExpanded={vi.fn()}
      />
    );

    const toggle = screen.getByRole('switch');
    expect(toggle).toBeChecked();
  });

  it('calls onToggleService when toggled', () => {
    const mockToggle = vi.fn();

    render(
      <ServiceToggleCard
        service={mockService}
        onToggleService={mockToggle}
        onToggleRoute={vi.fn()}
        onToggleExpanded={vi.fn()}
      />
    );

    const toggle = screen.getByRole('switch');
    fireEvent.click(toggle);

    expect(mockToggle).toHaveBeenCalledWith('service-1', false);
  });

  it('displays route count', () => {
    render(
      <ServiceToggleCard
        service={mockService}
        onToggleService={vi.fn()}
        onToggleRoute={vi.fn()}
        onToggleExpanded={vi.fn()}
      />
    );

    // Should show 1 out of 2 routes enabled
    expect(screen.getByText(/1/)).toBeInTheDocument();
    expect(screen.getByText(/2/)).toBeInTheDocument();
  });
});
