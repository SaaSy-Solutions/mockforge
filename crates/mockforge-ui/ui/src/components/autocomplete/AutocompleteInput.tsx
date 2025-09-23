import React, { useState, useRef, useCallback } from 'react';
import type { AutocompleteSuggestion } from './AutocompleteContext';
import { useAutocomplete } from './AutocompleteContext';

interface AutocompleteInputProps {
  value: string;
  onChange: (value: string) => void;
  onBlur?: () => void;
  placeholder?: string;
  className?: string;
  disabled?: boolean;
}

export const AutocompleteInput: React.FC<AutocompleteInputProps> = ({
  value,
  onChange,
  onBlur,
  placeholder = 'Type a template expression...',
  className = '',
  disabled = false,
}) => {
  const [suggestions, setSuggestions] = useState<AutocompleteSuggestion[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [cursorPosition, setCursorPosition] = useState(0);
  const [showSuggestions, setShowSuggestions] = useState(false);
  const [activePlaceholders, setActivePlaceholders] = useState<{ start: number; end: number; index: number }[]>([]);

  const inputRef = useRef<HTMLInputElement>(null);
  const { getSuggestionsForPosition } = useAutocomplete();

  const updateSuggestions = useCallback((text: string, position: number) => {
    const newSuggestions = getSuggestionsForPosition(text, position);
    setSuggestions(newSuggestions);
    setSelectedIndex(0);
    setShowSuggestions(newSuggestions.length > 0);
  }, [getSuggestionsForPosition]);

  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const newValue = e.target.value;
    const position = e.target.selectionStart || 0;

    onChange(newValue);
    setCursorPosition(position);
    setActivePlaceholders([]); // Clear active placeholders on manual edit
    updateSuggestions(newValue, position);
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Escape') {
      setShowSuggestions(false);
      setActivePlaceholders([]);
      return;
    }

    // Handle Tab navigation between placeholders
    if (e.key === 'Tab' && activePlaceholders.length > 0) {
      e.preventDefault();
      const currentPos = (e.target as HTMLInputElement).selectionStart || 0;
      const currentPlaceholderIndex = activePlaceholders.findIndex(p => currentPos >= p.start && currentPos <= p.end);

      if (currentPlaceholderIndex >= 0) {
        // Move to next placeholder
        const nextIndex = (currentPlaceholderIndex + 1) % activePlaceholders.length;
        const nextPlaceholder = activePlaceholders[nextIndex];
        setCursorPosition(nextPlaceholder.start);
        inputRef.current?.setSelectionRange(nextPlaceholder.start, nextPlaceholder.end);
      } else {
        // If not in a placeholder, go to first one
        const firstPlaceholder = activePlaceholders[0];
        setCursorPosition(firstPlaceholder.start);
        inputRef.current?.setSelectionRange(firstPlaceholder.start, firstPlaceholder.end);
      }
      return;
    }

    if (suggestions.length === 0 || !showSuggestions) return;

    if (e.key === 'ArrowDown') {
      e.preventDefault();
      setSelectedIndex(prev => (prev + 1) % suggestions.length);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setSelectedIndex(prev => prev === 0 ? suggestions.length - 1 : prev - 1);
    } else if (e.key === 'Tab' || e.key === 'Enter') {
      e.preventDefault();
      const selectedSuggestion = suggestions[selectedIndex];
      if (selectedSuggestion) {
        applySuggestion(selectedSuggestion);
      }
    } else if (e.key === 'Ctrl+ ') {
      // Ctrl+Space to trigger completion
      e.preventDefault();
      const position = (e.target as HTMLInputElement).selectionStart || 0;
      updateSuggestions(value, position);
    }
  };

  const applySuggestion = (suggestion: AutocompleteSuggestion) => {
    const beforeCursor = value.substring(0, cursorPosition);
    const afterCursor = value.substring(cursorPosition);

    // Parse template variables in insertText (e.g., ${1:variableName})
    const placeholderRegex = /\$\{(\d+):([^}]+)\}/g;
    let processedText = suggestion.insertText;
    const placeholders: { index: number; variable: string; start: number; end: number }[] = [];
    let match;

    while ((match = placeholderRegex.exec(suggestion.insertText)) !== null) {
      const [fullMatch, tabStop, variable] = match;
      const startPos = match.index;
      const endPos = startPos + fullMatch.length;

      placeholders.push({
        index: parseInt(tabStop),
        variable,
        start: startPos,
        end: endPos
      });

      // Replace placeholder with variable name for initial insertion
      processedText = processedText.replace(fullMatch, variable);
    }

    const newValue = beforeCursor + processedText + afterCursor;
    onChange(newValue);
    setShowSuggestions(false);

    // Set up active placeholders for navigation
    const activePlaceholderPositions: { start: number; end: number; index: number }[] = [];
    if (placeholders.length > 0) {
      placeholders.sort((a, b) => a.index - b.index);
      placeholders.forEach(placeholder => {
        activePlaceholderPositions.push({
          start: cursorPosition + placeholder.start,
          end: cursorPosition + placeholder.start + placeholder.variable.length,
          index: placeholder.index
        });
      });
    }
    setActivePlaceholders(activePlaceholderPositions);

    // Place cursor at first placeholder or end of inserted text
    let newCursorPosition = cursorPosition + processedText.length;
    if (activePlaceholderPositions.length > 0) {
      newCursorPosition = activePlaceholderPositions[0].start;
    }

    setCursorPosition(newCursorPosition);
    inputRef.current?.setSelectionRange(newCursorPosition, newCursorPosition);
  };

  const handleFocus = () => {
    // Focus handled by updating suggestions on click/position change
  };

  const handleBlur = () => {
    // Delay blur to allow clicking on suggestions
    setTimeout(() => {
      setShowSuggestions(false);
      setActivePlaceholders([]);
      onBlur?.();
    }, 150);
  };

  const handleClick = (e: React.MouseEvent<HTMLInputElement>) => {
    const position = (e.target as HTMLInputElement).selectionStart || 0;
    setCursorPosition(position);
    updateSuggestions(value, position);
  };

  const handleSuggestionClick = (suggestion: AutocompleteSuggestion) => {
    applySuggestion(suggestion);
  };

  return (
    <div className="relative w-full">
      <div className="relative">
        <input
          ref={inputRef}
          type="text"
          value={value}
          onChange={handleInputChange}
          onKeyDown={handleKeyDown}
          onFocus={handleFocus}
          onBlur={handleBlur}
          onClick={handleClick}
          placeholder={placeholder}
          className={`w-full px-3 py-2 text-sm font-mono border rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 ${className}`}
          disabled={disabled}
          spellCheck={false}
        />
        <div className="absolute inset-y-0 right-0 flex items-center pr-2 pointer-events-none">
          <span className="text-xs text-gray-400 bg-gray-100 px-1 rounded">
            Template
          </span>
        </div>
      </div>

      {showSuggestions && suggestions.length > 0 && (
        <div className="absolute z-10 w-full bg-white border border-gray-200 rounded-md shadow-lg mt-1 max-h-60 overflow-y-auto">
          {suggestions.map((suggestion, index) => (
            <div
              key={index}
              className={`px-3 py-2 text-sm cursor-pointer hover:bg-gray-100 ${
                index === selectedIndex ? 'bg-blue-50' : ''
              }`}
              onMouseDown={(e) => {
                e.preventDefault();
                handleSuggestionClick(suggestion);
              }}
              onMouseEnter={() => setSelectedIndex(index)}
            >
              <div className="flex items-center justify-between">
                <div className="flex-1">
                  <div className="font-mono font-medium text-gray-900">
                    {suggestion.label}
                  </div>
                  {suggestion.description && (
                    <div className="text-xs text-gray-600 mt-1">
                      {suggestion.description}
                    </div>
                  )}
                </div>
                <div className="ml-2 text-xs text-gray-400 capitalize">
                  {suggestion.category || suggestion.kind}
                </div>
              </div>
            </div>
          ))}
        </div>
      )}

      {!disabled && (
        <p className="text-xs text-gray-500 mt-1">
          Use Ctrl+Space for autocomplete suggestions
        </p>
      )}
    </div>
  );
};
