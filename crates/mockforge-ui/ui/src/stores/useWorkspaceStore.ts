import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { apiService } from '../services/api';
import type { WorkspaceSummary } from '../types';

interface WorkspaceState {
  activeWorkspace: WorkspaceSummary | null;
  workspaces: WorkspaceSummary[];
  loading: boolean;
  error: string | null;
}

interface WorkspaceActions {
  setActiveWorkspace: (workspace: WorkspaceSummary | null) => void;
  loadWorkspaces: () => Promise<void>;
  setActiveWorkspaceById: (workspaceId: string) => Promise<void>;
  refreshWorkspaces: () => Promise<void>;
}

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

      loadWorkspaces: async () => {
        set({ loading: true, error: null });
        try {
          const response = await apiService.listWorkspaces();
          set({ workspaces: response.workspaces, loading: false });

          // Set the first active workspace as the default active workspace
          const activeWorkspace = response.workspaces.find((w: WorkspaceSummary) => w.is_active);
          if (activeWorkspace) {
            set({ activeWorkspace });
          }
        } catch (error) {
          set({
            error: error instanceof Error ? error.message : 'Failed to load workspaces',
            loading: false
          });
        }
      },

      setActiveWorkspaceById: async (workspaceId) => {
        set({ loading: true, error: null });
        try {
          await apiService.setActiveWorkspace(workspaceId);
          const response = await apiService.listWorkspaces();
          set({ workspaces: response.workspaces, loading: false });

          const activeWorkspace = response.workspaces.find((w: WorkspaceSummary) => w.is_active);
          set({ activeWorkspace });
        } catch (error) {
          set({
            error: error instanceof Error ? error.message : 'Failed to set active workspace',
            loading: false
          });
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