/**
 * @jest-environment jsdom
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { FixturesPage } from '../FixturesPage';
import * as apiHooks from '../../hooks/api';
import type { FixtureInfo } from '../../services/api';

const mockFixtures: FixtureInfo[] = [
  {
    id: 'fixture-1',
    name: 'fixture-1',
    path: '/api/users',
    method: 'GET',
    protocol: 'http',
    description: 'User listing',
    tags: ['auth', 'users'],
    createdAt: '2024-01-01T00:00:00Z',
    updatedAt: '2024-01-01T00:00:00Z',
    saved_at: '2024-01-01T00:00:00Z',
    file_size: 1024,
    file_path: '/path/to/fixture1.json',
    fingerprint: 'abc123',
    metadata: {},
  },
  {
    id: 'fixture-2',
    name: 'fixture-2',
    path: '/api/posts',
    method: 'POST',
    protocol: 'http',
    description: '',
    tags: [],
    createdAt: '2024-01-02T00:00:00Z',
    updatedAt: '2024-01-02T00:00:00Z',
    saved_at: '2024-01-02T00:00:00Z',
    file_size: 2048,
    file_path: '/path/to/fixture2.json',
    fingerprint: 'def456',
    metadata: {},
  },
];

const mutationState = (overrides: Record<string, unknown> = {}) => ({
  mutate: vi.fn(),
  mutateAsync: vi.fn().mockResolvedValue(undefined),
  isPending: false,
  reset: vi.fn(),
  ...overrides,
});

const createFixtureMutate = vi.fn().mockResolvedValue(mockFixtures[0]);
const updateFixtureMutate = vi.fn().mockResolvedValue(mockFixtures[0]);
const deleteFixtureMutate = vi.fn().mockResolvedValue(undefined);
const renameFixtureMutate = vi.fn().mockResolvedValue(undefined);
const moveFixtureMutate = vi.fn().mockResolvedValue(undefined);
const downloadFixtureMutate = vi.fn().mockResolvedValue({
  blob: new Blob(['{}'], { type: 'application/json' }),
  filename: 'f.json',
});

vi.mock('../../hooks/api', () => ({
  useFixtures: vi.fn(() => ({
    data: mockFixtures,
    isLoading: false,
    error: null,
    refetch: vi.fn(),
    isFetching: false,
  })),
  useCreateFixture: vi.fn(() => mutationState({ mutateAsync: createFixtureMutate })),
  useUpdateFixture: vi.fn(() => mutationState({ mutateAsync: updateFixtureMutate })),
  useDeleteFixture: vi.fn(() => mutationState({ mutateAsync: deleteFixtureMutate })),
  useRenameFixture: vi.fn(() => mutationState({ mutateAsync: renameFixtureMutate })),
  useMoveFixture: vi.fn(() => mutationState({ mutateAsync: moveFixtureMutate })),
  useDownloadFixture: vi.fn(() => mutationState({ mutateAsync: downloadFixtureMutate })),
}));

vi.mock('sonner', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

describe('FixturesPage', () => {
  const mockUseFixtures = vi.mocked(apiHooks.useFixtures);

  const createWrapper = () => {
    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    return ({ children }: { children: React.ReactNode }) => (
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    );
  };

  beforeEach(() => {
    vi.clearAllMocks();
    mockUseFixtures.mockReturnValue({
      data: mockFixtures,
      isLoading: false,
      error: null,
      refetch: vi.fn(),
      isFetching: false,
    } as any);
  });

  it('renders loading state', () => {
    mockUseFixtures.mockReturnValue({
      data: null,
      isLoading: true,
      error: null,
      refetch: vi.fn(),
      isFetching: false,
    } as any);

    render(<FixturesPage />, { wrapper: createWrapper() });
    expect(screen.getByText('Loading fixtures...')).toBeInTheDocument();
  });

  it('displays fixtures list', () => {
    render(<FixturesPage />, { wrapper: createWrapper() });

    expect(screen.getByText('fixture-1')).toBeInTheDocument();
    expect(screen.getByText('fixture-2')).toBeInTheDocument();
  });

  it('shows fixture metadata including description and tags', () => {
    render(<FixturesPage />, { wrapper: createWrapper() });

    expect(screen.getByText('Path: /api/users')).toBeInTheDocument();
    expect(screen.getByText('Path: /api/posts')).toBeInTheDocument();
    expect(screen.getByText('User listing')).toBeInTheDocument();
    expect(screen.getAllByText('auth').length).toBeGreaterThan(0);
    expect(screen.getAllByText('users').length).toBeGreaterThan(0);
  });

  it('displays method badges', () => {
    render(<FixturesPage />, { wrapper: createWrapper() });

    expect(screen.getAllByText('GET').length).toBeGreaterThan(0);
    expect(screen.getAllByText('POST').length).toBeGreaterThan(0);
  });

  it('filters fixtures by search term (matches description/tags/path)', () => {
    render(<FixturesPage />, { wrapper: createWrapper() });

    const searchInput = screen.getByPlaceholderText(/Search by name, path, tag/);
    fireEvent.change(searchInput, { target: { value: 'users' } });

    expect(screen.getByText('fixture-1')).toBeInTheDocument();
    expect(screen.queryByText('fixture-2')).not.toBeInTheDocument();
  });

  it('filters fixtures by HTTP method', () => {
    render(<FixturesPage />, { wrapper: createWrapper() });

    const methodSelect = screen.getAllByRole('combobox')[0];
    fireEvent.change(methodSelect, { target: { value: 'GET' } });

    expect(screen.getByText('fixture-1')).toBeInTheDocument();
    expect(screen.queryByText('fixture-2')).not.toBeInTheDocument();
  });

  it('filters fixtures by tag', () => {
    render(<FixturesPage />, { wrapper: createWrapper() });

    const selects = screen.getAllByRole('combobox');
    const tagSelect = selects[1];
    fireEvent.change(tagSelect, { target: { value: 'auth' } });

    expect(screen.getByText('fixture-1')).toBeInTheDocument();
    expect(screen.queryByText('fixture-2')).not.toBeInTheDocument();
  });

  it('displays empty state when no fixtures exist', () => {
    mockUseFixtures.mockReturnValue({
      data: [],
      isLoading: false,
      error: null,
      refetch: vi.fn(),
      isFetching: false,
    } as any);

    render(<FixturesPage />, { wrapper: createWrapper() });

    expect(screen.getByText('No fixtures found')).toBeInTheDocument();
    expect(screen.getByText(/No fixtures have been created yet/)).toBeInTheDocument();
  });

  it('displays empty state when search returns no results', () => {
    render(<FixturesPage />, { wrapper: createWrapper() });

    const searchInput = screen.getByPlaceholderText(/Search by name, path, tag/);
    fireEvent.change(searchInput, { target: { value: 'nonexistent' } });

    expect(screen.getByText('No fixtures found')).toBeInTheDocument();
    expect(screen.getByText(/No fixtures match your current search criteria/)).toBeInTheDocument();
  });

  it('handles error state', () => {
    mockUseFixtures.mockReturnValue({
      data: null,
      isLoading: false,
      error: new Error('Failed to load fixtures'),
      refetch: vi.fn(),
      isFetching: false,
    } as any);

    render(<FixturesPage />, { wrapper: createWrapper() });

    expect(screen.getAllByText('Failed to load fixtures').length).toBeGreaterThan(0);
  });

  it('opens view dialog when clicking view button', () => {
    render(<FixturesPage />, { wrapper: createWrapper() });

    const eyeButton = document.querySelector('svg.lucide-eye')?.closest('button');
    fireEvent.click(eyeButton!);

    expect(screen.getByText('Response Content')).toBeInTheDocument();
  });

  it('invokes download mutation with the selected fixture', async () => {
    vi.spyOn(URL, 'createObjectURL').mockReturnValue('blob:fixture');
    vi.spyOn(URL, 'revokeObjectURL').mockImplementation(() => {});

    render(<FixturesPage />, { wrapper: createWrapper() });

    const firstFixtureRow = screen
      .getByText('fixture-1')
      .closest('.flex.items-center.justify-between');
    const downloadBtn = firstFixtureRow?.querySelector('svg.lucide-download')?.closest('button');
    expect(downloadBtn).toBeTruthy();
    fireEvent.click(downloadBtn as HTMLElement);

    await waitFor(() => {
      expect(downloadFixtureMutate).toHaveBeenCalledWith(mockFixtures[0]);
    });
  });

  it('opens rename dialog', () => {
    render(<FixturesPage />, { wrapper: createWrapper() });

    const renameButton = screen.getAllByText('Rename')[0];
    fireEvent.click(renameButton);

    expect(screen.getByText('Rename Fixture')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('Enter new fixture name')).toBeInTheDocument();
  });

  it('renames fixture through the mutation', async () => {
    render(<FixturesPage />, { wrapper: createWrapper() });

    const renameButton = screen.getAllByText('Rename')[0];
    fireEvent.click(renameButton);

    const input = screen.getByPlaceholderText('Enter new fixture name');
    fireEvent.change(input, { target: { value: 'new-fixture-name' } });

    const confirmButton = screen.getAllByRole('button', { name: 'Rename' }).at(-1)!;
    fireEvent.click(confirmButton);

    await waitFor(() => {
      expect(renameFixtureMutate).toHaveBeenCalledWith({
        fixtureId: 'fixture-1',
        newName: 'new-fixture-name',
      });
    });
  });

  it('opens move dialog', () => {
    render(<FixturesPage />, { wrapper: createWrapper() });

    const moveButton = screen.getAllByText('Move')[0];
    fireEvent.click(moveButton);

    expect(screen.getByText('Move Fixture')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('Enter new path')).toBeInTheDocument();
  });

  it('moves fixture through the mutation', async () => {
    render(<FixturesPage />, { wrapper: createWrapper() });

    const moveButton = screen.getAllByText('Move')[0];
    fireEvent.click(moveButton);

    const input = screen.getByPlaceholderText('Enter new path');
    fireEvent.change(input, { target: { value: '/new/path' } });

    const confirmButton = screen.getAllByRole('button', { name: 'Move' }).at(-1)!;
    fireEvent.click(confirmButton);

    await waitFor(() => {
      expect(moveFixtureMutate).toHaveBeenCalledWith({
        fixtureId: 'fixture-1',
        newPath: '/new/path',
      });
    });
  });

  it('opens delete confirmation dialog', () => {
    render(<FixturesPage />, { wrapper: createWrapper() });

    const deleteBtn = document
      .querySelectorAll('button.text-danger-600')[0] as HTMLElement | undefined;
    fireEvent.click(deleteBtn!);

    expect(screen.getByText('Delete Fixture')).toBeInTheDocument();
    expect(screen.getByText(/Are you sure you want to delete this fixture/)).toBeInTheDocument();
  });

  it('deletes fixture through the mutation', async () => {
    render(<FixturesPage />, { wrapper: createWrapper() });

    const deleteBtn = document
      .querySelectorAll('button.text-danger-600')[0] as HTMLElement | undefined;
    fireEvent.click(deleteBtn!);

    const confirmButton = screen.getByRole('button', { name: 'Delete' });
    fireEvent.click(confirmButton);

    await waitFor(() => {
      expect(deleteFixtureMutate).toHaveBeenCalledWith('fixture-1');
    });
  });

  it('refreshes fixtures list', () => {
    const refetchMock = vi.fn();
    mockUseFixtures.mockReturnValue({
      data: mockFixtures,
      isLoading: false,
      error: null,
      refetch: refetchMock,
      isFetching: false,
    } as any);

    render(<FixturesPage />, { wrapper: createWrapper() });

    const refreshButton = screen.getByText('Refresh');
    fireEvent.click(refreshButton);

    expect(refetchMock).toHaveBeenCalled();
  });

  it('disables rename button when name is unchanged', () => {
    render(<FixturesPage />, { wrapper: createWrapper() });

    const renameButton = screen.getAllByText('Rename')[0];
    fireEvent.click(renameButton);

    const confirmButton = screen.getAllByRole('button', { name: 'Rename' }).at(-1)!;
    expect(confirmButton).toBeDisabled();
  });

  it('disables move button when path is empty', () => {
    render(<FixturesPage />, { wrapper: createWrapper() });

    const moveButton = screen.getAllByText('Move')[0];
    fireEvent.click(moveButton);

    const input = screen.getByPlaceholderText('Enter new path');
    fireEvent.change(input, { target: { value: '' } });

    const confirmButton = screen.getAllByRole('button', { name: 'Move' }).at(-1)!;
    expect(confirmButton).toBeDisabled();
  });
});
