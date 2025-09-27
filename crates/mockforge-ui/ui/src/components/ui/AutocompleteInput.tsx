import React, { useState, useRef, useEffect, useCallback } from 'react';
import { useAutocomplete } from '../../hooks/useApi';
import type { AutocompleteSuggestion, AutocompleteRequest } from '../../types';

interface AutocompleteInputProps {
  value: string;
  onChange: (value: string) => void;
  onSelect?: (suggestion: AutocompleteSuggestion) => void;
  placeholder?: string;
  className?: string;
  workspaceId: string;
  context?: string;
  disabled?: boolean;
}

export function AutocompleteInput({
  value,
  onChange,
  onSelect,
  placeholder,
  className,
  workspaceId,
  context,
  disabled = false,
}: AutocompleteInputProps) {
  const [cursorPosition, setCursorPosition] = useState(0);
  const [showSuggestions, setShowSuggestions] = useState(false);
  const [selectedIndex, setSelectedIndex] = useState(-1);
  const inputRef = useRef<HTMLInputElement>(null);
  const suggestionsRef = useRef<HTMLDivElement>(null);

  const autocomplete = useAutocomplete(workspaceId);

  const handleInputChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const newValue = e.target.value;
    const newCursorPosition = e.target.selectionStart || 0;

    onChange(newValue);
    setCursorPosition(newCursorPosition);

    // Check if we should show autocomplete (look for {{ pattern)
    const textBeforeCursor = newValue.slice(0, newCursorPosition);
    const hasOpenBraces = textBeforeCursor.includes('{{');

    if (hasOpenBraces) {
      // Get the current token being typed
      const lastOpenBrace = textBeforeCursor.lastIndexOf('{{');
      const textAfterOpenBrace = textBeforeCursor.slice(lastOpenBrace + 2);

      // Only show if there's a potential variable being typed
      if (textAfterOpenBrace.length > 0 || textBeforeCursor.endsWith('{{')) {
        autocomplete.mutate({
          input: newValue,
          cursor_position: newCursorPosition,
          context,
        });
        setShowSuggestions(true);
        setSelectedIndex(-1);
      } else {
        setShowSuggestions(false);
      }
    } else {
      setShowSuggestions(false);
    }
  }, [onChange, context, autocomplete]);

  const handleKeyDown = useCallback((e: React.KeyboardEvent<HTMLInputElement>) => {
    if (!showSuggestions || !autocomplete.data?.suggestions.length) {
      // Handle Ctrl+Space to manually trigger autocomplete
      if (e.ctrlKey && e.key === ' ') {
        e.preventDefault();
        const currentValue = inputRef.current?.value || '';
        const currentCursorPosition = inputRef.current?.selectionStart || 0;

        autocomplete.mutate({
          input: currentValue,
          cursor_position: currentCursorPosition,
          context,
        });
        setShowSuggestions(true);
        setSelectedIndex(-1);
      }
      return;
    }

    const suggestions = autocomplete.data.suggestions;

    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault();
        setSelectedIndex(prev => (prev + 1) % suggestions.length);
        break;
      case 'ArrowUp':
        e.preventDefault();
        setSelectedIndex(prev => prev <= 0 ? suggestions.length - 1 : prev - 1);
        break;
      case 'Enter':
      case 'Tab':
        if (selectedIndex >= 0) {
          e.preventDefault();
          handleSuggestionSelect(suggestions[selectedIndex]);
        }
        break;
      case 'Escape':
        setShowSuggestions(false);
        setSelectedIndex(-1);
        break;
    }
  }, [showSuggestions, selectedIndex, autocomplete.data, context]);

  const handleSuggestionSelect = useCallback((suggestion: AutocompleteSuggestion) => {
    if (!autocomplete.data) return;

    const { start_position, end_position } = autocomplete.data;
    const beforeToken = value.slice(0, start_position);
    const afterToken = value.slice(end_position);

    const newValue = `${beforeToken}{{${suggestion.text}}}}${afterToken}`;
    const newCursorPosition = beforeToken.length + suggestion.text.length + 4; // 4 = {{}}

    onChange(newValue);
    onSelect?.(suggestion);

    // Update cursor position after state update
    setTimeout(() => {
      inputRef.current?.setSelectionRange(newCursorPosition, newCursorPosition);
      inputRef.current?.focus();
    }, 0);

    setShowSuggestions(false);
    setSelectedIndex(-1);
  }, [value, onChange, onSelect, autocomplete.data]);

  // Handle clicks outside to close suggestions
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (
        suggestionsRef.current &&
        !suggestionsRef.current.contains(event.target as Node) &&
        inputRef.current &&
        !inputRef.current.contains(event.target as Node)
      ) {
        setShowSuggestions(false);
        setSelectedIndex(-1);
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  // Update cursor position when input value changes externally
  useEffect(() => {
    if (inputRef.current) {
      setCursorPosition(inputRef.current.selectionStart || 0);
    }
  }, [value]);

  const suggestions = autocomplete.data?.suggestions || [];

  return (
    <div className="relative">
      <input
        ref={inputRef}
        type="text"
        value={value}
        onChange={handleInputChange}
        onKeyDown={handleKeyDown}
        placeholder={placeholder}
        className={className}
        disabled={disabled}
        autoComplete="off"
        spellCheck={false}
      />

      {showSuggestions && suggestions.length > 0 && (
        <div
          ref={suggestionsRef}
          className="absolute z-50 w-full mt-1 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-md shadow-lg max-h-60 overflow-y-auto"
        >
          {suggestions.map((suggestion, index) => (
            <div
              key={`${suggestion.kind}-${suggestion.text}`}
              className={`px-3 py-2 cursor-pointer border-b border-gray-100 dark:border-gray-700 last:border-b-0 ${
                index === selectedIndex
                  ? 'bg-blue-50 dark:bg-blue-900/20 text-blue-700 dark:text-blue-300'
                  : 'hover:bg-gray-50 dark:hover:bg-gray-700'
              }`}
              onClick={() => handleSuggestionSelect(suggestion)}
            >
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <span className="font-medium text-gray-900 dark:text-gray-100">
                    {suggestion.text}
                  </span>
                  <span className={`text-xs px-2 py-1 rounded ${
                    suggestion.kind === 'variable'
                      ? 'bg-green-100 dark:bg-green-900/20 text-green-700 dark:text-green-300'
                      : 'bg-blue-100 dark:bg-blue-900/20 text-blue-700 dark:text-blue-300'
                  }`}>
                    {suggestion.kind}
                  </span>
                </div>
              </div>
              {suggestion.description && (
                <div className="text-sm text-gray-600 dark:text-gray-400 mt-1">
                  {suggestion.description}
                </div>
              )}
            </div>
          ))}
        </div>
      )}

      {/* Loading indicator */}
      {autocomplete.isPending && showSuggestions && (
        <div className="absolute right-3 top-1/2 transform -translate-y-1/2">
          <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-blue-600"></div>
        </div>
      )}

      {/* Hint for Ctrl+Space */}
      <div className="absolute right-3 top-1/2 transform -translate-y-1/2 text-xs text-gray-400 dark:text-gray-500 opacity-0 group-hover:opacity-100 transition-opacity">
        Ctrl+Space
      </div>
    </div>
  );
}
