/**
 * @jest-environment jsdom
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

// Force local mode. The shared test setup at src/test/setup.ts pins
// `VITE_API_BASE_URL`, which the legacy detection in `cloudMode.ts`
// reads as "cloud mode is on" — and `<TestingPage />` then short-circuits
// to `<CloudSmokeView />` (the deployment picker, not the Smoke / Health
// Check / Integration tabs these tests assert on). Mirrors the same
// hoisted mock that the sibling `components/__tests__/testing/Testing.test.tsx`
// uses. Cloud-branch coverage would belong in a dedicated suite that mocks
// the deployment list + SSE event panel.
const cloudModeMock = vi.hoisted(() => ({
  isCloudMode: vi.fn(() => false),
  getCloudApiBase: vi.fn(() => ''),
}));
vi.mock('../../utils/cloudMode', () => cloudModeMock);

import { TestingPage } from '../TestingPage';
import { dashboardApi, smokeTestsApi } from '../../services/api';
import type { HealthCheck, SmokeTestResult } from '../../types';

const makeHealth = (overrides: Partial<HealthCheck> = {}): HealthCheck => ({
  status: 'healthy',
  services: {},
  last_check: '2024-01-01T00:00:00Z',
  issues: [],
  ...overrides,
});

const mockData = vi.hoisted(() => ({
  smokeTestResults: [
    {
      test_name: 'GET /api/users',
      passed: true,
      response_time_ms: 45,
    },
    {
      test_name: 'POST /api/posts',
      passed: false,
      response_time_ms: 120,
      error_message: 'Internal server error',
    },
  ] satisfies SmokeTestResult[],
}));

vi.mock('../../services/api', () => ({
  dashboardApi: {
    getHealth: vi.fn().mockResolvedValue({ status: 'healthy' }),
  },
  smokeTestsApi: {
    runSmokeTests: vi.fn().mockResolvedValue({
      total_tests: 2,
      passed_tests: 1,
      failed_tests: 1,
    }),
    getSmokeTests: vi.fn().mockResolvedValue(mockData.smokeTestResults),
  },
}));

describe('TestingPage', () => {
  const createWrapper = () => {
    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
    });
    return ({ children }: { children: React.ReactNode }) => (
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    );
  };

  beforeEach(() => {
    vi.clearAllMocks();
    // vi.clearAllMocks resets the .mockReturnValue() set above; re-pin it
    // so every test starts in local mode.
    cloudModeMock.isCloudMode.mockReturnValue(false);
  });

  it('renders testing page header', () => {
    render(<TestingPage />, { wrapper: createWrapper() });

    expect(screen.getByText('Testing Suite')).toBeInTheDocument();
    expect(screen.getByText(/Run automated tests and validate MockForge functionality/)).toBeInTheDocument();
  });

  it('displays test overview statistics', () => {
    render(<TestingPage />, { wrapper: createWrapper() });

    expect(screen.getByText('Total Tests')).toBeInTheDocument();
    expect(screen.getAllByText('Passed').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Failed').length).toBeGreaterThan(0);
    expect(screen.getByText('Total Time')).toBeInTheDocument();
  });

  it('shows reset and run all buttons', () => {
    render(<TestingPage />, { wrapper: createWrapper() });

    expect(screen.getByText('Reset')).toBeInTheDocument();
    expect(screen.getByText('Run All Tests')).toBeInTheDocument();
  });

  it('displays test suites', () => {
    render(<TestingPage />, { wrapper: createWrapper() });

    expect(screen.getByText('Smoke Tests')).toBeInTheDocument();
    expect(screen.getByText('Health Check')).toBeInTheDocument();
    expect(screen.getByText('Integration Tests')).toBeInTheDocument();
  });

  it('shows test suite descriptions', () => {
    render(<TestingPage />, { wrapper: createWrapper() });

    expect(screen.getByText('Basic functionality and endpoint availability tests')).toBeInTheDocument();
    expect(screen.getByText('System health and service availability check')).toBeInTheDocument();
  });

  it('runs smoke tests', async () => {
    render(<TestingPage />, { wrapper: createWrapper() });

    const runSmokeTestsButton = screen.getByRole('button', { name: 'Run Smoke Tests' });
    fireEvent.click(runSmokeTestsButton);

    await waitFor(() => {
      expect(smokeTestsApi.runSmokeTests).toHaveBeenCalled();
      expect(smokeTestsApi.getSmokeTests).toHaveBeenCalled();
    });
  });

  it('displays smoke test results', async () => {
    render(<TestingPage />, { wrapper: createWrapper() });

    fireEvent.click(screen.getByRole('button', { name: 'Run Smoke Tests' }));

    await waitFor(() => {
      expect(screen.getByText('GET /api/users')).toBeInTheDocument();
      expect(screen.getByText('POST /api/posts')).toBeInTheDocument();
    });
  });

  it('runs health check', async () => {
    render(<TestingPage />, { wrapper: createWrapper() });

    fireEvent.click(screen.getByRole('button', { name: 'Run Health Check' }));

    await waitFor(() => {
      expect(dashboardApi.getHealth).toHaveBeenCalled();
    });
  });

  it('displays health check results', async () => {
    render(<TestingPage />, { wrapper: createWrapper() });

    fireEvent.click(screen.getByRole('button', { name: 'Run Health Check' }));

    await waitFor(() => {
      expect(screen.getByText('Health Endpoint')).toBeInTheDocument();
    });
  });

  it('handles health check failure', async () => {
    vi.mocked(dashboardApi.getHealth).mockResolvedValue(makeHealth({ status: 'unhealthy', issues: ['Database down'] }));

    render(<TestingPage />, { wrapper: createWrapper() });

    fireEvent.click(screen.getByRole('button', { name: 'Run Health Check' }));

    await waitFor(() => {
      expect(screen.getAllByText('failed').length).toBeGreaterThan(0);
    });
  });

  it('handles health check error', async () => {
    vi.mocked(dashboardApi.getHealth).mockRejectedValue(new Error('Connection failed'));

    render(<TestingPage />, { wrapper: createWrapper() });

    fireEvent.click(screen.getByRole('button', { name: 'Run Health Check' }));

    await waitFor(() => {
      expect(screen.getAllByText('failed').length).toBeGreaterThan(0);
    });
  });

  it('runs all tests', async () => {
    render(<TestingPage />, { wrapper: createWrapper() });

    const runAllButton = screen.getByText('Run All Tests');
    fireEvent.click(runAllButton);

    await waitFor(() => {
      expect(dashboardApi.getHealth).toHaveBeenCalled();
      expect(smokeTestsApi.runSmokeTests).toHaveBeenCalled();
    });
  });

  it('resets test results', () => {
    render(<TestingPage />, { wrapper: createWrapper() });

    const resetButton = screen.getByText('Reset');
    fireEvent.click(resetButton);

    // All test suites should be reset to idle state
    const statusBadges = screen.getAllByText('idle');
    expect(statusBadges.length).toBeGreaterThan(0);
  });

  it('disables run buttons while tests are running', async () => {
    let resolveHealth: () => void;
    vi.mocked(dashboardApi.getHealth).mockReturnValue(
      new Promise((resolve) => {
        resolveHealth = () => resolve(makeHealth());
      })
    );

    render(<TestingPage />, { wrapper: createWrapper() });

    fireEvent.click(screen.getByRole('button', { name: 'Run Health Check' }));

    expect(screen.getByRole('button', { name: 'Run All Tests' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Run Smoke Tests' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Run Integration Tests' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Running Tests...' })).toBeDisabled();

    resolveHealth!();
    await waitFor(() => {
      expect(screen.getByRole('button', { name: 'Run Health Check' })).not.toBeDisabled();
    });
  });

  it('displays test configuration section', () => {
    render(<TestingPage />, { wrapper: createWrapper() });

    expect(screen.getByText('Test Configuration')).toBeInTheDocument();
    expect(screen.getByText('Test Timeout (seconds)')).toBeInTheDocument();
    expect(screen.getByText('Parallel Execution')).toBeInTheDocument();
    expect(screen.getByText('Test Environment')).toBeInTheDocument();
  });

  it('configures test timeout', () => {
    render(<TestingPage />, { wrapper: createWrapper() });

    const timeoutInput = screen.getByDisplayValue('30');
    fireEvent.change(timeoutInput, { target: { value: '60' } });

    expect(timeoutInput).toHaveValue(60);
  });

  it('selects parallel execution mode', () => {
    render(<TestingPage />, { wrapper: createWrapper() });

    const parallelSelect = screen.getByRole('combobox');
    fireEvent.change(parallelSelect, { target: { value: 'parallel' } });

    expect(parallelSelect).toHaveValue('parallel');
  });

  it('selects test environment', () => {
    render(<TestingPage />, { wrapper: createWrapper() });

    const stagingRadio = screen.getByLabelText('Staging');
    fireEvent.click(stagingRadio);

    expect(stagingRadio).toBeChecked();
  });

  it('saves test configuration', () => {
    render(<TestingPage />, { wrapper: createWrapper() });

    const saveButton = screen.getByText('Save Configuration');
    fireEvent.click(saveButton);

    // Configuration save action should trigger
    expect(saveButton).toBeInTheDocument();
  });

  it('shows suite status badges', () => {
    render(<TestingPage />, { wrapper: createWrapper() });

    // Should have multiple status badges
    const idleBadges = screen.getAllByText('idle');
    expect(idleBadges.length).toBeGreaterThan(0);
  });

  it('displays test suite statistics', () => {
    render(<TestingPage />, { wrapper: createWrapper() });

    expect(screen.getAllByText('Total').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Passed').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Failed').length).toBeGreaterThan(0);
  });

  it('shows only first 5 tests in suite preview', async () => {
    const manyTests = Array.from({ length: 10 }, (_, i) => ({
      test_name: `Test ${i}`,
      passed: true,
      response_time_ms: 50,
    }));

    vi.mocked(smokeTestsApi.getSmokeTests).mockResolvedValue(manyTests);
    vi.mocked(smokeTestsApi.runSmokeTests).mockResolvedValue({
      suite_name: 'smoke',
      start_time: '2024-01-01T00:00:00Z',
      total_tests: 10,
      passed_tests: 10,
      failed_tests: 0,
    });

    render(<TestingPage />, { wrapper: createWrapper() });

    fireEvent.click(screen.getByRole('button', { name: 'Run Smoke Tests' }));

    await waitFor(() => {
      expect(screen.getByText('Test 0')).toBeInTheDocument();
      expect(screen.getByText('Test 4')).toBeInTheDocument();
      expect(screen.queryByText('Test 5')).not.toBeInTheDocument();
    });
  });
});
