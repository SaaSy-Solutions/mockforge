import React, { createContext, useContext } from 'react';
import type { ReactNode } from 'react';

export interface AutocompleteSuggestion {
  label: string;
  description?: string;
  insertText: string;
  kind: 'function' | 'variable' | 'constant' | 'keyword' | 'property';
  category?: string;
}

export interface ChainContext {
  id: string;
  name: string;
  links: {
    id: string;
    request: {
      id: string;
      method: string;
      url: string;
    };
    storeAs?: string;
  }[];
}

interface AutocompleteContextType {
  suggestions: AutocompleteSuggestion[];
  chainContext: ChainContext | null;
  getSuggestionsForPosition: (text: string, position: number) => AutocompleteSuggestion[];
  updateChainContext: (chain: ChainContext | null) => void;
}

const AutocompleteContext = createContext<AutocompleteContextType | undefined>(undefined);

interface AutocompleteProviderProps {
  children: ReactNode;
}

export const AutocompleteProvider: React.FC<AutocompleteProviderProps> = ({ children }) => {
  const [chainContext, setChainContext] = React.useState<ChainContext | null>(null);

  // Generate base suggestions (functions, constants)
  const baseSuggestions: AutocompleteSuggestion[] = [
    {
      label: 'response(requestId, jsonPath)',
      description: 'Reference a response from a request in the current chain',
      insertText: "response('${1:requestId}', '${2:jsonPath}')",
      kind: 'function',
      category: 'chain'
    },
    {
      label: 'uuid()',
      description: 'Generate a random UUID',
      insertText: 'uuid()',
      kind: 'function',
      category: 'uuid'
    },
    {
      label: 'now()',
      description: 'Current timestamp in RFC3339 format',
      insertText: 'now()',
      kind: 'function',
      category: 'time'
    },
    {
      label: 'now+1h',
      description: 'Timestamp offset (supports +Nd|Nh|Nm|Ns)',
      insertText: 'now+1h',
      kind: 'function',
      category: 'time'
    },
    {
      label: 'rand.int',
      description: 'Random integer (0 to 1,000,000)',
      insertText: 'rand.int',
      kind: 'variable',
      category: 'random'
    },
    {
      label: 'rand.float',
      description: 'Random float (0 to 1.0)',
      insertText: 'rand.float',
      kind: 'variable',
      category: 'random'
    },
    {
      label: 'faker.name()',
      description: 'Generate a fake name',
      insertText: 'faker.name()',
      kind: 'function',
      category: 'faker'
    },
    {
      label: 'faker.email()',
      description: 'Generate a fake email',
      insertText: 'faker.email()',
      kind: 'function',
      category: 'faker'
    },
    {
      label: 'faker.uuid()',
      description: 'Generate a fake UUID',
      insertText: 'faker.uuid()',
      kind: 'function',
      category: 'faker'
    }
  ];

  // Generate dynamic suggestions based on chain context
  const getChainSuggestions = (): AutocompleteSuggestion[] => {
    if (!chainContext) return [];

    const suggestions: AutocompleteSuggestion[] = [];

    // Add request IDs as variables
    chainContext.links.forEach(link => {
      suggestions.push({
        label: `${link.id}`,
        description: `Response from ${link.request.method} ${link.request.url}`,
        insertText: link.id,
        kind: 'variable',
        category: 'request'
      });

      // Add storeAs variables if they exist
      if (link.storeAs) {
        suggestions.push({
          label: `${link.storeAs}`,
          description: `Stored response from ${link.request.method} ${link.request.url}`,
          insertText: link.storeAs,
          kind: 'variable',
          category: 'response'
        });
      }
    });

    return suggestions;
  };

  const getSuggestionsForPosition = (text: string, position: number): AutocompleteSuggestion[] => {
    const textBeforePosition = text.slice(0, position);

    // Parse context to determine what suggestions to show
    const lastWordMatch = textBeforePosition.match(/([^\s]+)$/);
    const lastWord = lastWordMatch ? lastWordMatch[1] : '';

    // Check for response function context
    const responseMatch = textBeforePosition.match(/response\s*\(\s*([^)]*)$/);
    if (responseMatch) {
      const insideResponse = responseMatch[1];
      if (!insideResponse.includes(',')) {
        // Suggesting request ID within response function
        return getChainSuggestions().filter(s => s.category === 'request' || s.category === 'response');
      } else {
        // Suggesting JSON path after comma
        return [
          {
            label: 'body.field',
            description: 'Access a JSON field from response body',
            insertText: 'body.${1:field}',
            kind: 'property',
            category: 'jsonpath'
          },
          {
            label: 'body[0].field',
            description: 'Access an array element',
            insertText: 'body[0].${1:field}',
            kind: 'property',
            category: 'jsonpath'
          },
          {
            label: 'data.result.id',
            description: 'Nested object property access',
            insertText: 'data.result.${1:id}',
            kind: 'property',
            category: 'jsonpath'
          }
        ];
      }
    }

    // Filter base suggestions based on what user is typing
    let filtered = baseSuggestions;

    if (lastWord.length > 0) {
      filtered = baseSuggestions.filter(suggestion => {
        return suggestion.label.toLowerCase().includes(lastWord.toLowerCase()) ||
               (suggestion.category && suggestion.category.toLowerCase().includes(lastWord.toLowerCase()));
      });
    }

    // Add chain-specific suggestions
    const chainSuggestions = getChainSuggestions();
    filtered = [...filtered, ...chainSuggestions];

    return filtered.slice(0, 10); // Limit to 10 suggestions
  };

  const updateChainContext = (chain: ChainContext | null) => {
    setChainContext(chain);
  };

  const value: AutocompleteContextType = {
    suggestions: baseSuggestions,
    chainContext,
    getSuggestionsForPosition,
    updateChainContext,
  };

  return (
    <AutocompleteContext.Provider value={value}>
      {children}
    </AutocompleteContext.Provider>
  );
};

export const useAutocomplete = (): AutocompleteContextType => {
  const context = useContext(AutocompleteContext);
  if (context === undefined) {
    throw new Error('useAutocomplete must be used within an AutocompleteProvider');
  }
  return context;
};
