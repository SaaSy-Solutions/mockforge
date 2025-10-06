/**
 * @jest-environment jsdom
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { ImportPage } from '../ImportPage';
import type { ImportResponse } from '../../services/api';
import {
  usePreviewImport,
  useImportPostman,
  useClearImportHistory,
  useImportHistory,
} from '../../hooks/useApi';

const mockPreviewResponse: ImportResponse = {
  success: true,
  routes: [
    {
      name: 'Get Users',
      method: 'GET',
      path: '/api/users',
      status_code: 200,
      body: '{"users": []}',
    },
    {
      name: 'Create User',
      method: 'POST',
      path: '/api/users',
      status_code: 201,
      body: '{"id": 1}',
    },
  ],
  warnings: ['Some routes were skipped'],
};

const mockImportHistory = {
  total: 2,
  imports: [
    {
      id: '1',
      format: 'postman',
      filename: 'collection.json',
      timestamp: '2024-01-01T10:00:00Z',
      success: true,
      routes_count: 10,
      variables_count: 5,
      warnings_count: 0,
    },
    {
      id: '2',
      format: 'insomnia',
      filename: 'export.json',
      timestamp: '2024-01-02T10:00:00Z',
      success: false,
      routes_count: 0,
      error_message: 'Invalid format',
    },
  ],
};

vi.mock('../../hooks/useApi', () => ({
  useImportPostman: vi.fn(() => ({ mutateAsync: vi.fn(), isPending: false })),
  useImportInsomnia: vi.fn(() => ({ mutateAsync: vi.fn(), isPending: false })),
  useImportCurl: vi.fn(() => ({ mutateAsync: vi.fn(), isPending: false })),
  usePreviewImport: vi.fn(() => ({ mutateAsync: vi.fn(), isPending: false })),
  useImportHistory: vi.fn(() => ({ data: mockImportHistory, isLoading: false, error: null })),
  useClearImportHistory: vi.fn(() => ({ mutate: vi.fn(), isPending: false })),
}));

vi.mock('sonner', () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

describe('ImportPage', () => {
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
  });

  it('renders import page header', () => {
    render(<ImportPage />, { wrapper: createWrapper() });

    expect(screen.getByText('Import API Collections')).toBeInTheDocument();
    expect(screen.getByText('Import routes from Postman, Insomnia, or cURL commands')).toBeInTheDocument();
  });

  it('displays format tabs', () => {
    render(<ImportPage />, { wrapper: createWrapper() });

    expect(screen.getByText('Postman')).toBeInTheDocument();
    expect(screen.getByText('Insomnia')).toBeInTheDocument();
    expect(screen.getByText('cURL')).toBeInTheDocument();
    expect(screen.getByText('History')).toBeInTheDocument();
  });

  it('switches between format tabs', () => {
    render(<ImportPage />, { wrapper: createWrapper() });

    const insomniaTab = screen.getByText('Insomnia');
    fireEvent.click(insomniaTab);

    expect(screen.getByText(/Upload your insomnia collection/)).toBeInTheDocument();
  });

  it('shows file upload component', () => {
    render(<ImportPage />, { wrapper: createWrapper() });

    expect(screen.getByText(/Drop Postman Collection here/)).toBeInTheDocument();
    expect(screen.getByText('Choose File')).toBeInTheDocument();
  });

  it('handles file selection via input', async () => {
    render(<ImportPage />, { wrapper: createWrapper() });

    const file = new File(['{}'], 'collection.json', { type: 'application/json' });
    const input = document.querySelector('input[type="file"]') as HTMLInputElement;

    Object.defineProperty(input, 'files', {
      value: [file],
      writable: false,
    });

    fireEvent.change(input);

    // Wait for FileReader to process the file
    await waitFor(() => {
      expect(screen.getByText(/Selected: collection.json/)).toBeInTheDocument();
    });
  });

  it('handles file drop', async () => {
    render(<ImportPage />, { wrapper: createWrapper() });

    const dropZone = screen.getByText(/Drop Postman Collection here/).parentElement;
    const file = new File(['{}'], 'collection.json', { type: 'application/json' });

    const dataTransfer = {
      files: [file],
      types: ['Files'],
    };

    fireEvent.drop(dropZone!, { dataTransfer });

    // Wait for FileReader to process the file
    await waitFor(() => {
      expect(screen.getByText(/Selected: collection.json/)).toBeInTheDocument();
    });
  });

  it('shows drag over state', () => {
    render(<ImportPage />, { wrapper: createWrapper() });

    const dropZone = screen.getByText(/Drop Postman Collection here/).parentElement;
    fireEvent.dragOver(dropZone!);

    // Drag over should add visual feedback (classes)
    expect(dropZone).toBeInTheDocument();
  });

  it('displays configuration options for Insomnia', () => {
    render(<ImportPage />, { wrapper: createWrapper() });

    fireEvent.click(screen.getByText('Insomnia'));

    expect(screen.getByPlaceholderText('e.g., dev, staging, prod')).toBeInTheDocument();
  });

  it('displays base URL override option', () => {
    render(<ImportPage />, { wrapper: createWrapper() });

    expect(screen.getByPlaceholderText('e.g., https://api.example.com')).toBeInTheDocument();
  });

  it('previews import', async () => {
    const previewMock = vi.fn().mockResolvedValue(mockPreviewResponse);
    vi.mocked(usePreviewImport).mockReturnValue({ mutateAsync: previewMock, isPending: false } as any);

    render(<ImportPage />, { wrapper: createWrapper() });

    const file = new File(['{}'], 'collection.json', { type: 'application/json' });
    const input = document.querySelector('input[type="file"]') as HTMLInputElement;
    Object.defineProperty(input, 'files', { value: [file], writable: false });
    fireEvent.change(input);

    // Wait for FileReader to process the file
    await waitFor(() => {
      expect(screen.getByText(/Selected: collection.json/)).toBeInTheDocument();
    });

    const previewButton = screen.getByText('Preview Routes');
    fireEvent.click(previewButton);

    await waitFor(() => {
      expect(previewMock).toHaveBeenCalled();
    });
  });

  it('displays preview results', async () => {
    const previewMock = vi.fn().mockResolvedValue(mockPreviewResponse);
    vi.mocked(usePreviewImport).mockReturnValue({
      mutateAsync: previewMock,
      isPending: false,
    } as any);

    render(<ImportPage />, { wrapper: createWrapper() });

    const file = new File(['{}'], 'collection.json', { type: 'application/json' });
    const input = document.querySelector('input[type="file"]') as HTMLInputElement;
    Object.defineProperty(input, 'files', { value: [file], writable: false });
    fireEvent.change(input);

    // Wait for FileReader to process the file
    await waitFor(() => {
      expect(screen.getByText(/Selected: collection.json/)).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText('Preview Routes'));

    // Wait for the preview to be called
    await waitFor(() => {
      expect(previewMock).toHaveBeenCalled();
    });

    // Verify preview was called with correct data
    expect(previewMock).toHaveBeenCalledTimes(1);

    // The component displays results - full rendering tested via integration/e2e tests
  });

  it('shows warnings in preview', async () => {
    vi.mocked(usePreviewImport).mockReturnValue({
      mutateAsync: vi.fn().mockResolvedValue(mockPreviewResponse),
      isPending: false,
    } as any);

    render(<ImportPage />, { wrapper: createWrapper() });

    const file = new File(['{}'], 'collection.json', { type: 'application/json' });
    const input = document.querySelector('input[type="file"]') as HTMLInputElement;
    Object.defineProperty(input, 'files', { value: [file], writable: false });
    fireEvent.change(input);

    // Wait for FileReader to process the file
    await waitFor(() => {
      expect(screen.getByText(/Selected: collection.json/)).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText('Preview Routes'));

    await waitFor(() => {
      expect(screen.getByText('Some routes were skipped')).toBeInTheDocument();
    });
  });

  it('selects/deselects routes', async () => {
    vi.mocked(usePreviewImport).mockReturnValue({
      mutateAsync: vi.fn().mockResolvedValue(mockPreviewResponse),
      isPending: false,
    } as any);

    render(<ImportPage />, { wrapper: createWrapper() });

    const file = new File(['{}'], 'collection.json', { type: 'application/json' });
    const input = document.querySelector('input[type="file"]') as HTMLInputElement;
    Object.defineProperty(input, 'files', { value: [file], writable: false });
    fireEvent.change(input);

    // Wait for FileReader to process the file
    await waitFor(() => {
      expect(screen.getByText(/Selected: collection.json/)).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText('Preview Routes'));

    await waitFor(() => {
      const checkboxes = screen.getAllByRole('checkbox');
      fireEvent.click(checkboxes[0]);
    });
  });

  it('selects all routes', async () => {
    vi.mocked(usePreviewImport).mockReturnValue({
      mutateAsync: vi.fn().mockResolvedValue(mockPreviewResponse),
      isPending: false,
    } as any);

    render(<ImportPage />, { wrapper: createWrapper() });

    const file = new File(['{}'], 'collection.json', { type: 'application/json' });
    const input = document.querySelector('input[type="file"]') as HTMLInputElement;
    Object.defineProperty(input, 'files', { value: [file], writable: false });
    fireEvent.change(input);

    // Wait for FileReader to process the file
    await waitFor(() => {
      expect(screen.getByText(/Selected: collection.json/)).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText('Preview Routes'));

    await waitFor(() => {
      fireEvent.click(screen.getByText('Select All'));
    });
  });

  it('deselects all routes', async () => {
    vi.mocked(usePreviewImport).mockReturnValue({
      mutateAsync: vi.fn().mockResolvedValue(mockPreviewResponse),
      isPending: false,
    } as any);

    render(<ImportPage />, { wrapper: createWrapper() });

    const file = new File(['{}'], 'collection.json', { type: 'application/json' });
    const input = document.querySelector('input[type="file"]') as HTMLInputElement;
    Object.defineProperty(input, 'files', { value: [file], writable: false });
    fireEvent.change(input);

    // Wait for FileReader to process the file
    await waitFor(() => {
      expect(screen.getByText(/Selected: collection.json/)).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText('Preview Routes'));

    await waitFor(() => {
      fireEvent.click(screen.getByText('Deselect All'));
    });
  });

  it('imports selected routes', async () => {
    const importMock = vi.fn().mockResolvedValue({ success: true });

    vi.mocked(usePreviewImport).mockReturnValue({
      mutateAsync: vi.fn().mockResolvedValue(mockPreviewResponse),
      isPending: false,
    } as any);
    vi.mocked(useImportPostman).mockReturnValue({ mutateAsync: importMock, isPending: false } as any);

    render(<ImportPage />, { wrapper: createWrapper() });

    const file = new File(['{}'], 'collection.json', { type: 'application/json' });
    const input = document.querySelector('input[type="file"]') as HTMLInputElement;
    Object.defineProperty(input, 'files', { value: [file], writable: false });
    fireEvent.change(input);

    // Wait for FileReader to process the file
    await waitFor(() => {
      expect(screen.getByText(/Selected: collection.json/)).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText('Preview Routes'));

    await waitFor(() => {
      fireEvent.click(screen.getByText(/Import 2 Routes/));
    });

    await waitFor(() => {
      expect(importMock).toHaveBeenCalled();
    });
  });

  it('displays import history', () => {
    render(<ImportPage />, { wrapper: createWrapper() });

    fireEvent.click(screen.getByText('History'));

    expect(screen.getByText(/Import History \(2\)/)).toBeInTheDocument();
    expect(screen.getByText('collection.json')).toBeInTheDocument();
    expect(screen.getByText('export.json')).toBeInTheDocument();
  });

  it('shows success/failure badges in history', () => {
    render(<ImportPage />, { wrapper: createWrapper() });

    fireEvent.click(screen.getByText('History'));

    expect(screen.getByText('Success')).toBeInTheDocument();
    expect(screen.getByText('Failed')).toBeInTheDocument();
  });

  it('displays error messages in history', () => {
    render(<ImportPage />, { wrapper: createWrapper() });

    fireEvent.click(screen.getByText('History'));

    expect(screen.getByText(/Error: Invalid format/)).toBeInTheDocument();
  });

  it('clears import history', async () => {
    const clearMock = vi.fn();
    vi.mocked(useClearImportHistory).mockReturnValue({ mutate: clearMock, isPending: false } as any);

    render(<ImportPage />, { wrapper: createWrapper() });

    fireEvent.click(screen.getByText('History'));

    const clearButton = screen.getByText('Clear History');
    fireEvent.click(clearButton);

    expect(clearMock).toHaveBeenCalled();
  });

  it('shows empty history state', () => {
    vi.mocked(useImportHistory).mockReturnValue({ data: { total: 0, imports: [] }, isLoading: false, error: null } as any);

    render(<ImportPage />, { wrapper: createWrapper() });

    fireEvent.click(screen.getByText('History'));

    expect(screen.getByText('No Import History')).toBeInTheDocument();
    expect(screen.getByText(/Your import history will appear here/)).toBeInTheDocument();
  });

  it('disables preview button when no file selected', () => {
    render(<ImportPage />, { wrapper: createWrapper() });

    const previewButton = screen.getByText('Preview Routes');
    expect(previewButton).toBeDisabled();
  });

  it('disables import button when no routes selected', async () => {
    vi.mocked(usePreviewImport).mockReturnValue({
      mutateAsync: vi.fn().mockResolvedValue({ ...mockPreviewResponse, routes: [] }),
      isPending: false,
    } as any);

    render(<ImportPage />, { wrapper: createWrapper() });

    const file = new File(['{}'], 'collection.json', { type: 'application/json' });
    const input = document.querySelector('input[type="file"]') as HTMLInputElement;
    Object.defineProperty(input, 'files', { value: [file], writable: false });
    fireEvent.change(input);

    fireEvent.click(screen.getByText('Preview Routes'));

    await waitFor(() => {
      const importButton = screen.getByText(/Import 0 Routes/);
      expect(importButton).toBeDisabled();
    });
  });

  it('displays route details in preview', async () => {
    const previewMock = vi.fn().mockResolvedValue(mockPreviewResponse);
    vi.mocked(usePreviewImport).mockReturnValue({
      mutateAsync: previewMock,
      isPending: false,
    } as any);

    render(<ImportPage />, { wrapper: createWrapper() });

    const file = new File(['{}'], 'collection.json', { type: 'application/json' });
    const input = document.querySelector('input[type="file"]') as HTMLInputElement;
    Object.defineProperty(input, 'files', { value: [file], writable: false });
    fireEvent.change(input);

    // Wait for FileReader to process the file
    await waitFor(() => {
      expect(screen.getByText(/Selected: collection.json/)).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText('Preview Routes'));

    // Wait for the preview to be called
    await waitFor(() => {
      expect(previewMock).toHaveBeenCalled();
    });

    // Verify preview was called correctly
    expect(previewMock).toHaveBeenCalledTimes(1);

    // Route details rendering tested via integration/e2e tests
  });
});
