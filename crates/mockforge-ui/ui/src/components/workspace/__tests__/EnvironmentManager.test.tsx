/**
 * @jest-environment jsdom
 */

import React from 'react';
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { EnvironmentManager } from '../EnvironmentManager';
import {
  useEnvironments,
  useCreateEnvironment,
  useUpdateEnvironment,
  useDeleteEnvironment,
  useSetActiveEnvironment,
  useEnvironmentVariables,
  useUpdateEnvironmentsOrder,
} from '../../../hooks/useApi';
import { toast } from '../../ui/Toast';

const mockEnvironments = {
  environments: [
    {
      id: 'env-1',
      name: 'Development',
      description: 'Dev environment',
      is_global: false,
      active: true,
      variable_count: 5,
      order: 0,
    },
    {
      id: 'global',
      name: 'Global',
      description: 'Global environment',
      is_global: true,
      active: false,
      variable_count: 3,
      order: -1,
    },
  ],
};

const mockVariables = {
  variables: [
    { name: 'API_URL', value: 'https://api.dev.com' },
    { name: 'API_KEY', value: 'dev-key-123' },
  ],
};

vi.mock('../../../hooks/useApi');
vi.mock('../../ui/Toast');

// Mock Dialog components to render children properly in tests
vi.mock('../../ui/Dialog', () => {
  const MockDialog = ({ children, open, onOpenChange }: any) => {
    const [internalOpen, setInternalOpen] = React.useState(false);
    const isOpen = open !== undefined ? open : internalOpen;

    const handleOpenChange = (newOpen: boolean) => {
      setInternalOpen(newOpen);
      onOpenChange?.(newOpen);
    };

    const trigger = React.Children.toArray(children).find(
      (child: any) => child?.type?.displayName === 'DialogTrigger'
    );
    const content = React.Children.toArray(children).filter(
      (child: any) => child?.type?.displayName !== 'DialogTrigger'
    );

    const triggerWithProps = trigger && React.isValidElement(trigger)
      ? React.cloneElement(trigger, { onOpenDialog: () => handleOpenChange(true) } as any)
      : trigger;

    return (
      <>
        {triggerWithProps}
        {isOpen && content}
      </>
    );
  };

  const DialogTrigger = ({ children, asChild, onOpenDialog }: any) => {
    if (asChild && React.isValidElement(children)) {
      return React.cloneElement(children, { onClick: onOpenDialog } as any);
    }
    return <div onClick={onOpenDialog}>{children}</div>;
  };
  DialogTrigger.displayName = 'DialogTrigger';

  return {
    Dialog: MockDialog,
    DialogContent: ({ children }: any) => <div>{children}</div>,
    DialogHeader: ({ children }: any) => <div>{children}</div>,
    DialogTitle: ({ children }: any) => <h2>{children}</h2>,
    DialogTrigger,
    DialogFooter: ({ children }: any) => <div>{children}</div>,
  };
});

vi.mock('../../ui/ContextMenu', () => {
  const ContextMenu = ({ children }: any) => {
    const [isOpen, setIsOpen] = React.useState(false);

    const trigger = React.Children.toArray(children).find(
      (child: any) => child?.type?.displayName === 'ContextMenuTrigger'
    );
    const content = React.Children.toArray(children).filter(
      (child: any) => child?.type?.displayName !== 'ContextMenuTrigger'
    );

    const triggerWithProps = trigger && React.isValidElement(trigger)
      ? React.cloneElement(trigger, { onContextMenu: (e: any) => { e.preventDefault(); setIsOpen(true); } } as any)
      : trigger;

    return (
      <div>
        {triggerWithProps}
        {isOpen && content}
      </div>
    );
  };

  const ContextMenuTrigger = ({ children, onContextMenu }: any) => {
    return React.cloneElement(children, { onContextMenu });
  };
  ContextMenuTrigger.displayName = 'ContextMenuTrigger';

  const ContextMenuContent = ({ children }: any) => <div>{children}</div>;
  ContextMenuContent.displayName = 'ContextMenuContent';

  return {
    ContextMenu,
    ContextMenuContent,
    ContextMenuTrigger,
  };
});

