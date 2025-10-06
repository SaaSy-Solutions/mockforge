/**
 * @jest-environment jsdom
 */

import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { FixtureEditor } from '../../fixtures/FixtureEditor';
import type { FixtureInfo } from '../../../types';

const mockFixture: FixtureInfo = {
  id: 'test-fixture-1',
  name: 'test.json',
  path: '/fixtures/test.json',
  content: '{"message": "Hello, World!"}',
  size_bytes: 32,
  last_modified: '2024-01-01T00:00:00Z',
  updatedAt: '2024-01-01T00:00:00Z',
  route_path: '/api/test',
  method: 'GET',
  version: '1.0',
};

describe('FixtureEditor', () => {
  it('renders fixture editor with fixture information', () => {
    const mockSave = vi.fn();
    const mockClose = vi.fn();

    render(<FixtureEditor fixture={mockFixture} onSave={mockSave} onClose={mockClose} />);

    expect(screen.getByText('test.json')).toBeInTheDocument();
    expect(screen.getByText('/fixtures/test.json')).toBeInTheDocument();
  });

  it('displays fixture content in textarea', () => {
    const mockSave = vi.fn();
    const mockClose = vi.fn();

    render(<FixtureEditor fixture={mockFixture} onSave={mockSave} onClose={mockClose} />);

    const textarea = screen.getByDisplayValue('{"message": "Hello, World!"}');
    expect(textarea).toBeInTheDocument();
    expect(textarea).toBeInstanceOf(HTMLTextAreaElement);
  });

  it('allows editing fixture content', () => {
    const mockSave = vi.fn();
    const mockClose = vi.fn();

    render(<FixtureEditor fixture={mockFixture} onSave={mockSave} onClose={mockClose} />);

    const textarea = screen.getByDisplayValue('{"message": "Hello, World!"}') as HTMLTextAreaElement;

    fireEvent.change(textarea, { target: { value: '{"updated": true}' } });

    expect(textarea.value).toBe('{"updated": true}');
  });

  it('shows unsaved changes indicator when content is modified', () => {
    const mockSave = vi.fn();
    const mockClose = vi.fn();

    render(<FixtureEditor fixture={mockFixture} onSave={mockSave} onClose={mockClose} />);

    const textarea = screen.getByDisplayValue('{"message": "Hello, World!"}') as HTMLTextAreaElement;

    fireEvent.change(textarea, { target: { value: '{"message": "Hello, World!"} modified' } });

    expect(screen.getByText('Unsaved changes')).toBeInTheDocument();
  });

  it('calls onSave with fixture id and content when save button is clicked', () => {
    const mockSave = vi.fn();
    const mockClose = vi.fn();

    render(<FixtureEditor fixture={mockFixture} onSave={mockSave} onClose={mockClose} />);

    const textarea = screen.getByDisplayValue('{"message": "Hello, World!"}') as HTMLTextAreaElement;
    fireEvent.change(textarea, { target: { value: '{"message": "Hello, World!"} updated' } });

    const saveButton = screen.getByRole('button', { name: /save/i });
    fireEvent.click(saveButton);

    expect(mockSave).toHaveBeenCalledWith('test-fixture-1', '{"message": "Hello, World!"} updated');
  });

  it('calls onClose when close button is clicked', () => {
    const mockSave = vi.fn();
    const mockClose = vi.fn();

    render(<FixtureEditor fixture={mockFixture} onSave={mockSave} onClose={mockClose} />);

    const closeButton = screen.getByRole('button', { name: /close/i });
    fireEvent.click(closeButton);

    expect(mockClose).toHaveBeenCalled();
  });

  it('disables save button when there are no changes', () => {
    const mockSave = vi.fn();
    const mockClose = vi.fn();

    render(<FixtureEditor fixture={mockFixture} onSave={mockSave} onClose=  {mockClose} />);

    const saveButton = screen.getByRole('button', { name: /save/i });
    expect(saveButton).toBeDisabled();
  });

  it('saves on Ctrl+S keyboard shortcut', () => {
    const mockSave = vi.fn();
    const mockClose = vi.fn();

    render(<FixtureEditor fixture={mockFixture} onSave={mockSave} onClose={mockClose} />);

    const textarea = screen.getByDisplayValue('{"message": "Hello, World!"}') as HTMLTextAreaElement;

    fireEvent.change(textarea, { target: { value: '{"message": "Hello, World!"} modified' } });
    fireEvent.keyDown(textarea, { key: 's', ctrlKey: true });

    expect(mockSave).toHaveBeenCalled();
  });

  it('renders in read-only mode when readOnly prop is true', () => {
    const mockSave = vi.fn();
    const mockClose = vi.fn();

    render(<FixtureEditor fixture={mockFixture} onSave={mockSave} onClose={mockClose} readOnly={true} />);

    // In read-only mode, content is shown in a pre tag
    expect(screen.getByText('{"message": "Hello, World!"}')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /save/i })).not.toBeInTheDocument();
  });

  it('displays file size information', () => {
    const mockSave = vi.fn();
    const mockClose = vi.fn();

    render(<FixtureEditor fixture={mockFixture} onSave={mockSave} onClose={mockClose} />);

    expect(screen.getByText(/32 B/i)).toBeInTheDocument();
  });

  it('displays route information when available', () => {
    const mockSave = vi.fn();
    const mockClose = vi.fn();

    render(<FixtureEditor fixture={mockFixture} onSave={mockSave} onClose={mockClose} />);

    expect(screen.getByText(/GET \/api\/test/i)).toBeInTheDocument();
  });

  it('displays character and line count in footer', () => {
    const mockSave = vi.fn();
    const mockClose = vi.fn();

    render(<FixtureEditor fixture={mockFixture} onSave={mockSave} onClose={mockClose} />);

    expect(screen.getByText(/Characters: 28/i)).toBeInTheDocument();
    expect(screen.getByText(/Lines: 1/i)).toBeInTheDocument();
  });

  it('handles fixture with object content', () => {
    const mockSave = vi.fn();
    const mockClose = vi.fn();
    const fixtureWithObject: FixtureInfo = {
      ...mockFixture,
      content: { message: 'Object content' },
    };

    render(<FixtureEditor fixture={fixtureWithObject} onSave={mockSave} onClose={mockClose} />);

    // When content is an object, it gets stringified
    const textarea = screen.getByRole('textbox');
    expect(textarea).toBeInTheDocument();
    expect((textarea as HTMLTextAreaElement).value).toContain('Object content');
  });

  it('displays version information', () => {
    const mockSave = vi.fn();
    const mockClose = vi.fn();

    render(<FixtureEditor fixture={mockFixture} onSave={mockSave} onClose={mockClose} />);

    expect(screen.getByText(/Version: 1\.0/i)).toBeInTheDocument();
  });
});
