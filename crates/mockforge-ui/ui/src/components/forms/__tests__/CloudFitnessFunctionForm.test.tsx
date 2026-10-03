import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { CloudFitnessFunctionForm } from '../CloudFitnessFunctionForm';
import { fetchJsonWithErrorBody } from '../../../services/api/client';
import { cloudContractApi } from '../../../services/api/cloudContract';
vi.mock('../../../services/api/client', () => ({ fetchJsonWithErrorBody: vi.fn() }));
vi.mock('../../../services/api/cloudContract', () => ({ cloudContractApi: { listMonitoredServices: vi.fn() } }));
function form(initial = null) {
  const onSave = vi.fn();
  render(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
    <CloudFitnessFunctionForm initial={initial} workspaceId="workspace" saving={false} onSave={onSave} onCancel={() => {}} />
  </QueryClientProvider>);
  return onSave;
}
describe('cloud fitness authoring', () => {
  beforeEach(() => {
    vi.mocked(fetchJsonWithErrorBody).mockResolvedValue([{ id: 'deployment', name: 'Hosted mock' }]);
    vi.mocked(cloudContractApi.listMonitoredServices).mockResolvedValue([{ id: 'service', name: 'API' }] as never);
  });
  it('creates a runner-compatible latency function with a deployment and threshold', async () => {
    const save = form();
    fireEvent.change(screen.getByLabelText('Name'), { target: { value: 'Latency budget' } });
    await waitFor(() => expect(screen.getByLabelText('Hosted deployment')).toHaveValue('deployment'));
    fireEvent.click(screen.getByRole('button', { name: 'Create Fitness Function' }));
    expect(save).toHaveBeenCalledWith(expect.objectContaining({ kind: 'latency_threshold', config: expect.objectContaining({ deployment_id: 'deployment', threshold_ms: 500, percentile: 'p95' }) }));
  });
  it('converts error percentages to fractions and excludes local-only kinds', async () => {
    const save = form();
    expect(screen.queryByRole('option', { name: 'Response Size' })).not.toBeInTheDocument();
    fireEvent.change(screen.getByLabelText('Name'), { target: { value: 'Error budget' } });
    fireEvent.change(screen.getByLabelText('Function Type'), { target: { value: 'error_rate' } });
    fireEvent.change(screen.getByLabelText('Maximum error rate (%)'), { target: { value: '2.5' } });
    await waitFor(() => expect(screen.getByLabelText('Hosted deployment')).toHaveValue('deployment'));
    fireEvent.click(screen.getByRole('button', { name: 'Create Fitness Function' }));
    expect(save).toHaveBeenCalledWith(expect.objectContaining({ kind: 'error_rate', config: expect.objectContaining({ threshold_rate: 0.025 }) }));
  });
  it('requires a monitored service for contract stability', async () => {
    const save = form();
    fireEvent.change(screen.getByLabelText('Name'), { target: { value: 'Contract budget' } });
    fireEvent.change(screen.getByLabelText('Function Type'), { target: { value: 'contract_stability' } });
    await waitFor(() => expect(screen.getByLabelText('Monitored service')).toHaveValue('service'));
    fireEvent.click(screen.getByRole('button', { name: 'Create Fitness Function' }));
    expect(save).toHaveBeenCalledWith(expect.objectContaining({ kind: 'contract_stability', config: expect.objectContaining({ monitored_service_id: 'service', max_breaking: 0 }) }));
  });
});