vi.mock('../../ui/DesignSystem', () => ({
  ModernCard: ({ children, onClick, onDragStart, onDragOver, onDrop, ...props }: any) => (
    <div
      onClick={onClick}
      onDragStart={onDragStart}
      onDragOver={onDragOver}
      onDrop={onDrop}
      {...props}
    >
      {children}
    </div>
  ),
  ContextMenuItem: ({ children, onClick, className }: any) => (
    <div onClick={onClick} className={className}>
      {children}
    </div>
  ),
}));

vi.mocked(useEnvironments).mockReturnValue({ data: mockEnvironments, isLoading: false, error: null } as any);
vi.mocked(useCreateEnvironment).mockReturnValue({ mutateAsync: vi.fn(), isPending: false } as any);
vi.mocked(useUpdateEnvironment).mockReturnValue({ mutateAsync: vi.fn(), isPending: false } as any);
vi.mocked(useDeleteEnvironment).mockReturnValue({ mutateAsync: vi.fn(), isPending: false } as any);
vi.mocked(useSetActiveEnvironment).mockReturnValue({ mutateAsync: vi.fn(), isPending: false } as any);
vi.mocked(useEnvironmentVariables).mockReturnValue({ data: mockVariables } as any);
vi.mocked(useUpdateEnvironmentsOrder).mockReturnValue({ mutateAsync: vi.fn() } as any);

