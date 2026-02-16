/**
 * @jest-environment jsdom
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { FixturesPage } from '../FixturesPage';
import * as apiHooks from '../../hooks/useApi';
import type { FixtureInfo } from '../../services/api';

const mockFixtures: FixtureInfo[] = [
  {
    id: 'fixture-1',
    path: '/api/users',
    method: 'GET',
    protocol: 'REST',
    saved_at: '2024-01-01T00:00:00Z',
    file_size: 1024,
    file_path: '/path/to/fixture1.json',
    fingerprint: 'abc123',
    metadata: {},
  },
  {
    id: 'fixture-2',
    path: '/api/posts',
    method: 'POST',
    protocol: 'REST',
    saved_at: '2024-01-02T00:00:00Z',
    file_size: 2048,
    file_path: '/path/to/fixture2.json',
    fingerprint: 'def456',
    metadata: {},
  },
];

vi.mock('../../hooks/useApi', () => ({
  useFixtures: vi.fn(() => ({
    data: mockFixtures,
    isLoading: false,
    error: null,
    refetch: vi.fn(),
  })),
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
    } as any);
  });

  it('renders loading state', () => {
    mockUseFixtures.mockReturnValue({ data: null, isLoading: true, error: null, refetch: vi.fn() } as any);

    render(<FixturesPage />, { wrapper: createWrapper() });
    expect(screen.getByText('Loading fixtures...')).toBeInTheDocument();
  });

  it('displays fixtures list', () => {
    render(<FixturesPage />, { wrapper: createWrapper() });

    expect(screen.getByText('fixture-1')).toBeInTheDocument();
    expect(screen.getByText('fixture-2')).toBeInTheDocument();
  });

  it('shows fixture metadata', () => {
    render(<FixturesPage />, { wrapper: createWrapper() });

    expect(screen.getByText('Path: /api/users')).toBeInTheDocument();
    expect(screen.getByText('Path: /api/posts')).toBeInTheDocument();
    expect(screen.getByText('1 KB')).toBeInTheDocument();
    expect(screen.getByText('2 KB')).toBeInTheDocument();
  });

  it('displays method badges', () => {
    render(<FixturesPage />, { wrapper: createWrapper() });

    expect(screen.getAllByText('GET').length).toBeGreaterThan(0);
    expect(screen.getAllByText('POST').length).toBeGreaterThan(0);
  });

  it('filters fixtures by search term', () => {
    render(<FixturesPage />, { wrapper: createWrapper() });

    const searchInput = screen.getByPlaceholderText('Search by name, path, or route...');
    fireEvent.change(searchInput, { target: { value: 'users' } });

    expect(screen.getByText('fixture-1')).toBeInTheDocument();
    expect(screen.queryByText('fixture-2')).not.toBeInTheDocument();
  });

  it('filters fixtures by HTTP method', () => {
    render(<FixturesPage />, { wrapper: createWrapper() });

    const methodSelect = screen.getByRole('combobox');
    fireEvent.change(methodSelect, { target: { value: 'GET' } });

    expect(screen.getByText('fixture-1')).toBeInTheDocument();
    expect(screen.queryByText('fixture-2')).not.toBeInTheDocument();
  });

  it('shows total fixture count and size', () => {
    render(<FixturesPage />, { wrapper: createWrapper() });

    expect(screen.getByText('2')).toBeInTheDocument(); // count
    expect(screen.getByText('3 KB')).toBeInTheDocument(); // total size
  });

  it('displays empty state when no fixtures exist', () => {
    mockUseFixtures.mockReturnValue({ data: [], isLoading: false, error: null, refetch: vi.fn() } as any);

    render(<FixturesPage />, { wrapper: createWrapper() });

    expect(screen.getByText('No fixtures found')).toBeInTheDocument();
    expect(screen.getByText(/No fixtures have been created yet/)).toBeInTheDocument();
  });

  it('displays empty state when search returns no results', () => {
    render(<FixturesPage />, { wrapper: createWrapper() });

    const searchInput = screen.getByPlaceholderText('Search by name, path, or route...');
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
    } as any);

    render(<FixturesPage />, { wrapper: createWrapper() });

    expect(screen.getAllByText('Failed to load fixtures').length).toBeGreaterThan(0);
  });

  it('opens view dialog when clicking view button', () => {
    render(<FixturesPage />, { wrapper: createWrapper() });

    const eyeButton = document.querySelector('svg.lucide-eye')?.closest('button');
    fireEvent.click(eyeButton!);

    expect(screen.getByText('Metadata')).toBeInTheDocument();
  });

  it('downloads fixture when clicking download button', async () => {
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      blob: vi.fn().mockResolvedValue(new Blob(['{}'], { type: 'application/json' })),
      headers: { get: vi.fn().mockReturnValue('attachment; filename="fixture-1.json"') },
    });
    vi.spyOn(URL, 'createObjectURL').mockReturnValue('blob:fixture');
    vi.spyOn(URL, 'revokeObjectURL').mockImplementation(() => {});
    const createElementSpy = vi.spyOn(document, 'createElement');
    render(<FixturesPage />, { wrapper: createWrapper() });

    const firstFixtureRow = screen.getByText('fixture-1').closest('.flex.items-center.justify-between');
    const downloadBtn = firstFixtureRow?.querySelector('svg.lucide-download')?.closest('button');
    expect(downloadBtn).toBeTruthy();
    fireEvent.click(downloadBtn);

    await waitFor(() => {
      expect(createElementSpy).toHaveBeenCalledWith('a');
    });
  });

  it('opens rename dialog', () => {
    render(<FixturesPage />, { wrapper: createWrapper() });

    const renameButton = screen.getAllByText('Rename')[0];
    fireEvent.click(renameButton);

    expect(screen.getByText('Rename Fixture')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('Enter new fixture name')).toBeInTheDocument();
  });

  it('renames fixture successfully', async () => {
    global.fetch = vi.fn().mockResolvedValue({ ok: true });

    render(<FixturesPage />, { wrapper: createWrapper() });

    const renameButton = screen.getAllByText('Rename')[0];
    fireEvent.click(renameButton);

    const input = screen.getByPlaceholderText('Enter new fixture name');
    fireEvent.change(input, { target: { value: 'new-fixture-name' } });

    const confirmButton = screen.getAllByRole('button', { name: 'Rename' }).at(-1)!;
    fireEvent.click(confirmButton);

    await waitFor(() => {
      expect(global.fetch).toHaveBeenCalledWith(
        '/__mockforge/fixtures/fixture-1/rename',
        expect.objectContaining({
          method: 'PUT',
          body: JSON.stringify({ new_name: 'new-fixture-name' }),
        })
      );
    });
  });

  it('opens move dialog', () => {
    render(<FixturesPage />, { wrapper: createWrapper() });

    const moveButton = screen.getAllByText('Move')[0];
    fireEvent.click(moveButton);

    expect(screen.getByText('Move Fixture')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('Enter new path')).toBeInTheDocument();
  });

  it('moves fixture successfully', async () => {
    global.fetch = vi.fn().mockResolvedValue({ ok: true });

    render(<FixturesPage />, { wrapper: createWrapper() });

    const moveButton = screen.getAllByText('Move')[0];
    fireEvent.click(moveButton);

    const input = screen.getByPlaceholderText('Enter new path');
    fireEvent.change(input, { target: { value: '/new/path' } });

    const confirmButton = screen.getAllByRole('button', { name: 'Move' }).at(-1)!;
    fireEvent.click(confirmButton);

    await waitFor(() => {
      expect(global.fetch).toHaveBeenCalledWith(
        '/__mockforge/fixtures/fixture-1/move',
        expect.objectContaining({
          method: 'PUT',
          body: JSON.stringify({ new_path: '/new/path' }),
        })
      );
    });
  });

  it('opens delete confirmation dialog', () => {
    render(<FixturesPage />, { wrapper: createWrapper() });

    const deleteButtons = screen.getAllByRole('button');
    // Delete button is usually the last one or has a trash icon
    const deleteBtn = deleteButtons[deleteButtons.length - 1];
    fireEvent.click(deleteBtn);

    expect(screen.getByText('Delete Fixture')).toBeInTheDocument();
    expect(screen.getByText(/Are you sure you want to delete this fixture/)).toBeInTheDocument();
  });

  it('deletes fixture successfully', async () => {
    global.fetch = vi.fn().mockResolvedValue({ ok: true });

    render(<FixturesPage />, { wrapper: createWrapper() });

    const deleteButtons = Array.from(document.querySelectorAll('button.text-red-600'));
    const deleteBtn = deleteButtons[0];
    fireEvent.click(deleteBtn);

    const confirmButton = screen.getByRole('button', { name: 'Delete' });
    fireEvent.click(confirmButton);

    await waitFor(() => {
      expect(global.fetch).toHaveBeenCalledWith('/__mockforge/fixtures/fixture-1', {
        method: 'DELETE',
      });
    });
  });

  it('refreshes fixtures list', () => {
    const refetchMock = vi.fn();
    mockUseFixtures.mockReturnValue({
      data: mockFixtures,
      isLoading: false,
      error: null,
      refetch: refetchMock,
    } as any);

    render(<FixturesPage />, { wrapper: createWrapper() });

    const refreshButton = screen.getByText('Refresh');
    fireEvent.click(refreshButton);

    expect(refetchMock).toHaveBeenCalled();
  });

  it('formats file size correctly', () => {
    render(<FixturesPage />, { wrapper: createWrapper() });

    expect(screen.getByText('1 KB')).toBeInTheDocument();
    expect(screen.getByText('2 KB')).toBeInTheDocument();
  });

  it('formats dates correctly', () => {
    render(<FixturesPage />, { wrapper: createWrapper() });

    // Check that dates are displayed
    expect(screen.getByText(/Jan/)).toBeInTheDocument();
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

    const confirmButton = screen.getAllByRole('button', { name: 'Move' }).at(-1)!;
    expect(confirmButton).toBeDisabled();
  });
});
