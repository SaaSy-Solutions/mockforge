/**
 * @jest-environment jsdom
 */

import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { AutocompleteProvider, AutocompleteContext } from '../../autocomplete/AutocompleteContext';
import { AutocompleteInput } from '../../autocomplete/AutocompleteInput';
import { useAutocomplete } from '../../autocomplete/useAutocomplete';
import type { ChainContext } from '../../autocomplete/AutocompleteContext';

describe('AutocompleteContext', () => {
  it('provides base suggestions by default', () => {
    let contextValue: any;

    const TestComponent = () => {
      contextValue = useAutocomplete();
      return null;
    };

    render(
      <AutocompleteProvider>
        <TestComponent />
      </AutocompleteProvider>
    );

    expect(contextValue.suggestions).toBeDefined();
    expect(contextValue.suggestions.length).toBeGreaterThan(0);
    expect(contextValue.suggestions.some((s: any) => s.label === 'uuid()')).toBe(true);
    expect(contextValue.suggestions.some((s: any) => s.label === 'now()')).toBe(true);
  });

  it('provides null chain context by default', () => {
    let contextValue: any;

    const TestComponent = () => {
      contextValue = useAutocomplete();
      return null;
    };

    render(
      <AutocompleteProvider>
        <TestComponent />
      </AutocompleteProvider>
    );

    expect(contextValue.chainContext).toBeNull();
  });

  it('allows updating chain context', async () => {
    let contextValue: any;

    const TestComponent = () => {
      contextValue = useAutocomplete();
      return <div>{contextValue.chainContext?.id || 'no-chain'}</div>;
    };

    render(
      <AutocompleteProvider>
        <TestComponent />
      </AutocompleteProvider>
    );

    const mockChainContext: ChainContext = {
      id: 'chain-1',
      name: 'Test Chain',
      links: [
        {
          id: 'req-1',
          request: {
            id: 'req-1',
            method: 'GET',
            url: '/api/users',
          },
          storeAs: 'users',
        },
      ],
    };

    contextValue.updateChainContext(mockChainContext);

    // Wait for the context to update
    await waitFor(() => {
      expect(screen.getByText('chain-1')).toBeInTheDocument();
    });

    expect(contextValue.chainContext).toEqual(mockChainContext);
  });

  it('returns filtered suggestions for position in text', () => {
    let contextValue: any;

    const TestComponent = () => {
      contextValue = useAutocomplete();
      return null;
    };

    render(
      <AutocompleteProvider>
        <TestComponent />
      </AutocompleteProvider>
    );

    const suggestions = contextValue.getSuggestionsForPosition('uuid', 4);
    expect(suggestions.some((s: any) => s.label.toLowerCase().includes('uuid'))).toBe(true);
  });

  it('returns JSON path suggestions inside response function', () => {
    let contextValue: any;

    const TestComponent = () => {
      contextValue = useAutocomplete();
      return null;
    };

    render(
      <AutocompleteProvider>
        <TestComponent />
      </AutocompleteProvider>
    );

    const suggestions = contextValue.getSuggestionsForPosition('response(req-1, ', 16);
    expect(suggestions.some((s: any) => s.category === 'jsonpath')).toBe(true);
  });

  it('returns chain suggestions inside response function first argument', async () => {
    let contextValue: any;

    const TestComponent = () => {
      contextValue = useAutocomplete();
      return <div>{contextValue.chainContext?.id || 'no-chain'}</div>;
    };

    render(
      <AutocompleteProvider>
        <TestComponent />
      </AutocompleteProvider>
    );

    const mockChainContext: ChainContext = {
      id: 'chain-1',
      name: 'Test Chain',
      links: [
        {
          id: 'req-1',
          request: {
            id: 'req-1',
            method: 'GET',
            url: '/api/users',
          },
        },
      ],
    };

    contextValue.updateChainContext(mockChainContext);

    // Wait for the context to update
    await waitFor(() => {
      expect(screen.getByText('chain-1')).toBeInTheDocument();
    });

    const suggestions = contextValue.getSuggestionsForPosition('response(', 9);
    expect(suggestions.some((s: any) => s.category === 'request')).toBe(true);
  });

  it('limits suggestions to 10 items', () => {
    let contextValue: any;

    const TestComponent = () => {
      contextValue = useAutocomplete();
      return null;
    };

    render(
      <AutocompleteProvider>
        <TestComponent />
      </AutocompleteProvider>
    );

    const suggestions = contextValue.getSuggestionsForPosition('', 0);
    expect(suggestions.length).toBeLessThanOrEqual(10);
  });
});

