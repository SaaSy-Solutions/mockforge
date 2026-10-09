import { describe, it, expect, vi, beforeEach } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { CloudFlowsPage } from '../CloudFlowsPage';
import { ConfirmationDialog } from '../../components/ui/ConfirmationDialog';
import { cloudFlowsApi, type Flow } from '../../services/api/cloudFlows';

vi.mock('../../utils/cloudMode', () => ({ isCloudMode: () => true }));
vi.mock('../../stores/useWorkspaceStore', () => ({
    useWorkspaceStore: (select: (state: unknown) => unknown) => select({ activeWorkspace: { id: 'workspace' } }),
}));
vi.mock('../../services/api/cloudFlows', () => ({ cloudFlowsApi: {
    listForWorkspace: vi.fn(), delete: vi.fn(), triggerRun: vi.fn(),
} }));
vi.mock('../../components/RunLiveTail', () => ({ default: ({ runId }: { runId: string }) => <div>Live run {runId}</div> }));

const flow: Flow = {
    id: 'flow-1', workspace_id: 'workspace', kind: 'scenario', name: 'QA flow',
    description: null, current_version_id: null, metadata: {}, created_by: null,
    created_at: '2026-10-09T00:00:00Z', updated_at: '2026-10-09T00:00:00Z',
};
function showPage() {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } });
    return render(<QueryClientProvider client={client}><CloudFlowsPage /><ConfirmationDialog /></QueryClientProvider>);
}
describe('Cloud flow row actions', () => {
    beforeEach(() => {
        vi.clearAllMocks();
        vi.mocked(cloudFlowsApi.listForWorkspace).mockResolvedValue([flow]);
    });

    it('uses a nonblocking confirmation and cancels without deleting or opening the editor', async () => {
        const nativeConfirm = vi.spyOn(window, 'confirm').mockImplementation(() => { throw new Error('Native dialog must not open'); });
        showPage();
        fireEvent.click(await screen.findByRole('button', { name: 'Delete QA flow' }));
        expect(screen.getByRole('dialog')).toHaveTextContent('Delete flow "QA flow"');
        expect(screen.queryByText('Config (JSON — saving creates a new version)')).not.toBeInTheDocument();
        fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
        await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
        expect(cloudFlowsApi.delete).not.toHaveBeenCalled();
        expect(nativeConfirm).not.toHaveBeenCalled();
        nativeConfirm.mockRestore();
    });

    it('disables row actions while deleting and refreshes the list on success', async () => {
        let resolve!: (value: { deleted: boolean }) => void;
        vi.mocked(cloudFlowsApi.delete).mockReturnValue(new Promise((done) => { resolve = done; }));
        showPage();
        fireEvent.click(await screen.findByRole('button', { name: 'Delete QA flow' }));
        fireEvent.click(screen.getByRole('button', { name: 'Confirm' }));
        await waitFor(() => expect(screen.getByRole('button', { name: 'Delete QA flow' })).toBeDisabled());
        expect(screen.getByRole('button', { name: 'Trigger run for QA flow' })).toBeDisabled();
        expect(cloudFlowsApi.delete).toHaveBeenCalledTimes(1);
        expect(cloudFlowsApi.delete).toHaveBeenCalledWith('flow-1');
        vi.mocked(cloudFlowsApi.listForWorkspace).mockResolvedValue([]);
        resolve({ deleted: true });
        await waitFor(() => expect(screen.queryByText('QA flow')).not.toBeInTheDocument());
    });

    it('reports delete failures and leaves the flow available to retry', async () => {
        vi.mocked(cloudFlowsApi.delete).mockRejectedValue(new Error('Access denied'));
        showPage();
        fireEvent.click(await screen.findByRole('button', { name: 'Delete QA flow' }));
        fireEvent.click(screen.getByRole('button', { name: 'Confirm' }));
        expect(await screen.findByRole('alert')).toHaveTextContent('Delete failed: Access denied');
        expect(screen.getByRole('button', { name: 'Delete QA flow' })).toBeEnabled();
    });

    it('prevents duplicate triggers while a run request is pending', async () => {
        vi.mocked(cloudFlowsApi.triggerRun).mockReturnValue(new Promise(() => {}));
        showPage();
        const trigger = await screen.findByRole('button', { name: 'Trigger run for QA flow' });
        fireEvent.click(trigger);
        await waitFor(() => expect(trigger).toBeDisabled());
        fireEvent.click(trigger);
        expect(cloudFlowsApi.triggerRun).toHaveBeenCalledTimes(1);
        expect(cloudFlowsApi.triggerRun).toHaveBeenCalledWith('flow-1');
        expect(screen.queryByText('Config (JSON — saving creates a new version)')).not.toBeInTheDocument();
    });
});
