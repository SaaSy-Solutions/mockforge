/**
 * @jest-environment jsdom
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { WorkflowValidator } from '../WorkflowValidator';
import { useServiceStore } from '../../../stores/useServiceStore';
import { useAuthStore } from '../../../stores/useAuthStore';

const mockState = vi.hoisted(() => ({
  services: [
    { id: 'service-1', name: 'Test Service', enabled: true, routes: [{ method: 'GET', path: '/test', enabled: true }] },
    { id: 'service-2', name: 'Aux Service', enabled: false, routes: [{ method: 'POST', path: '/aux', enabled: true }] },
  ],
  fixtures: [
    { id: 'fixture-1', name: 'test.json', content: '{"test": true}' },
  ],
}));

vi.mock('../../../stores/useServiceStore', () => ({
  useServiceStore: vi.fn(() => ({
    services: mockState.services,
    updateService: vi.fn((serviceId: string, updates: { enabled?: boolean }) => {
      const service = mockState.services.find(s => s.id === serviceId);
      if (service && typeof updates.enabled === 'boolean') {
        service.enabled = updates.enabled;
      }
    }),
    toggleRoute: vi.fn((serviceId: string, routeId: string, enabled: boolean) => {
      const service = mockState.services.find(s => s.id === serviceId);
      if (!service) return;
      const route = service.routes.find(r => (r.method ? `${r.method}-${r.path}` : r.path) === routeId);
      if (route) {
        route.enabled = enabled;
      }
    }),
  })),
}));

vi.mock('../../../stores/useFixtureStore', () => ({
  useFixtureStore: vi.fn(() => ({
    fixtures: mockState.fixtures,
    updateFixture: vi.fn((fixtureId: string, content: string) => {
      const fixture = mockState.fixtures.find(f => f.id === fixtureId);
      if (fixture) {
        fixture.content = content;
      }
    }),
    renameFixture: vi.fn((fixtureId: string, name: string) => {
      const fixture = mockState.fixtures.find(f => f.id === fixtureId);
      if (fixture) {
        fixture.name = name;
      }
    }),
    generateDiff: vi.fn(() => ({ changes: [{ type: 'modified', line: 1 }] })),
  })),
}));

vi.mock('../../../stores/useLogStore', () => ({
  useLogStore: vi.fn(() => ({
    logs: [
      { method: 'GET', path: '/api/users', timestamp: new Date().toISOString() },
      { method: 'POST', path: '/api/posts', timestamp: new Date().toISOString() },
    ],
    filteredLogs: [{ method: 'GET', path: '/api/users', timestamp: new Date().toISOString() }],
    setFilter: vi.fn(),
    clearFilter: vi.fn(),
  })),
}));

vi.mock('../../../stores/useMetricsStore', () => ({
  useMetricsStore: vi.fn(() => ({
    latencyMetrics: [
      { endpoint: '/test', p50: 45, p95: 120, p99: 250, histogram: [1, 2, 3] },
    ],
    failureMetrics: [
      { endpoint: '/test', failure_rate: 0.05 },
    ],
  })),
}));

vi.mock('../../../stores/useAuthStore', () => ({
  useAuthStore: vi.fn(() => ({
    user: { username: 'admin', role: 'admin' },
    isAuthenticated: true,
    login: vi.fn().mockResolvedValue(undefined),
    logout: vi.fn(),
  })),
}));

describe('WorkflowValidator', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockState.services = [
      { id: 'service-1', name: 'Test Service', enabled: true, routes: [{ method: 'GET', path: '/test', enabled: true }] },
      { id: 'service-2', name: 'Aux Service', enabled: false, routes: [{ method: 'POST', path: '/aux', enabled: true }] },
    ];
    mockState.fixtures = [{ id: 'fixture-1', name: 'test.json', content: '{"test": true}' }];
  });

  it('renders workflow validator header', () => {
    render(<WorkflowValidator />);

    expect(screen.getByText('Power-User Workflow Validation')).toBeInTheDocument();
    expect(screen.getByText(/Testing all admin workflows/)).toBeInTheDocument();
  });

  it('renders run all tests button', () => {
    render(<WorkflowValidator />);

    expect(screen.getByText('Run All Tests')).toBeInTheDocument();
  });

  it('shows empty state initially', () => {
    render(<WorkflowValidator />);

    expect(screen.getByText('Ready to Test Workflows')).toBeInTheDocument();
    expect(screen.getByText(/Click "Run All Tests" to validate/)).toBeInTheDocument();
  });

  it('runs all workflow tests when button clicked', async () => {
    render(<WorkflowValidator />);

    const runButton = screen.getByText('Run All Tests');
    fireEvent.click(runButton);

    await waitFor(() => {
      expect(screen.getByText('Running Tests...')).toBeInTheDocument();
    });
  });

  it('displays test results after running', async () => {
    render(<WorkflowValidator />);

    fireEvent.click(screen.getByText('Run All Tests'));

    await waitFor(() => {
      expect(screen.getByText('Admin Authentication')).toBeInTheDocument();
      expect(screen.getByText('Service Toggle Management')).toBeInTheDocument();
    });
  });

  it('shows test statistics', async () => {
    render(<WorkflowValidator />);

    fireEvent.click(screen.getByText('Run All Tests'));

    await waitFor(() => {
      expect(screen.getByText('Passed')).toBeInTheDocument();
      expect(screen.getByText('Failed')).toBeInTheDocument();
      expect(screen.getByText('Total')).toBeInTheDocument();
    });
  });

  it('displays test status icons', async () => {
    render(<WorkflowValidator />);

    fireEvent.click(screen.getByText('Run All Tests'));

    await waitFor(() => {
      const statusIcons = screen.getAllByText(/[⏳🔄✅❌]/);
      expect(statusIcons.length).toBeGreaterThan(0);
    });
  });

  it('runs admin auth test successfully', async () => {
    vi.mocked(useAuthStore).mockReturnValue({
      user: { username: 'admin', role: 'admin' },
      isAuthenticated: true,
      login: vi.fn().mockResolvedValue(undefined),
      logout: vi.fn(),
    } as any);

    render(<WorkflowValidator />);

    fireEvent.click(screen.getByText('Run All Tests'));

    await waitFor(() => {
      expect(screen.getByText(/Admin login successful/)).toBeInTheDocument();
    }, { timeout: 3000 });
  });

  it('runs service management test successfully', async () => {
    render(<WorkflowValidator />);

    fireEvent.click(screen.getByText('Run All Tests'));

    await waitFor(() => {
      expect(screen.getByText(/Service enable\/disable works/)).toBeInTheDocument();
    }, { timeout: 3000 });
  });

  it('runs fixture editing test successfully', async () => {
    render(<WorkflowValidator />);

    fireEvent.click(screen.getByText('Run All Tests'));

    await waitFor(() => {
      expect(screen.getByText(/Fixture content editing works/)).toBeInTheDocument();
    }, { timeout: 3000 });
  });

  it('runs fixture diffing test successfully', async () => {
    render(<WorkflowValidator />);

    fireEvent.click(screen.getByText('Run All Tests'));

    await waitFor(() => {
      expect(screen.getByText(/Diff visualization ready/)).toBeInTheDocument();
    }, { timeout: 3000 });
  });

  it('disables run button while tests are running', async () => {
    render(<WorkflowValidator />);

    const runButton = screen.getByText('Run All Tests');
    fireEvent.click(runButton);

    await waitFor(() => {
      expect(screen.getByText('Running Tests...')).toBeDisabled();
    });
  });

  it('displays test details when available', async () => {
    render(<WorkflowValidator />);

    fireEvent.click(screen.getByText('Run All Tests'));

    await waitFor(() => {
      const checkmarks = screen.getAllByText(/✓/);
      expect(checkmarks.length).toBeGreaterThan(0);
    }, { timeout: 3000 });
  });

  it('displays error messages for failed tests', async () => {
    vi.mocked(useServiceStore).mockReturnValue({
      services: [], // No services to cause failure
      updateService: vi.fn(),
      toggleRoute: vi.fn(),
    } as any);

    render(<WorkflowValidator />);

    fireEvent.click(screen.getByText('Run All Tests'));

    await waitFor(() => {
      expect(screen.getByText(/Error:/)).toBeInTheDocument();
    }, { timeout: 3000 });
  });

  it('shows workflow descriptions', () => {
    render(<WorkflowValidator />);

    fireEvent.click(screen.getByText('Run All Tests'));

    expect(screen.getByText(/Login as admin and verify full access/)).toBeInTheDocument();
    expect(screen.getByText(/Enable\/disable services and routes/)).toBeInTheDocument();
  });

  it('counts passed tests correctly', async () => {
    render(<WorkflowValidator />);

    fireEvent.click(screen.getByText('Run All Tests'));

    await waitFor(() => {
      const passedCount = screen.getAllByText(/\d+/)[0];
      expect(Number(passedCount.textContent)).toBeGreaterThan(0);
    }, { timeout: 3000 });
  });

  it('displays all 10 workflow tests', async () => {
    render(<WorkflowValidator />);

    fireEvent.click(screen.getByText('Run All Tests'));

    await waitFor(() => {
      expect(screen.getByText('Admin Authentication')).toBeInTheDocument();
      expect(screen.getByText('Viewer Authentication')).toBeInTheDocument();
      expect(screen.getByText('Service Toggle Management')).toBeInTheDocument();
      expect(screen.getByText('Fixture Content Management')).toBeInTheDocument();
      expect(screen.getByText('Fixture Diff Visualization')).toBeInTheDocument();
      expect(screen.getByText('Live Log Monitoring')).toBeInTheDocument();
      expect(screen.getByText('Performance Metrics Analysis')).toBeInTheDocument();
      expect(screen.getByText('Bulk Service Operations')).toBeInTheDocument();
      expect(screen.getByText('Search and Filtering')).toBeInTheDocument();
      expect(screen.getByText('Role-based Feature Access')).toBeInTheDocument();
    });
  });

  it('shows pending status before tests run', async () => {
    render(<WorkflowValidator />);

    fireEvent.click(screen.getByText('Run All Tests'));

    // Immediately after click, tests should be pending
    expect(screen.getAllByText('PENDING').length).toBeGreaterThan(0);
  });

  it('shows running status during test execution', async () => {
    render(<WorkflowValidator />);

    fireEvent.click(screen.getByText('Run All Tests'));

    await waitFor(() => {
      expect(screen.getAllByText(/RUNNING/i).length).toBeGreaterThan(0);
    }, { timeout: 500 });
  });
});