describe('useAutocomplete', () => {
  it('throws error when used outside AutocompleteProvider', () => {
    const TestComponent = () => {
      useAutocomplete();
      return null;
    };

    // Suppress console.error for this test
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    expect(() => {
      render(<TestComponent />);
    }).toThrow('useAutocomplete must be used within an AutocompleteProvider');

    consoleSpy.mockRestore();
  });

  it('returns context when used inside AutocompleteProvider', () => {
    let contextValue: any;

    const TestComponent = () => {
      contextValue = useAutocomplete();
      return null;
    };

    render(
      <AutocompleteProvider>
        <TestComponent />
      </AutocompleteProvider>
    );

    expect(contextValue).toBeDefined();
    expect(contextValue.suggestions).toBeDefined();
    expect(contextValue.getSuggestionsForPosition).toBeDefined();
    expect(contextValue.updateChainContext).toBeDefined();
  });
});

describe('AutocompleteInput', () => {
  const mockOnChange = vi.fn();
  const mockOnBlur = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders input field with placeholder', () => {
    render(
      <AutocompleteProvider>
        <AutocompleteInput value="" onChange={mockOnChange} />
      </AutocompleteProvider>
    );

    expect(screen.getByPlaceholderText('Type a template expression...')).toBeInTheDocument();
  });

  it('renders custom placeholder when provided', () => {
    render(
      <AutocompleteProvider>
        <AutocompleteInput
          value=""
          onChange={mockOnChange}
          placeholder="Custom placeholder"
        />
      </AutocompleteProvider>
    );

    expect(screen.getByPlaceholderText('Custom placeholder')).toBeInTheDocument();
  });

  it('displays current value in input', () => {
    render(
      <AutocompleteProvider>
        <AutocompleteInput value="uuid()" onChange={mockOnChange} />
      </AutocompleteProvider>
    );

    expect(screen.getByDisplayValue('uuid()')).toBeInTheDocument();
  });

  it('calls onChange when input value changes', () => {
    render(
      <AutocompleteProvider>
        <AutocompleteInput value="" onChange={mockOnChange} />
      </AutocompleteProvider>
    );

    const input = screen.getByPlaceholderText('Type a template expression...');
    fireEvent.change(input, { target: { value: 'uuid', selectionStart: 4 } });

    expect(mockOnChange).toHaveBeenCalledWith('uuid');
  });

  it('shows suggestions when typing', async () => {
    render(
      <AutocompleteProvider>
        <AutocompleteInput value="" onChange={mockOnChange} />
      </AutocompleteProvider>
    );

    const input = screen.getByPlaceholderText('Type a template expression...');
    fireEvent.change(input, { target: { value: 'uuid', selectionStart: 4 } });

    await waitFor(() => {
      expect(screen.getByText('uuid()')).toBeInTheDocument();
    });
  });

  it('hides suggestions when pressing Escape', async () => {
    render(
      <AutocompleteProvider>
        <AutocompleteInput value="" onChange={mockOnChange} />
      </AutocompleteProvider>
    );

    const input = screen.getByPlaceholderText('Type a template expression...');

    // Show suggestions
    fireEvent.change(input, { target: { value: 'uuid', selectionStart: 4 } });

    await waitFor(() => {
      expect(screen.getByText('uuid()')).toBeInTheDocument();
    });

    // Hide suggestions with Escape
    fireEvent.keyDown(input, { key: 'Escape' });

    await waitFor(() => {
      expect(screen.queryByText('uuid()')).not.toBeInTheDocument();
    });
  });

  it('navigates suggestions with arrow keys', async () => {
    render(
      <AutocompleteProvider>
        <AutocompleteInput value="" onChange={mockOnChange} />
      </AutocompleteProvider>
    );

    const input = screen.getByPlaceholderText('Type a template expression...');
    fireEvent.change(input, { target: { value: 'rand', selectionStart: 4 } });

    await waitFor(() => {
      expect(screen.getByText('rand.int')).toBeInTheDocument();
    });

    // Arrow down should highlight next suggestion
    fireEvent.keyDown(input, { key: 'ArrowDown' });

    // Arrow up should go back
    fireEvent.keyDown(input, { key: 'ArrowUp' });
  });

  it('applies suggestion on Enter key', async () => {
    render(
      <AutocompleteProvider>
        <AutocompleteInput value="" onChange={mockOnChange} />
      </AutocompleteProvider>
    );

    const input = screen.getByPlaceholderText('Type a template expression...');
    fireEvent.change(input, { target: { value: 'uuid', selectionStart: 4 } });

    await waitFor(() => {
      expect(screen.getByText('uuid()')).toBeInTheDocument();
    });

    fireEvent.keyDown(input, { key: 'Enter' });

    await waitFor(() => {
      expect(mockOnChange).toHaveBeenCalledWith(expect.stringContaining('uuid()'));
    });
  });

  it('applies suggestion on Tab key', async () => {
    render(
      <AutocompleteProvider>
        <AutocompleteInput value="" onChange={mockOnChange} />
      </AutocompleteProvider>
    );

    const input = screen.getByPlaceholderText('Type a template expression...');
    fireEvent.change(input, { target: { value: 'now', selectionStart: 3 } });

    await waitFor(() => {
      expect(screen.getByText('now()')).toBeInTheDocument();
    });

    fireEvent.keyDown(input, { key: 'Tab' });

    await waitFor(() => {
      expect(mockOnChange).toHaveBeenCalledWith(expect.stringContaining('now()'));
    });
  });

  it('applies suggestion on click', async () => {
    render(
      <AutocompleteProvider>
        <AutocompleteInput value="" onChange={mockOnChange} />
      </AutocompleteProvider>
    );

    const input = screen.getByPlaceholderText('Type a template expression...');
    fireEvent.change(input, { target: { value: 'uuid', selectionStart: 4 } });

    await waitFor(() => {
      expect(screen.getByText('uuid()')).toBeInTheDocument();
    });

    const suggestion = screen.getByText('uuid()');
    fireEvent.mouseDown(suggestion);

    await waitFor(() => {
      expect(mockOnChange).toHaveBeenCalledWith(expect.stringContaining('uuid()'));
    });
  });

  it('shows help text when not disabled', () => {
    render(
      <AutocompleteProvider>
        <AutocompleteInput value="" onChange={mockOnChange} />
      </AutocompleteProvider>
    );

    expect(screen.getByText('Use Ctrl+Space for autocomplete suggestions')).toBeInTheDocument();
  });

  it('hides help text when disabled', () => {
    render(
      <AutocompleteProvider>
        <AutocompleteInput value="" onChange={mockOnChange} disabled />
      </AutocompleteProvider>
    );

    expect(screen.queryByText('Use Ctrl+Space for autocomplete suggestions')).not.toBeInTheDocument();
  });

  it('disables input when disabled prop is true', () => {
    render(
      <AutocompleteProvider>
        <AutocompleteInput value="" onChange={mockOnChange} disabled />
      </AutocompleteProvider>
    );

    const input = screen.getByPlaceholderText('Type a template expression...');
    expect(input).toBeDisabled();
  });

  it('calls onBlur when input loses focus', async () => {
    render(
      <AutocompleteProvider>
        <AutocompleteInput value="" onChange={mockOnChange} onBlur={mockOnBlur} />
      </AutocompleteProvider>
    );

    const input = screen.getByPlaceholderText('Type a template expression...');
    fireEvent.blur(input);

    await waitFor(() => {
      expect(mockOnBlur).toHaveBeenCalled();
    }, { timeout: 200 });
  });

  it('updates suggestions on click', async () => {
    render(
      <AutocompleteProvider>
        <AutocompleteInput value="uuid() now()" onChange={mockOnChange} />
      </AutocompleteProvider>
    );

    const input = screen.getByPlaceholderText('Type a template expression...');

    // Mock selectionStart
    Object.defineProperty(input, 'selectionStart', {
      writable: true,
      value: 5,
    });

    fireEvent.click(input);

    // Should show suggestions based on cursor position
    await waitFor(() => {
      const suggestionElements = screen.queryAllByText(/uuid|now|faker/);
      expect(suggestionElements.length).toBeGreaterThan(0);
    });
  });

  it('displays suggestion descriptions', async () => {
    render(
      <AutocompleteProvider>
        <AutocompleteInput value="" onChange={mockOnChange} />
      </AutocompleteProvider>
    );

    const input = screen.getByPlaceholderText('Type a template expression...');
    fireEvent.change(input, { target: { value: 'uuid', selectionStart: 4 } });

    await waitFor(() => {
      expect(screen.getByText('Generate a random UUID')).toBeInTheDocument();
    });
  });

  it('highlights selected suggestion on mouse enter', async () => {
    render(
      <AutocompleteProvider>
        <AutocompleteInput value="" onChange={mockOnChange} />
      </AutocompleteProvider>
    );

    const input = screen.getByPlaceholderText('Type a template expression...');
    fireEvent.change(input, { target: { value: 'rand', selectionStart: 4 } });

    await waitFor(() => {
      expect(screen.getByText('rand.int')).toBeInTheDocument();
    });

    const suggestion = screen.getByText('rand.int');
    // The parent of the text is the font-mono div, we need its parent
    const suggestionContainer = suggestion.closest('.cursor-pointer');
    fireEvent.mouseEnter(suggestionContainer!);

    // Wait for the state update to apply the highlight class
    await waitFor(() => {
      expect(suggestionContainer).toHaveClass('bg-blue-50');
    });
  });
});
