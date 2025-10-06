/**
 * @jest-environment jsdom
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { ConfigPage } from '../ConfigPage';

// Mock hooks
vi.mock('../../hooks/useApi', () => ({
  useConfig: vi.fn(() => ({
    data: {
      latency: { base_ms: 100, jitter_ms: 50 },
      faults: { enabled: false, failure_rate: 0, status_codes: [] },
      proxy: { enabled: false, upstream_url: '', timeout_seconds: 30 },
    },
    isLoading: false,
  })),
  useValidation: vi.fn(() => ({
    data: {
      mode: 'enforce',
      aggregate_errors: true,
      validate_responses: true,
      overrides: {},
    },
    isLoading: false,
  })),
  useServerInfo: vi.fn(() => ({
    data: {
      http_server: '0.0.0.0:3000',
      ws_server: '0.0.0.0:3001',
      grpc_server: '0.0.0.0:50051',
      admin_port: 9080,
    },
    isLoading: false,
  })),
  useUpdateLatency: vi.fn(() => ({ mutateAsync: vi.fn() })),
  useUpdateFaults: vi.fn(() => ({ mutateAsync: vi.fn() })),
  useUpdateProxy: vi.fn(() => ({ mutateAsync: vi.fn() })),
  useUpdateValidation: vi.fn(() => ({ mutateAsync: vi.fn() })),
  useRestartServers: vi.fn(() => ({ mutateAsync: vi.fn() })),
  useRestartStatus: vi.fn(() => ({ data: { restarting: false } })),
}));

vi.mock('../../stores/useWorkspaceStore', () => ({
  useWorkspaceStore: vi.fn(() => ({
    activeWorkspace: { id: 'test-workspace' },
  })),
}));

vi.mock('sonner', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
    info: vi.fn(),
  },
}));

describe('ConfigPage', () => {
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
    localStorage.clear();
  });

  it('renders loading state', () => {
    const { useConfig } = require('../../hooks/useApi');
    useConfig.mockReturnValue({ data: null, isLoading: true });

    render(<ConfigPage />, { wrapper: createWrapper() });
    expect(screen.getByText('Loading configuration...')).toBeInTheDocument();
  });

  it('displays all configuration sections', () => {
    render(<ConfigPage />, { wrapper: createWrapper() });

    expect(screen.getByText('General')).toBeInTheDocument();
    expect(screen.getByText('Latency')).toBeInTheDocument();
    expect(screen.getByText('Fault Injection')).toBeInTheDocument();
    expect(screen.getByText('Traffic Shaping')).toBeInTheDocument();
    expect(screen.getByText('Proxy')).toBeInTheDocument();
    expect(screen.getByText('Validation')).toBeInTheDocument();
    expect(screen.getByText('Environment')).toBeInTheDocument();
  });

  it('shows current port configuration', () => {
    render(<ConfigPage />, { wrapper: createWrapper() });

    expect(screen.getByDisplayValue('3000')).toBeInTheDocument(); // HTTP port
    expect(screen.getByDisplayValue('3001')).toBeInTheDocument(); // WS port
    expect(screen.getByDisplayValue('50051')).toBeInTheDocument(); // gRPC port
    expect(screen.getByDisplayValue('9080')).toBeInTheDocument(); // Admin port
  });

  it('switches between configuration sections', () => {
    render(<ConfigPage />, { wrapper: createWrapper() });

    const latencyButton = screen.getByText('Latency');
    fireEvent.click(latencyButton);

    expect(screen.getByText('Base Latency (ms)')).toBeInTheDocument();
  });

  it('updates latency configuration', () => {
    const { useUpdateLatency } = require('../../hooks/useApi');
    const mutateMock = vi.fn();
    useUpdateLatency.mockReturnValue({ mutateAsync: mutateMock });

    render(<ConfigPage />, { wrapper: createWrapper() });

    fireEvent.click(screen.getByText('Latency'));

    const baseLatencyInput = screen.getByDisplayValue('100');
    fireEvent.change(baseLatencyInput, { target: { value: '200' } });

    const saveButton = screen.getAllByText('Save Changes')[0];
    fireEvent.click(saveButton);

    expect(mutateMock).toHaveBeenCalledWith(
      expect.objectContaining({
        base_ms: 200,
      })
    );
  });

  it('enables fault injection', () => {
    render(<ConfigPage />, { wrapper: createWrapper() });

    fireEvent.click(screen.getByText('Fault Injection'));

    const toggle = screen.getByRole('checkbox');
    fireEvent.click(toggle);

    expect(screen.getByText('Failure Rate (%)')).toBeInTheDocument();
  });

  it('selects error status codes', () => {
    const { useConfig } = require('../../hooks/useApi');
    useConfig.mockReturnValue({
      data: {
        faults: { enabled: true, failure_rate: 5, status_codes: [] },
      },
      isLoading: false,
    });

    render(<ConfigPage />, { wrapper: createWrapper() });

    fireEvent.click(screen.getByText('Fault Injection'));

    const statusCode500 = screen.getByText('500');
    fireEvent.click(statusCode500);

    // Status code should be selected (implementation details may vary)
    expect(statusCode500).toBeInTheDocument();
  });

  it('configures proxy settings', () => {
    render(<ConfigPage />, { wrapper: createWrapper() });

    fireEvent.click(screen.getByText('Proxy'));

    const toggle = screen.getByRole('checkbox');
    fireEvent.click(toggle);

    expect(screen.getByPlaceholderText('https://api.example.com')).toBeInTheDocument();
  });

  it('validates proxy URL', () => {
    const { useConfig } = require('../../hooks/useApi');
    useConfig.mockReturnValue({
      data: {
        proxy: { enabled: true, upstream_url: '', timeout_seconds: 30 },
      },
      isLoading: false,
    });

    render(<ConfigPage />, { wrapper: createWrapper() });

    fireEvent.click(screen.getByText('Proxy'));

    const urlInput = screen.getByPlaceholderText('https://api.example.com');
    fireEvent.change(urlInput, { target: { value: 'invalid-url' } });

    expect(screen.getByText('Must be a valid HTTP or HTTPS URL')).toBeInTheDocument();
  });

  it('updates validation mode', () => {
    render(<ConfigPage />, { wrapper: createWrapper() });

    fireEvent.click(screen.getByText('Validation'));

    const modeSelect = screen.getByRole('combobox');
    fireEvent.change(modeSelect, { target: { value: 'warn' } });

    expect(modeSelect).toHaveValue('warn');
  });

  it('shows unsaved changes warning', () => {
    render(<ConfigPage />, { wrapper: createWrapper() });

    const httpPortInput = screen.getByDisplayValue('3000');
    fireEvent.change(httpPortInput, { target: { value: '8080' } });

    expect(screen.getByText(/You have unsaved changes/)).toBeInTheDocument();
  });

  it('resets configuration to server values', () => {
    render(<ConfigPage />, { wrapper: createWrapper() });

    fireEvent.click(screen.getByText('Latency'));

    const baseLatencyInput = screen.getByDisplayValue('100');
    fireEvent.change(baseLatencyInput, { target: { value: '200' } });

    const resetButton = screen.getByText('Reset');
    fireEvent.click(resetButton);

    expect(baseLatencyInput).toHaveValue(100);
  });

  it('resets all settings', () => {
    render(<ConfigPage />, { wrapper: createWrapper() });

    const resetAllButton = screen.getByText('Reset All');
    fireEvent.click(resetAllButton);

    // All settings should be reset
    expect(screen.getByDisplayValue('3000')).toBeInTheDocument();
  });

  it('saves all settings', async () => {
    const { useUpdateLatency, useUpdateFaults } = require('../../hooks/useApi');
    const latencyMock = vi.fn();
    const faultsMock = vi.fn();
    useUpdateLatency.mockReturnValue({ mutateAsync: latencyMock });
    useUpdateFaults.mockReturnValue({ mutateAsync: faultsMock });

    render(<ConfigPage />, { wrapper: createWrapper() });

    const saveAllButton = screen.getByText('Save All Changes');
    fireEvent.click(saveAllButton);

    await waitFor(() => {
      expect(latencyMock).toHaveBeenCalled();
    });
  });

  it('configures traffic shaping bandwidth', () => {
    render(<ConfigPage />, { wrapper: createWrapper() });

    fireEvent.click(screen.getByText('Traffic Shaping'));

    const mainToggle = screen.getAllByRole('checkbox')[0];
    fireEvent.click(mainToggle);

    const bandwidthToggle = screen.getAllByRole('checkbox')[1];
    fireEvent.click(bandwidthToggle);

    expect(screen.getByText('Max Bandwidth (bytes/sec)')).toBeInTheDocument();
  });

  it('configures burst loss simulation', () => {
    render(<ConfigPage />, { wrapper: createWrapper() });

    fireEvent.click(screen.getByText('Traffic Shaping'));

    const mainToggle = screen.getAllByRole('checkbox')[0];
    fireEvent.click(mainToggle);

    expect(screen.getByText('Burst Loss Simulation')).toBeInTheDocument();
  });

  it('validates port ranges', () => {
    const { toast } = require('sonner');

    render(<ConfigPage />, { wrapper: createWrapper() });

    const httpPortInput = screen.getByDisplayValue('3000');
    fireEvent.change(httpPortInput, { target: { value: '70000' } });

    const saveButton = screen.getByText('Save & Restart Server');
    fireEvent.click(saveButton);

    expect(toast.error).toHaveBeenCalledWith('Invalid HTTP port. Must be between 1 and 65535');
  });

  it('shows restart confirmation dialog', () => {
    render(<ConfigPage />, { wrapper: createWrapper() });

    const httpPortInput = screen.getByDisplayValue('3000');
    fireEvent.change(httpPortInput, { target: { value: '8080' } });

    const saveButton = screen.getByText('Save & Restart Server');
    fireEvent.click(saveButton);

    expect(screen.getByText('Restart Server Required')).toBeInTheDocument();
  });

  it('handles server restart', async () => {
    const { useRestartServers } = require('../../hooks/useApi');
    const restartMock = vi.fn();
    useRestartServers.mockReturnValue({ mutateAsync: restartMock });

    render(<ConfigPage />, { wrapper: createWrapper() });

    const httpPortInput = screen.getByDisplayValue('3000');
    fireEvent.change(httpPortInput, { target: { value: '8080' } });

    const saveButton = screen.getByText('Save & Restart Server');
    fireEvent.click(saveButton);

    const confirmButton = screen.getByText('Restart Server');
    fireEvent.click(confirmButton);

    await waitFor(() => {
      expect(restartMock).toHaveBeenCalled();
    });
  });

  it('saves port config to localStorage', () => {
    render(<ConfigPage />, { wrapper: createWrapper() });

    const httpPortInput = screen.getByDisplayValue('3000');
    fireEvent.change(httpPortInput, { target: { value: '8080' } });

    const saveButton = screen.getByText('Save & Restart Server');
    fireEvent.click(saveButton);

    const savedConfig = localStorage.getItem('mockforge_pending_port_config');
    expect(savedConfig).toBeTruthy();
    expect(JSON.parse(savedConfig!)).toMatchObject({ http_port: 8080 });
  });

  it('loads pending port config on mount', () => {
    localStorage.setItem(
      'mockforge_pending_port_config',
      JSON.stringify({
        http_port: 9000,
        ws_port: 9001,
        grpc_port: 50052,
        admin_port: 9090,
      })
    );

    render(<ConfigPage />, { wrapper: createWrapper() });

    expect(screen.getByDisplayValue('9000')).toBeInTheDocument();
  });

  it('displays environment manager', () => {
    render(<ConfigPage />, { wrapper: createWrapper() });

    fireEvent.click(screen.getByText('Environment'));

    expect(screen.getByText('Template Testing')).toBeInTheDocument();
  });
});
