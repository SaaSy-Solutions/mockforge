/**
 * @jest-environment jsdom
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import WorkspacesPage from '../WorkspacesPage';
import type { WorkspaceSummary } from '../../types';
import { apiService } from '../../services/api';
import { useWorkspaceStore } from '../../stores/useWorkspaceStore';
import { useUpdateWorkspacesOrder } from '../../hooks/useApi';

const mockWorkspaces: WorkspaceSummary[] = [
  {
    id: 'workspace-1',
    name: 'Development',
    description: 'Development workspace',
    is_active: true,
    request_count: 10,
    folder_count: 3,
  },
  {
    id: 'workspace-2',
    name: 'Testing',
    description: 'Testing workspace',
    is_active: false,
    request_count: 5,
    folder_count: 2,
  },
];

vi.mock('../../stores/useWorkspaceStore');
vi.mock('../../services/api');
vi.mock('../../hooks/useApi');

// Set default mocks
vi.mocked(useWorkspaceStore).mockReturnValue({
  workspaces: mockWorkspaces,
  loading: false,
  error: null,
  setActiveWorkspaceById: vi.fn(),
  getState: () => ({ refreshWorkspaces: vi.fn() }),
} as any);

Object.assign(apiService, {
  createWorkspace: vi.fn().mockResolvedValue({ data: { id: 'new-workspace' } }),
  getWorkspace: vi.fn().mockResolvedValue({ workspace: { summary: mockWorkspaces[0], folders: [], requests: [] } }),
  deleteWorkspace: vi.fn().mockResolvedValue({}),
  openWorkspaceFromDirectory: vi.fn().mockResolvedValue({}),
  createFolder: vi.fn().mockResolvedValue({}),
  getFolder: vi.fn().mockResolvedValue({ folder: { summary: { id: 'folder-1', name: 'Folder 1' }, requests: [] } }),
  createRequest: vi.fn().mockResolvedValue({}),
  importToWorkspace: vi.fn().mockResolvedValue({}),
  previewImport: vi.fn().mockResolvedValue({ routes: [], success: true }),
  configureSync: vi.fn().mockResolvedValue({}),
  getWorkspaceEncryptionStatus: vi.fn().mockResolvedValue({
    enabled: false,
    masterKeySet: false,
    workspaceKeySet: false,
  }),
  getWorkspaceEncryptionConfig: vi.fn().mockResolvedValue({
    autoEncrypt: false,
    encryptPaths: [],
  }),
});

vi.mocked(useUpdateWorkspacesOrder).mockReturnValue({ mutateAsync: vi.fn() } as any);

vi.mock('sonner', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
    info: vi.fn(),
  },
}));

describe('WorkspacesPage', () => {
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

    // Re-apply default mocks after clearing
    vi.mocked(useWorkspaceStore).mockReturnValue({
      workspaces: mockWorkspaces,
      loading: false,
      error: null,
      setActiveWorkspaceById: vi.fn(),
      getState: () => ({ refreshWorkspaces: vi.fn() }),
    } as any);

    vi.mocked(useUpdateWorkspacesOrder).mockReturnValue({ mutateAsync: vi.fn() } as any);
  });

  it('renders workspaces page header', () => {
    render(<WorkspacesPage />, { wrapper: createWrapper() });

    expect(screen.getByText('Workspaces')).toBeInTheDocument();
    expect(screen.getByText('Manage your mock API workspaces')).toBeInTheDocument();
  });

  it('displays workspace cards', () => {
    render(<WorkspacesPage />, { wrapper: createWrapper() });

    expect(screen.getByText('Development')).toBeInTheDocument();
    expect(screen.getByText('Testing')).toBeInTheDocument();
  });

  it('shows active workspace badge', () => {
    render(<WorkspacesPage />, { wrapper: createWrapper() });

    expect(screen.getByText('Active')).toBeInTheDocument();
  });

  it('displays workspace statistics', () => {
    render(<WorkspacesPage />, { wrapper: createWrapper() });

    expect(screen.getByText('10 requests')).toBeInTheDocument();
    expect(screen.getByText('3 folders')).toBeInTheDocument();
  });

  it('opens create workspace dialog', () => {
    render(<WorkspacesPage />, { wrapper: createWrapper() });

    const createButton = screen.getByText('New Workspace');
    fireEvent.click(createButton);

    expect(screen.getByText('Create New Workspace')).toBeInTheDocument();
  });

  it('creates new workspace', async () => {
    render(<WorkspacesPage />, { wrapper: createWrapper() });

    fireEvent.click(screen.getByText('New Workspace'));

    const nameInput = screen.getByPlaceholderText('My Workspace');
    fireEvent.change(nameInput, { target: { value: 'New Workspace' } });

    const createButton = screen.getAllByText('Create Workspace').pop()!;
    fireEvent.click(createButton);

    await waitFor(() => {
      expect(apiService.createWorkspace).toHaveBeenCalledWith({
        name: 'New Workspace',
        description: '',
      });
    });
  });

  it('enables directory sync when creating workspace', async () => {

    render(<WorkspacesPage />, { wrapper: createWrapper() });

    fireEvent.click(screen.getByText('New Workspace'));

    const syncCheckbox = screen.getByLabelText('Enable directory sync');
    fireEvent.click(syncCheckbox);

    const syncInput = screen.getByPlaceholderText('/path/to/workspace');
    fireEvent.change(syncInput, { target: { value: '/my/path' } });

    const nameInput = screen.getByPlaceholderText('My Workspace');
    fireEvent.change(nameInput, { target: { value: 'Synced Workspace' } });

    const createButton = screen.getAllByText('Create Workspace').pop()!;
    fireEvent.click(createButton);

    await waitFor(() => {
      expect(apiService.configureSync).toHaveBeenCalledWith(
        'new-workspace',
        expect.objectContaining({
          target_directory: '/my/path',
          sync_direction: 'Bidirectional',
        })
      );
    });
  });

  it('opens workspace from directory', async () => {

    render(<WorkspacesPage />, { wrapper: createWrapper() });

    fireEvent.click(screen.getByText('Open from Directory'));

    const dirInput = screen.getByPlaceholderText('/path/to/workspace');
    fireEvent.change(dirInput, { target: { value: '/existing/workspace' } });

    const openButton = screen.getByText('Open Workspace');
    fireEvent.click(openButton);

    await waitFor(() => {
      expect(apiService.openWorkspaceFromDirectory).toHaveBeenCalled();
    });
  });

  it('selects workspace', async () => {

    render(<WorkspacesPage />, { wrapper: createWrapper() });

    // Click on the workspace description/stats area which has the onClick handler
    const workspaceDescription = screen.getByText('Development workspace');
    fireEvent.click(workspaceDescription);

    await waitFor(() => {
      expect(apiService.getWorkspace).toHaveBeenCalledWith('workspace-1');
    });
  });

  it('deletes workspace with confirmation', async () => {

    render(<WorkspacesPage />, { wrapper: createWrapper() });

    // Find the trash icon button specifically
    const deleteButtons = screen.getAllByRole('button');
    const deleteButton = deleteButtons.find((btn) => {
      const svg = btn.querySelector('svg');
      return svg && svg.classList.contains('lucide-trash-2');
    });
    fireEvent.click(deleteButton!);

    // Wait for the dialog to appear in the portal
    await waitFor(() => {
      expect(screen.getByRole('dialog')).toBeInTheDocument();
    });

    // Find the confirm button (not the title)
    const confirmButton = screen.getAllByText('Delete Workspace').find((el) => el.tagName === 'BUTTON');
    fireEvent.click(confirmButton!);

    await waitFor(() => {
      expect(apiService.deleteWorkspace).toHaveBeenCalled();
    });
  });

  it('sets active workspace', async () => {
    const setActiveMock = vi.fn();
    vi.mocked(useWorkspaceStore).mockReturnValue({
      workspaces: mockWorkspaces,
      loading: false,
      error: null,
      setActiveWorkspaceById: setActiveMock,
      getState: () => ({ refreshWorkspaces: vi.fn() }),
    } as any);

    render(<WorkspacesPage />, { wrapper: createWrapper() });

    // Find the Play button that is not disabled (Testing workspace)
    const playButtons = screen.getAllByRole('button');
    const activateButton = playButtons.find((btn) => {
      const svg = btn.querySelector('svg');
      return svg && svg.classList.contains('lucide-play') && !btn.disabled;
    });
    fireEvent.click(activateButton!);

    await waitFor(() => {
      expect(setActiveMock).toHaveBeenCalled();
    });
  });

  it('creates folder in workspace', async () => {

    render(<WorkspacesPage />, { wrapper: createWrapper() });

    // Select workspace first by clicking on the description
    const workspaceDescription = screen.getByText('Development workspace');
    fireEvent.click(workspaceDescription);

    // Wait for the workspace to load and New Folder button to appear
    await waitFor(() => {
      expect(screen.getByText('New Folder')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText('New Folder'));

    const nameInput = screen.getByPlaceholderText('My Folder');
    fireEvent.change(nameInput, { target: { value: 'API Folder' } });

    const createButton = screen.getAllByText('Create Folder').pop()!;
    fireEvent.click(createButton);

    await waitFor(() => {
      expect(apiService.createFolder).toHaveBeenCalledWith(
        'workspace-1',
        expect.objectContaining({ name: 'API Folder' })
      );
    });
  });

  it('creates request in workspace', async () => {

    render(<WorkspacesPage />, { wrapper: createWrapper() });

    // Select workspace by clicking on the description
    fireEvent.click(screen.getByText('Development workspace'));

    // Wait for the workspace to load and New Request button to appear
    await waitFor(() => {
      expect(screen.getByText('New Request')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText('New Request'));

    const nameInput = screen.getByPlaceholderText('My Request');
    fireEvent.change(nameInput, { target: { value: 'Get Users' } });

    const pathInput = screen.getByPlaceholderText('/api/users');
    fireEvent.change(pathInput, { target: { value: '/api/users' } });

    const createButton = screen.getAllByText('Create Request').pop()!;
    fireEvent.click(createButton);

    await waitFor(() => {
      expect(apiService.createRequest).toHaveBeenCalled();
    });
  });

  it('drags and drops workspace to reorder', async () => {
    const updateOrderMock = vi.fn().mockResolvedValue({});
    vi.mocked(useUpdateWorkspacesOrder).mockReturnValue({ mutateAsync: updateOrderMock } as any);

    const { container } = render(<WorkspacesPage />, { wrapper: createWrapper() });

    // Get the actual draggable card elements, not just the text elements
    const workspaceCards = Array.from(container.querySelectorAll('[draggable="true"]'));
    expect(workspaceCards).toHaveLength(2);

    // Create a proper dataTransfer mock
    const createDataTransfer = () => ({
      effectAllowed: '',
      dropEffect: '',
      setData: vi.fn(),
      getData: vi.fn(),
      clearData: vi.fn(),
      setDragImage: vi.fn(),
    });

    // Simulate the complete drag-and-drop flow
    const dragDataTransfer = createDataTransfer();
    fireEvent.dragStart(workspaceCards[0], {
      dataTransfer: dragDataTransfer,
    });

    // Wait for state update from dragStart
    await waitFor(() => {
      expect(dragDataTransfer.effectAllowed).toBe('move');
    });

    // Simulate dragging over the target
    const dragOverDataTransfer = createDataTransfer();
    fireEvent.dragOver(workspaceCards[1], {
      dataTransfer: dragOverDataTransfer,
    });

    // Simulate the drop
    const dropDataTransfer = createDataTransfer();
    fireEvent.drop(workspaceCards[1], {
      dataTransfer: dropDataTransfer,
    });

    // Verify the reorder was called with the correct workspace IDs
    await waitFor(() => {
      expect(updateOrderMock).toHaveBeenCalledWith(['workspace-2', 'workspace-1']);
    });
  });

  it('shows empty state when no workspaces', () => {
    vi.mocked(useWorkspaceStore).mockReturnValue({
      workspaces: [],
      loading: false,
      error: null,
      setActiveWorkspaceById: vi.fn(),
      getState: () => ({ refreshWorkspaces: vi.fn() }),
    } as any);

    render(<WorkspacesPage />, { wrapper: createWrapper() });

    expect(screen.getByText('No Workspaces Yet')).toBeInTheDocument();
    expect(screen.getByText(/Get started by creating a new workspace/)).toBeInTheDocument();
  });

  it('shows loading state', () => {
    vi.mocked(useWorkspaceStore).mockReturnValue({
      workspaces: null,
      loading: true,
      error: null,
      setActiveWorkspaceById: vi.fn(),
      getState: () => ({ refreshWorkspaces: vi.fn() }),
    } as any);

    render(<WorkspacesPage />, { wrapper: createWrapper() });

    expect(screen.getByText('Loading workspaces...')).toBeInTheDocument();
  });

  it('shows error state', () => {
    vi.mocked(useWorkspaceStore).mockReturnValue({
      workspaces: null,
      loading: false,
      error: 'Failed to load workspaces',
      setActiveWorkspaceById: vi.fn(),
      getState: () => ({ refreshWorkspaces: vi.fn() }),
    } as any);

    render(<WorkspacesPage />, { wrapper: createWrapper() });

    expect(screen.getByText('Failed to load workspaces')).toBeInTheDocument();
  });

  it('opens encryption settings', async () => {
    render(<WorkspacesPage />, { wrapper: createWrapper() });

    // Select workspace by clicking on the description
    fireEvent.click(screen.getByText('Development workspace'));

    // Wait for the workspace to load and Encryption button to appear
    await waitFor(() => {
      expect(screen.getByText('Encryption')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText('Encryption'));

    // Wait for the dialog to appear
    await waitFor(() => {
      expect(screen.getByRole('dialog')).toBeInTheDocument();
    });
  });

  it('validates required fields in create workspace', () => {
    render(<WorkspacesPage />, { wrapper: createWrapper() });

    fireEvent.click(screen.getByText('New Workspace'));

    const createButton = screen.getAllByText('Create Workspace').pop()!;
    expect(createButton).toBeDisabled();
  });

  it('validates required fields in create folder', async () => {
    render(<WorkspacesPage />, { wrapper: createWrapper() });

    // Select workspace by clicking on the description
    fireEvent.click(screen.getByText('Development workspace'));

    // Wait for the workspace to load and New Folder button to appear
    await waitFor(() => {
      expect(screen.getByText('New Folder')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText('New Folder'));

    const createButton = screen.getAllByText('Create Folder').pop()!;
    expect(createButton).toBeDisabled();
  });

  it('validates required fields in create request', async () => {
    render(<WorkspacesPage />, { wrapper: createWrapper() });

    // Select workspace by clicking on the description
    fireEvent.click(screen.getByText('Development workspace'));

    // Wait for the workspace to load and New Request button to appear
    await waitFor(() => {
      expect(screen.getByText('New Request')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText('New Request'));

    const createButton = screen.getAllByText('Create Request').pop()!;
    expect(createButton).toBeDisabled();
  });
});