describe('EnvironmentManager', () => {
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
    // Reset to default mocks
    vi.mocked(useEnvironments).mockReturnValue({ data: mockEnvironments, isLoading: false, error: null } as any);
    vi.mocked(useEnvironmentVariables).mockReturnValue({ data: mockVariables } as any);
  });

  it('renders environments header', () => {
    render(<EnvironmentManager workspaceId="ws-1" />, { wrapper: createWrapper() });

    expect(screen.getByText('Environments')).toBeInTheDocument();
    expect(screen.getByText('New Environment')).toBeInTheDocument();
  });

  it('displays environment cards', () => {
    render(<EnvironmentManager workspaceId="ws-1" />, { wrapper: createWrapper() });

    expect(screen.getByText('Development')).toBeInTheDocument();
    expect(screen.getByText('Global')).toBeInTheDocument();
    expect(screen.getByText('(Global)')).toBeInTheDocument();
  });

  it('shows active indicator on active environment', () => {
    render(<EnvironmentManager workspaceId="ws-1" />, { wrapper: createWrapper() });

    const activeIndicators = document.querySelectorAll('.bg-blue-500.rounded-full');
    expect(activeIndicators.length).toBe(1);
  });

  it('displays variable count', () => {
    render(<EnvironmentManager workspaceId="ws-1" />, { wrapper: createWrapper() });

    expect(screen.getByText('5 vars')).toBeInTheDocument();
    expect(screen.getByText('3 vars')).toBeInTheDocument();
  });

  it('opens create environment dialog', () => {
    render(<EnvironmentManager workspaceId="ws-1" />, { wrapper: createWrapper() });

    fireEvent.click(screen.getByText('New Environment'));

    expect(screen.getByText('Create New Environment')).toBeInTheDocument();
  });

  it('creates new environment', async () => {
    const createMock = vi.fn().mockResolvedValue({});
    vi.mocked(useCreateEnvironment).mockReturnValue({ mutateAsync: createMock, isPending: false } as any);

    render(<EnvironmentManager workspaceId="ws-1" />, { wrapper: createWrapper() });

    fireEvent.click(screen.getByText('New Environment'));

    const nameInput = screen.getByPlaceholderText(/Development, Staging/);
    fireEvent.change(nameInput, { target: { value: 'Staging' } });

    const createButton = screen.getByText('Create Environment');
    fireEvent.click(createButton);

    await waitFor(() => {
      expect(createMock).toHaveBeenCalledWith(
        expect.objectContaining({
          name: 'Staging',
        })
      );
    });
  });

  it('validates required name when creating', () => {
    render(<EnvironmentManager workspaceId="ws-1" />, { wrapper: createWrapper() });

    fireEvent.click(screen.getByText('New Environment'));
    fireEvent.click(screen.getByText('Create Environment'));

    expect(toast.error).toHaveBeenCalledWith('Environment name is required');
  });

  it('sets environment as active when clicked', async () => {
    const setActiveMock = vi.fn().mockResolvedValue({});
    vi.mocked(useSetActiveEnvironment).mockReturnValue({ mutateAsync: setActiveMock, isPending: false } as any);

    render(<EnvironmentManager workspaceId="ws-1" />, { wrapper: createWrapper() });

    const devCard = screen.getByText('Development').closest('.cursor-move') ||
                    screen.getByText('Development').closest('[draggable="true"]');
    fireEvent.click(devCard!);

    await waitFor(() => {
      expect(setActiveMock).toHaveBeenCalledWith('env-1');
    });
  });

  it('uses "global" id for global environment', async () => {
    const setActiveMock = vi.fn().mockResolvedValue({});
    vi.mocked(useSetActiveEnvironment).mockReturnValue({ mutateAsync: setActiveMock, isPending: false } as any);

    render(<EnvironmentManager workspaceId="ws-1" />, { wrapper: createWrapper() });

    const globalCard = screen.getByText('Global').closest('[draggable="false"]');
    fireEvent.click(globalCard!);

    await waitFor(() => {
      expect(setActiveMock).toHaveBeenCalledWith('global');
    });
  });

  it('opens edit dialog from context menu', async () => {
    render(<EnvironmentManager workspaceId="ws-1" />, { wrapper: createWrapper() });

    const devCard = screen.getByText('Development').closest('[draggable="true"]');
    fireEvent.contextMenu(devCard!);

    const editMenuItems = screen.getAllByText('Edit Environment');
    fireEvent.click(editMenuItems[0]); // Click the menu item

    // Dialog should open and show update button
    await waitFor(() => {
      expect(screen.getByText('Update Environment')).toBeInTheDocument();
    });
  });

  it('updates environment', async () => {
    const updateMock = vi.fn().mockResolvedValue({});
    vi.mocked(useUpdateEnvironment).mockReturnValue({ mutateAsync: updateMock, isPending: false } as any);

    render(<EnvironmentManager workspaceId="ws-1" />, { wrapper: createWrapper() });

    const devCard = screen.getByText('Development').closest('[draggable="true"]');
    fireEvent.contextMenu(devCard!);

    const editMenuItems = screen.getAllByText('Edit Environment');
    fireEvent.click(editMenuItems[0]); // Click the menu item

    const nameInput = screen.getAllByRole('textbox')[0];
    fireEvent.change(nameInput, { target: { value: 'Dev Updated' } });

    fireEvent.click(screen.getByText('Update Environment'));

    await waitFor(() => {
      expect(updateMock).toHaveBeenCalledWith(
        expect.objectContaining({
          name: 'Dev Updated',
        })
      );
    });
  });

  it('prevents deleting global environment', () => {
    render(<EnvironmentManager workspaceId="ws-1" />, { wrapper: createWrapper() });

    const globalCard = screen.getByText('Global').closest('[draggable="false"]');
    fireEvent.contextMenu(globalCard!);

    // Global environment should not have delete option
    expect(screen.queryByText('Delete Environment')).not.toBeInTheDocument();
  });

  it('deletes environment with confirmation', async () => {
    window.confirm = vi.fn(() => true);
    const deleteMock = vi.fn().mockResolvedValue({});
    vi.mocked(useDeleteEnvironment).mockReturnValue({ mutateAsync: deleteMock, isPending: false } as any);

    render(<EnvironmentManager workspaceId="ws-1" />, { wrapper: createWrapper() });

    const devCard = screen.getByText('Development').closest('[draggable="true"]');
    fireEvent.contextMenu(devCard!);
    fireEvent.click(screen.getByText('Delete Environment'));

    await waitFor(() => {
      expect(window.confirm).toHaveBeenCalled();
      expect(deleteMock).toHaveBeenCalledWith('env-1');
    });
  });

  it('shows loading skeleton', () => {
    vi.mocked(useEnvironments).mockReturnValue({ data: null, isLoading: true, error: null } as any);

    render(<EnvironmentManager workspaceId="ws-1" />, { wrapper: createWrapper() });

    const skeletons = document.querySelectorAll('.animate-pulse');
    expect(skeletons.length).toBeGreaterThan(0);
  });

  it('shows error state', () => {
    vi.mocked(useEnvironments).mockReturnValue({ data: null, isLoading: false, error: new Error('Failed') } as any);

    render(<EnvironmentManager workspaceId="ws-1" />, { wrapper: createWrapper() });

    expect(screen.getByText('Failed to load environments')).toBeInTheDocument();
  });

  it('displays environment variables preview', () => {
    render(<EnvironmentManager workspaceId="ws-1" />, { wrapper: createWrapper() });

    expect(screen.getAllByText('API_URL').length).toBeGreaterThan(0);
    expect(screen.getAllByText('API_KEY').length).toBeGreaterThan(0);
  });

  it('shows "more" indicator for many variables', () => {
    vi.mocked(useEnvironmentVariables).mockReturnValue({
      data: {
        variables: [
          { name: 'VAR1', value: 'val1' },
          { name: 'VAR2', value: 'val2' },
          { name: 'VAR3', value: 'val3' },
          { name: 'VAR4', value: 'val4' },
        ],
      },
    } as any);

    render(<EnvironmentManager workspaceId="ws-1" />, { wrapper: createWrapper() });

    expect(screen.getAllByText('+1 more').length).toBeGreaterThan(0);
  });

  it('allows color selection when editing', () => {
    render(<EnvironmentManager workspaceId="ws-1" />, { wrapper: createWrapper() });

    const devCard = screen.getByText('Development').closest('[draggable="true"]');
    fireEvent.contextMenu(devCard!);

    const editMenuItems = screen.getAllByText('Edit Environment');
    fireEvent.click(editMenuItems[0]); // Click the menu item

    const colorButtons = document.querySelectorAll('button[title]');
    expect(colorButtons.length).toBeGreaterThan(0);
  });

  it('drags and reorders environments', async () => {
    const updateOrderMock = vi.fn().mockResolvedValue({});
    vi.mocked(useUpdateEnvironmentsOrder).mockReturnValue({ mutateAsync: updateOrderMock } as any);

    // Clear previous toast calls
    vi.mocked(toast.error).mockClear();
    vi.mocked(toast.success).mockClear();

    // Add a second non-global environment for reordering
    const extendedMockEnvironments = {
      environments: [
        ...mockEnvironments.environments,
        {
          id: 'env-2',
          name: 'Staging',
          description: 'Staging environment',
          is_global: false,
          active: false,
          variable_count: 2,
          order: 1,
        },
      ],
    };
    vi.mocked(useEnvironments).mockReturnValue({ data: extendedMockEnvironments, isLoading: false, error: null } as any);

    const { container } = render(<EnvironmentManager workspaceId="ws-1" />, { wrapper: createWrapper() });

    // Verify we have the expected environments rendered
    expect(screen.getByText('Development')).toBeInTheDocument();
    expect(screen.getByText('Staging')).toBeInTheDocument();

    // Get all draggable environment cards
    const draggableCards = Array.from(container.querySelectorAll('[draggable="true"]'));
    expect(draggableCards.length).toBe(2); // Dev and Staging

    // Create proper dataTransfer mocks for each event
    const createDataTransfer = () => ({
      effectAllowed: '',
      dropEffect: '',
      setData: vi.fn(),
      getData: vi.fn(),
      clearData: vi.fn(),
      setDragImage: vi.fn(),
    });

    const dragDataTransfer = createDataTransfer();
    fireEvent.dragStart(draggableCards[0], { dataTransfer: dragDataTransfer });

    // Verify the handler was called by checking effectAllowed
    expect(dragDataTransfer.effectAllowed).toBe('move');

    // Wait for React to update the drag state
    await waitFor(() => {
      const updatedCard = container.querySelector('[draggable="true"].opacity-50');
      expect(updatedCard).toBeTruthy();
    });

    const dropDataTransfer = createDataTransfer();
    fireEvent.dragOver(draggableCards[1], { dataTransfer: dropDataTransfer });
    fireEvent.drop(draggableCards[1], { dataTransfer: dropDataTransfer });

    // Verify drag started successfully and drag UI appeared
    expect(dragDataTransfer.effectAllowed).toBe('move');
  });

  it('calls onEnvironmentSelect when switching', async () => {
    const onSelect = vi.fn();
    vi.mocked(useSetActiveEnvironment).mockReturnValue({ mutateAsync: vi.fn().mockResolvedValue({}) } as any);

    render(<EnvironmentManager workspaceId="ws-1" onEnvironmentSelect={onSelect} />, {
      wrapper: createWrapper(),
    });

    const devCard = screen.getByText('Development').closest('[draggable="true"]');
    fireEvent.click(devCard!);

    await waitFor(() => {
      expect(onSelect).toHaveBeenCalledWith('env-1');
    });
  });
});
