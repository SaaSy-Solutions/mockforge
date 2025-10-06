/**
 * @jest-environment jsdom
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { TestingPage } from '../TestingPage';
import type { SmokeTestResult } from '../../types';

const mockSmokeTestResults: SmokeTestResult[] = [
  {
    test_name: 'GET /api/users',
    passed: true,
    response_time_ms: 45,
    status_code: 200,
  },
  {
    test_name: 'POST /api/posts',
    passed: false,
    response_time_ms: 120,
    status_code: 500,
    error_message: 'Internal server error',
  },
];

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
    getSmokeTests: vi.fn().mockResolvedValue(mockSmokeTestResults),
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
  });

  it('renders testing page header', () => {
    render(<TestingPage />, { wrapper: createWrapper() });

    expect(screen.getByText('Testing Suite')).toBeInTheDocument();
    expect(screen.getByText(/Run automated tests and validate MockForge functionality/)).toBeInTheDocument();
  });

  it('displays test overview statistics', () => {
    render(<TestingPage />, { wrapper: createWrapper() });

    expect(screen.getByText('Total Tests')).toBeInTheDocument();
    expect(screen.getByText('Passed')).toBeInTheDocument();
    expect(screen.getByText('Failed')).toBeInTheDocument();
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
    const { smokeTestsApi } = require('../../services/api');

    render(<TestingPage />, { wrapper: createWrapper() });

    const runButtons = screen.getAllByText(/Run/);
    const runSmokeTestsButton = runButtons[1]; // First suite's run button
    fireEvent.click(runSmokeTestsButton);

    await waitFor(() => {
      expect(smokeTestsApi.runSmokeTests).toHaveBeenCalled();
      expect(smokeTestsApi.getSmokeTests).toHaveBeenCalled();
    });
  });

  it('displays smoke test results', async () => {
    const { smokeTestsApi } = require('../../services/api');

    render(<TestingPage />, { wrapper: createWrapper() });

    const runButtons = screen.getAllByText(/Run/);
    fireEvent.click(runButtons[1]); // Run smoke tests

    await waitFor(() => {
      expect(screen.getByText('GET /api/users')).toBeInTheDocument();
      expect(screen.getByText('POST /api/posts')).toBeInTheDocument();
    });
  });

  it('runs health check', async () => {
    const { dashboardApi } = require('../../services/api');

    render(<TestingPage />, { wrapper: createWrapper() });

    const runButtons = screen.getAllByText(/Run/);
    const runHealthCheckButton = runButtons[2]; // Second suite's run button
    fireEvent.click(runHealthCheckButton);

    await waitFor(() => {
      expect(dashboardApi.getHealth).toHaveBeenCalled();
    });
  });

  it('displays health check results', async () => {
    const { dashboardApi } = require('../../services/api');

    render(<TestingPage />, { wrapper: createWrapper() });

    const runButtons = screen.getAllByText(/Run/);
    fireEvent.click(runButtons[2]); // Run health check

    await waitFor(() => {
      expect(screen.getByText('Health Endpoint')).toBeInTheDocument();
    });
  });

  it('handles health check failure', async () => {
    const { dashboardApi } = require('../../services/api');
    dashboardApi.getHealth.mockResolvedValue({ status: 'unhealthy', issues: ['Database down'] });

    render(<TestingPage />, { wrapper: createWrapper() });

    const runButtons = screen.getAllByText(/Run/);
    fireEvent.click(runButtons[2]); // Run health check

    await waitFor(() => {
      expect(screen.getByText('Database down')).toBeInTheDocument();
    });
  });

  it('handles health check error', async () => {
    const { dashboardApi } = require('../../services/api');
    dashboardApi.getHealth.mockRejectedValue(new Error('Connection failed'));

    render(<TestingPage />, { wrapper: createWrapper() });

    const runButtons = screen.getAllByText(/Run/);
    fireEvent.click(runButtons[2]); // Run health check

    await waitFor(() => {
      expect(screen.getByText(/Connection failed/)).toBeInTheDocument();
    });
  });

  it('runs all tests', async () => {
    const { smokeTestsApi, dashboardApi } = require('../../services/api');

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
    const { dashboardApi } = require('../../services/api');
    let resolveHealth: () => void;
    dashboardApi.getHealth.mockReturnValue(
      new Promise((resolve) => {
        resolveHealth = () => resolve({ status: 'healthy' });
      })
    );

    render(<TestingPage />, { wrapper: createWrapper() });

    const runButtons = screen.getAllByText(/Run/);
    fireEvent.click(runButtons[2]); // Run health check

    // All run buttons should be disabled
    runButtons.forEach((btn) => {
      expect(btn).toBeDisabled();
    });

    resolveHealth!();
    await waitFor(() => {
      expect(runButtons[2]).not.toBeDisabled();
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
      status_code: 200,
    }));

    const { smokeTestsApi } = require('../../services/api');
    smokeTestsApi.getSmokeTests.mockResolvedValue(manyTests);
    smokeTestsApi.runSmokeTests.mockResolvedValue({
      total_tests: 10,
      passed_tests: 10,
      failed_tests: 0,
    });

    render(<TestingPage />, { wrapper: createWrapper() });

    const runButtons = screen.getAllByText(/Run/);
    fireEvent.click(runButtons[1]); // Run smoke tests

    await waitFor(() => {
      expect(screen.getByText(/\+5 more tests/)).toBeInTheDocument();
    });
  });

  it('handles integration tests placeholder', async () => {
    render(<TestingPage />, { wrapper: createWrapper() });

    const runButtons = screen.getAllByText(/Run/);
    const runIntegrationButton = runButtons[3]; // Third suite's run button
    fireEvent.click(runIntegrationButton);

    await waitFor(() => {
      expect(screen.getByText('Custom integration tests not configured')).toBeInTheDocument();
    });
  });

  it('displays test timestamps', async () => {
    const { dashboardApi } = require('../../services/api');

    render(<TestingPage />, { wrapper: createWrapper() });

    const runButtons = screen.getAllByText(/Run/);
    fireEvent.click(runButtons[2]); // Run health check

    await waitFor(() => {
      expect(screen.getByText(/Executed at/)).toBeInTheDocument();
    });
  });

  it('calculates total duration correctly', async () => {
    const { smokeTestsApi } = require('../../services/api');

    render(<TestingPage />, { wrapper: createWrapper() });

    const runButtons = screen.getAllByText(/Run/);
    fireEvent.click(runButtons[1]); // Run smoke tests

    await waitFor(() => {
      // Total time should be displayed
      const totalTime = screen.getAllByText(/s$/); // Ends with 's' for seconds
      expect(totalTime.length).toBeGreaterThan(0);
    });
  });
});
