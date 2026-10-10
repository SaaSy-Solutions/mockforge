import { describe, it, expect, vi, beforeEach } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { CloudTestRunsPage } from '../CloudTestRunsPage';
import { ConfirmationDialog } from '../../components/ui/ConfirmationDialog';
import { cloudTestRunsApi, type TestRun } from '../../services/api/cloudTestRuns';
vi.mock('../../utils/cloudMode', () => ({ isCloudMode: () => true }));
vi.mock('../../hooks/useCloudOrgId', () => ({ useCloudOrgId: () => 'org' }));
vi.mock('../../services/api/cloudTestRuns', () => ({ cloudTestRunsApi: { listOrgRuns: vi.fn(), deleteRun: vi.fn(), cancelRun: vi.fn(), streamRunEvents: vi.fn() } }));
const run: TestRun = { id: 'b004805f-fixture', org_id: 'org', suite_id: 'suite', kind: 'orchestration', status: 'passed', triggered_by: 'manual', triggered_by_user: null, queued_at: '2026-10-09T00:00:00Z', started_at: null, finished_at: null, runner_seconds: 1, summary: null, git_ref: null, git_sha: null };
function show() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } });
  return render(<QueryClientProvider client={client}><CloudTestRunsPage /><ConfirmationDialog /></QueryClientProvider>);
}
describe('Completed test run cleanup', () => {
  beforeEach(() => { vi.clearAllMocks(); vi.mocked(cloudTestRunsApi.listOrgRuns).mockResolvedValue([run]); });
  it('requires a nonblocking confirmation and removes completed history', async () => {
    const confirm = vi.spyOn(window, 'confirm').mockImplementation(() => { throw new Error('Blocking dialog'); });
    vi.mocked(cloudTestRunsApi.deleteRun).mockResolvedValue({ deleted: true });
    show();
    fireEvent.click(await screen.findByRole('button', { name: 'Delete run b004805f' }));
    expect(screen.getByRole('dialog')).toHaveTextContent('Delete run b004805f');
    expect(cloudTestRunsApi.deleteRun).not.toHaveBeenCalled();
    vi.mocked(cloudTestRunsApi.listOrgRuns).mockResolvedValue([]);
    fireEvent.click(screen.getByRole('button', { name: 'Confirm' }));
    await waitFor(() => expect(cloudTestRunsApi.deleteRun).toHaveBeenCalledWith(run.id));
    expect(await screen.findByText('No runs')).toBeVisible();
    expect(confirm).not.toHaveBeenCalled();
    confirm.mockRestore();
  });
  it('shows API deletion errors and keeps the run', async () => {
    vi.mocked(cloudTestRunsApi.deleteRun).mockRejectedValue(new Error('Deletion failed'));
    show();
    fireEvent.click(await screen.findByRole('button', { name: 'Delete run b004805f' }));
    fireEvent.click(screen.getByRole('button', { name: 'Confirm' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Deletion failed');
    expect(screen.getByText('b004805f')).toBeVisible();
  });
  it('offers cancel, rather than delete, for an active run', async () => {
    vi.mocked(cloudTestRunsApi.listOrgRuns).mockResolvedValue([{ ...run, status: 'running' }]);
    show();
    expect(await screen.findByRole('button', { name: 'Cancel run b004805f' })).toBeVisible();
    expect(screen.queryByRole('button', { name: 'Delete run b004805f' })).not.toBeInTheDocument();
  });
});
