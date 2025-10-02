/**
 * @jest-environment jsdom
 */

import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { FixtureEditor } from '../fixtures/FixtureEditor';
import { useFixtureStore } from '../../stores/useFixtureStore';

// Mock the fixture store
vi.mock('../../stores/useFixtureStore');
const mockUseFixtureStore = vi.mocked(useFixtureStore);

// Mock Monaco Editor
vi.mock('@monaco-editor/react', () => ({
  default: ({ value, onChange, language }: { value: string; onChange?: (val: string) => void; language: string }) => (
    <textarea
      data-testid="monaco-editor"
      value={value}
      onChange={(e) => onChange?.(e.target.value)}
      data-language={language}
      placeholder={`Monaco Editor (${language})`}
    />
  ),
}));

const mockFixture = {
  id: 'test-fixture-1',
  name: 'Test Fixture',
  description: 'A test fixture for testing',
  method: 'GET',
  path: '/api/test',
  statusCode: 200,
  headers: {
    'Content-Type': 'application/json',
  },
  body: JSON.stringify({ message: 'Hello, World!' }, null, 2),
  responseDelay: 0,
  isActive: true,
  createdAt: new Date().toISOString(),
  updatedAt: new Date().toISOString(),
};

describe('FixtureEditor', () => {
  const mockUpdateFixture = vi.fn();
  const mockCreateFixture = vi.fn();
  const mockDeleteFixture = vi.fn();
  const mockValidateFixture = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    
    mockUseFixtureStore.mockReturnValue({
      fixtures: [mockFixture],
      selectedFixture: mockFixture,
      isLoading: false,
      error: null,
      updateFixture: mockUpdateFixture,
      createFixture: mockCreateFixture,
      deleteFixture: mockDeleteFixture,
      validateFixture: mockValidateFixture,
      setSelectedFixture: vi.fn(),
      fetchFixtures: vi.fn(),
    });

    mockValidateFixture.mockResolvedValue({ isValid: true, errors: [] });
  });

  it('renders fixture editor with existing fixture data', () => {
    render(<FixtureEditor />);

    expect(screen.getByDisplayValue('Test Fixture')).toBeInTheDocument();
    expect(screen.getByDisplayValue('/api/test')).toBeInTheDocument();
    expect(screen.getByDisplayValue('200')).toBeInTheDocument();
    expect(screen.getByTestId('monaco-editor')).toHaveValue(
      JSON.stringify({ message: 'Hello, World!' }, null, 2)
    );
  });

  it('allows editing fixture name', async () => {
    const user = userEvent.setup();
    render(<FixtureEditor />);

    const nameInput = screen.getByDisplayValue('Test Fixture');
    await user.clear(nameInput);
    await user.type(nameInput, 'Updated Fixture Name');

    expect(nameInput).toHaveValue('Updated Fixture Name');
  });

  it('allows editing fixture path', async () => {
    const user = userEvent.setup();
    render(<FixtureEditor />);

    const pathInput = screen.getByDisplayValue('/api/test');
    await user.clear(pathInput);
    await user.type(pathInput, '/api/updated');

    expect(pathInput).toHaveValue('/api/updated');
  });

  it('allows selecting HTTP method', async () => {
    const user = userEvent.setup();
    render(<FixtureEditor />);

    const methodSelect = screen.getByRole('combobox', { name: /method/i });
    await user.click(methodSelect);
    await user.click(screen.getByText('POST'));

    expect(methodSelect).toHaveTextContent('POST');
  });

  it('allows editing response status code', async () => {
    const user = userEvent.setup();
    render(<FixtureEditor />);

    const statusInput = screen.getByDisplayValue('200');
    await user.clear(statusInput);
    await user.type(statusInput, '404');

    expect(statusInput).toHaveValue('404');
  });

  it('allows editing response body in Monaco editor', async () => {
    const user = userEvent.setup();
    render(<FixtureEditor />);

    const editor = screen.getByTestId('monaco-editor');
    await user.clear(editor);
    await user.type(editor, '{"error": "Not found"}');

    expect(editor).toHaveValue('{"error": "Not found"}');
  });

  it('validates JSON in response body', async () => {
    const user = userEvent.setup();
    render(<FixtureEditor />);

    const editor = screen.getByTestId('monaco-editor');
    await user.clear(editor);
    await user.type(editor, 'invalid json');

    // Should show validation error
    await waitFor(() => {
      expect(screen.getByText(/invalid json/i)).toBeInTheDocument();
    });
  });

  it('allows adding custom headers', async () => {
    const user = userEvent.setup();
    render(<FixtureEditor />);

    const addHeaderButton = screen.getByRole('button', { name: /add header/i });
    await user.click(addHeaderButton);

    const headerKeyInput = screen.getByPlaceholderText(/header name/i);
    const headerValueInput = screen.getByPlaceholderText(/header value/i);

    await user.type(headerKeyInput, 'X-Custom-Header');
    await user.type(headerValueInput, 'custom-value');

    expect(headerKeyInput).toHaveValue('X-Custom-Header');
    expect(headerValueInput).toHaveValue('custom-value');
  });

  it('allows removing headers', async () => {
    const user = userEvent.setup();
    render(<FixtureEditor />);

    // Should have the existing Content-Type header
    expect(screen.getByDisplayValue('Content-Type')).toBeInTheDocument();

    const removeHeaderButton = screen.getByRole('button', { name: /remove.*content-type/i });
    await user.click(removeHeaderButton);

    expect(screen.queryByDisplayValue('Content-Type')).not.toBeInTheDocument();
  });

  it('allows setting response delay', async () => {
    const user = userEvent.setup();
    render(<FixtureEditor />);

    const delayInput = screen.getByLabelText(/response delay/i);
    await user.clear(delayInput);
    await user.type(delayInput, '500');

    expect(delayInput).toHaveValue('500');
  });

  it('saves fixture when save button is clicked', async () => {
    const user = userEvent.setup();
    render(<FixtureEditor />);

    const nameInput = screen.getByDisplayValue('Test Fixture');
    await user.clear(nameInput);
    await user.type(nameInput, 'Updated Fixture');

    const saveButton = screen.getByRole('button', { name: /save/i });
    await user.click(saveButton);

    await waitFor(() => {
      expect(mockUpdateFixture).toHaveBeenCalledWith('test-fixture-1', {
        ...mockFixture,
        name: 'Updated Fixture',
      });
    });
  });

  it('creates new fixture when in create mode', async () => {
    mockUseFixtureStore.mockReturnValue({
      fixtures: [],
      selectedFixture: null,
      isLoading: false,
      error: null,
      updateFixture: mockUpdateFixture,
      createFixture: mockCreateFixture,
      deleteFixture: mockDeleteFixture,
      validateFixture: mockValidateFixture,
      setSelectedFixture: vi.fn(),
      fetchFixtures: vi.fn(),
    });

    const user = userEvent.setup();
    render(<FixtureEditor />);

    const nameInput = screen.getByLabelText(/name/i);
    await user.type(nameInput, 'New Fixture');

    const pathInput = screen.getByLabelText(/path/i);
    await user.type(pathInput, '/api/new');

    const saveButton = screen.getByRole('button', { name: /create/i });
    await user.click(saveButton);

    await waitFor(() => {
      expect(mockCreateFixture).toHaveBeenCalledWith({
        name: 'New Fixture',
        path: '/api/new',
        method: 'GET',
        statusCode: 200,
        headers: {},
        body: '',
        responseDelay: 0,
        isActive: true,
        description: '',
      });
    });
  });

  it('shows validation errors', async () => {
    mockValidateFixture.mockResolvedValue({
      isValid: false,
      errors: [
        { field: 'path', message: 'Path is required' },
        { field: 'statusCode', message: 'Status code must be between 100 and 599' },
      ],
    });

    const user = userEvent.setup();
    render(<FixtureEditor />);

    const saveButton = screen.getByRole('button', { name: /save/i });
    await user.click(saveButton);

    await waitFor(() => {
      expect(screen.getByText('Path is required')).toBeInTheDocument();
      expect(screen.getByText('Status code must be between 100 and 599')).toBeInTheDocument();
    });

    expect(mockUpdateFixture).not.toHaveBeenCalled();
  });

  it('handles preview mode', async () => {
    const user = userEvent.setup();
    render(<FixtureEditor />);

    const previewButton = screen.getByRole('button', { name: /preview/i });
    await user.click(previewButton);

    expect(screen.getByTestId('fixture-preview')).toBeInTheDocument();
    expect(screen.getByText('GET /api/test')).toBeInTheDocument();
    expect(screen.getByText('Status: 200')).toBeInTheDocument();
  });

  it('allows switching between edit and preview modes', async () => {
    const user = userEvent.setup();
    render(<FixtureEditor />);

    // Switch to preview mode
    const previewButton = screen.getByRole('button', { name: /preview/i });
    await user.click(previewButton);

    expect(screen.getByTestId('fixture-preview')).toBeInTheDocument();

    // Switch back to edit mode
    const editButton = screen.getByRole('button', { name: /edit/i });
    await user.click(editButton);

    expect(screen.queryByTestId('fixture-preview')).not.toBeInTheDocument();
    expect(screen.getByDisplayValue('Test Fixture')).toBeInTheDocument();
  });

  it('handles delete fixture', async () => {
    window.confirm = vi.fn(() => true);
    const user = userEvent.setup();
    render(<FixtureEditor />);

    const deleteButton = screen.getByRole('button', { name: /delete/i });
    await user.click(deleteButton);

    expect(window.confirm).toHaveBeenCalledWith(
      'Are you sure you want to delete this fixture? This action cannot be undone.'
    );

    await waitFor(() => {
      expect(mockDeleteFixture).toHaveBeenCalledWith('test-fixture-1');
    });
  });

  it('cancels delete when user clicks cancel', async () => {
    window.confirm = vi.fn(() => false);
    const user = userEvent.setup();
    render(<FixtureEditor />);

    const deleteButton = screen.getByRole('button', { name: /delete/i });
    await user.click(deleteButton);

    expect(mockDeleteFixture).not.toHaveBeenCalled();
  });

  it('shows loading state while saving', async () => {
    mockUseFixtureStore.mockReturnValue({
      ...mockUseFixtureStore(),
      isLoading: true,
    });

    render(<FixtureEditor />);

    const saveButton = screen.getByRole('button', { name: /saving/i });
    expect(saveButton).toBeDisabled();
    expect(screen.getByTestId('loading-spinner')).toBeInTheDocument();
  });

  it('handles keyboard shortcuts', async () => {
    const user = userEvent.setup();
    render(<FixtureEditor />);

    // Ctrl+S should save
    await user.keyboard('{Control>}s{/Control}');

    await waitFor(() => {
      expect(mockUpdateFixture).toHaveBeenCalled();
    });
  });

  it('warns about unsaved changes', async () => {
    const user = userEvent.setup();
    render(<FixtureEditor />);

    const nameInput = screen.getByDisplayValue('Test Fixture');
    await user.clear(nameInput);
    await user.type(nameInput, 'Modified Fixture');

    // Should show unsaved changes indicator
    expect(screen.getByText(/unsaved changes/i)).toBeInTheDocument();
  });
});