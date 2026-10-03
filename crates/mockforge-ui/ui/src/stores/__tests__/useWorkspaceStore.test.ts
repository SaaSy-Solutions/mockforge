import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useWorkspaceStore } from '../useWorkspaceStore';
import { apiService } from '../../services/api';
import type { WorkspaceSummary } from '../../types';
vi.mock('../../services/api', () => ({ apiService: { listWorkspaces: vi.fn(), setActiveWorkspace: vi.fn() } }));
const workspace = (id: string, name = id, is_active = false): WorkspaceSummary => ({
  id, name, is_active, created_at: '', updated_at: '', config_count: 0, service_count: 0,
});
describe('workspace reconciliation', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useWorkspaceStore.setState({ activeWorkspace: workspace('deleted', 'QA-TEST Workspace'), workspaces: [], error: null, loading: false });
  });
  it('replaces a deleted persisted selection with a real workspace', async () => {
    const available = workspace('available');
    vi.mocked(apiService.listWorkspaces).mockResolvedValue([available]);
    await useWorkspaceStore.getState().refreshWorkspaces();
    expect(useWorkspaceStore.getState().activeWorkspace).toEqual(available);
  });
  it('clears the selection after deleting the last workspace', async () => {
    vi.mocked(apiService.listWorkspaces).mockResolvedValue([]);
    await useWorkspaceStore.getState().refreshWorkspaces();
    expect(useWorkspaceStore.getState().activeWorkspace).toBeNull();
  });
  it('updates stale workspace metadata from the canonical list', () => {
    useWorkspaceStore.setState({ activeWorkspace: workspace('available', 'Old name') });
    useWorkspaceStore.getState().setWorkspaces([workspace('available', 'New name')]);
    expect(useWorkspaceStore.getState().activeWorkspace?.name).toBe('New name');
  });
  it('uses the selected ID even when an activation response has no active flag', async () => {
    vi.mocked(apiService.listWorkspaces).mockResolvedValue([workspace('first'), workspace('selected')]);
    await useWorkspaceStore.getState().setActiveWorkspaceById('selected');
    expect(useWorkspaceStore.getState().activeWorkspace?.id).toBe('selected');
  });
});
