import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { CloudContractPage } from '../CloudContractPage';
import { ConfirmationDialog } from '../../components/ui/ConfirmationDialog';
import { cloudContractApi, type MonitoredService } from '../../services/api/cloudContract';

vi.mock('../../utils/cloudMode', () => ({ isCloudMode: () => true }));
vi.mock('../../stores/useWorkspaceStore', () => ({
    useWorkspaceStore: (select: (state: unknown) => unknown) => select({ activeWorkspace: { id: 'workspace' } }),
}));
vi.mock('../../services/api/cloudContract', () => ({ cloudContractApi: {
    listMonitoredServices: vi.fn(), listFitnessFunctions: vi.fn(), createMonitoredService: vi.fn(),
    deleteMonitoredService: vi.fn(), triggerDiff: vi.fn(), listDiffRuns: vi.fn(), listFindings: vi.fn(),
} }));

const service: MonitoredService = {
    id: 'service', workspace_id: 'workspace', name: 'Orders', base_url: 'https://example.test',
    openapi_spec_url: 'https://example.test/api.json', traffic_source: 'probe', enabled: true,
    created_at: '2026-10-12T00:00:00Z', updated_at: '2026-10-12T00:00:00Z',
};
function showPage() {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } });
    return render(<QueryClientProvider client={client}><CloudContractPage /><ConfirmationDialog /></QueryClientProvider>);
}
async function fillService() {
    fireEvent.click(await screen.findByRole('button', { name: 'Add First Service' }));
    fireEvent.change(screen.getByRole('textbox', { name: 'Name' }), { target: { value: 'Orders' } });
    fireEvent.change(screen.getByRole('textbox', { name: 'Base URL' }), { target: { value: service.base_url } });
    fireEvent.change(screen.getByRole('textbox', { name: 'OpenAPI spec URL' }), { target: { value: service.openapi_spec_url } });
}
describe('Cloud contract creation and schedules', () => {
    beforeEach(() => {
        vi.resetAllMocks();
        vi.mocked(cloudContractApi.listMonitoredServices).mockResolvedValue([]);
        vi.mocked(cloudContractApi.listFitnessFunctions).mockResolvedValue([]);
        vi.mocked(cloudContractApi.createMonitoredService).mockResolvedValue(service);
    });

    it('submits a supported traffic source and omits an empty cron schedule', async () => {
        showPage(); await fillService();
        const source = screen.getByRole('combobox', { name: 'Traffic source' });
        expect(source).toHaveValue('probe');
        expect(Array.from((source as HTMLSelectElement).options).map(option => option.value))
            .toEqual(['probe', 'capture_session', 'logs']);
        fireEvent.click(screen.getByRole('button', { name: /^Create$/ }));
        await waitFor(() => expect(cloudContractApi.createMonitoredService).toHaveBeenCalledWith('workspace', {
            name: 'Orders', base_url: service.base_url, openapi_spec_url: service.openapi_spec_url,
            traffic_source: 'probe', probe_cron: undefined, probe_timezone: undefined,
        }));
        await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
    });

    it('sends the configured cron and timezone and keeps captured traffic selectable', async () => {
        showPage(); await fillService();
        fireEvent.change(screen.getByRole('combobox', { name: 'Traffic source' }), { target: { value: 'capture_session' } });
        fireEvent.change(screen.getByRole('textbox', { name: 'Cron schedule (optional)' }), { target: { value: ' 0 0 9 * * * ' } });
        fireEvent.change(screen.getByRole('textbox', { name: 'Schedule timezone' }), { target: { value: 'America/New_York' } });
        fireEvent.click(screen.getByRole('button', { name: /^Create$/ }));
        await waitFor(() => expect(cloudContractApi.createMonitoredService).toHaveBeenCalledWith('workspace', expect.objectContaining({
            traffic_source: 'capture_session', probe_cron: '0 0 9 * * *', probe_timezone: 'America/New_York',
        })));
    });

    it('surfaces server schedule validation errors and preserves the draft for correction', async () => {
        vi.mocked(cloudContractApi.createMonitoredService).mockRejectedValue(new Error('probe_cron must be a valid seconds-first cron expression'));
        showPage(); await fillService();
        fireEvent.change(screen.getByRole('textbox', { name: 'Cron schedule (optional)' }), { target: { value: 'invalid cron' } });
        fireEvent.click(screen.getByRole('button', { name: /^Create$/ }));
        expect(await screen.findByRole('alert')).toHaveTextContent('probe_cron must be a valid');
        expect(screen.getByRole('textbox', { name: 'Name' })).toHaveValue('Orders');
        expect(screen.getByRole('textbox', { name: 'Cron schedule (optional)' })).toHaveValue('invalid cron');
        expect(screen.getByRole('button', { name: /^Create$/ })).toBeEnabled();
    });

    it('shows the actual schedule and cancels deletion without a native dialog', async () => {
        vi.mocked(cloudContractApi.listMonitoredServices).mockResolvedValue([{ ...service, probe_cron: '0 0 9 * * *', probe_timezone: 'America/New_York' }]);
        const nativeConfirm = vi.spyOn(window, 'confirm').mockImplementation(() => { throw new Error('Native dialog must not open'); });
        showPage();
        expect(await screen.findByText('America/New_York')).toBeInTheDocument();
        fireEvent.click(screen.getByRole('button', { name: 'Delete Orders' }));
        expect(screen.getByRole('dialog')).toHaveTextContent('Delete monitored service "Orders"');
        fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
        expect(cloudContractApi.deleteMonitoredService).not.toHaveBeenCalled();
        expect(nativeConfirm).not.toHaveBeenCalled(); nativeConfirm.mockRestore();
    });

    it('shows a deletion failure while retaining the service', async () => {
        vi.mocked(cloudContractApi.listMonitoredServices).mockResolvedValue([service]);
        vi.mocked(cloudContractApi.deleteMonitoredService).mockRejectedValue(new Error('Access denied'));
        showPage();
        fireEvent.click(await screen.findByRole('button', { name: 'Delete Orders' }));
        fireEvent.click(screen.getByRole('button', { name: 'Confirm' }));
        expect(await screen.findByRole('alert')).toHaveTextContent('Delete failed: Access denied');
        expect(screen.getByRole('button', { name: 'Delete Orders' })).toBeInTheDocument();
    });
});
