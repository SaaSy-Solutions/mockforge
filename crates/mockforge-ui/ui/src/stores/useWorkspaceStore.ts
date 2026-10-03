import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { apiService } from '../services/api';
import type { WorkspaceSummary } from '../schemas/api';

interface WorkspaceState {
  activeWorkspace: WorkspaceSummary | null;
  workspaces: WorkspaceSummary[];
  loading: boolean;
  error: string | null;
}

interface WorkspaceActions {
  setActiveWorkspace: (workspace: WorkspaceSummary | null) => void;
  setWorkspaces: (workspaces: WorkspaceSummary[]) => void;
  loadWorkspaces: () => Promise<void>;
  setActiveWorkspaceById: (workspaceId: string) => Promise<void>;
  refreshWorkspaces: () => Promise<void>;
}

export function reconcileWorkspace(workspaces: WorkspaceSummary[], selected: WorkspaceSummary | null) {
  return workspaces.find((workspace) => workspace.is_active)
    ?? workspaces.find((workspace) => workspace.id === selected?.id)
    ?? workspaces[0]
    ?? null;
}

let requestVersion = 0;

export const useWorkspaceStore = create<WorkspaceState & WorkspaceActions>()(
  persist(
    (set, get) => ({
      activeWorkspace: null,
      workspaces: [],
      loading: false,
      error: null,

      setActiveWorkspace: (workspace) => {
        set({ activeWorkspace: workspace });
      },

      setWorkspaces: (workspaces) => {
        set({ workspaces, activeWorkspace: reconcileWorkspace(workspaces, get().activeWorkspace) });
      },

      loadWorkspaces: async () => {
        const version = ++requestVersion;
        const selected = get().activeWorkspace;
        // Persisted selections are untrusted until the server lists them.
        set({ loading: true, error: null, activeWorkspace: get().workspaces.find((workspace) => workspace.id === selected?.id) ?? null });
        try {
          const workspaces = await apiService.listWorkspaces();
          if (version !== requestVersion) return;
          set({ workspaces, loading: false, activeWorkspace: reconcileWorkspace(workspaces, selected) });
        } catch (error) {
          if (version !== requestVersion) return;
          set({
            error: error instanceof Error ? error.message : 'Failed to load workspaces',
            loading: false,
          });
        }
      },

      setActiveWorkspaceById: async (workspaceId) => {
        const version = ++requestVersion;
        set({ loading: true, error: null });
        try {
          await apiService.setActiveWorkspace(workspaceId);
          const workspaces = await apiService.listWorkspaces();
          if (version !== requestVersion) return;
          const activeWorkspace = workspaces.find((workspace) => workspace.id === workspaceId);
          if (!activeWorkspace) throw new Error('Workspace no longer exists');
          set({ workspaces, loading: false, activeWorkspace });
        } catch (error) {
          set({
            error: error instanceof Error ? error.message : 'Failed to set active workspace',
            loading: false,
          });
          throw error;
        }
      },

      refreshWorkspaces: async () => {
        await get().loadWorkspaces();
      },
    }),
    {
      name: 'mockforge-workspace',
      partialize: (state) => ({
        activeWorkspace: state.activeWorkspace,
      }),
    }
  )
);
