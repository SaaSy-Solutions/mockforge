import { logger } from '@/utils/logger';
import { useContext } from 'react';
import { AutocompleteContext } from './AutocompleteContext';
import type { AutocompleteContextType } from './AutocompleteContext';

export const useAutocomplete = (): AutocompleteContextType => {
  const context = useContext(AutocompleteContext);
  if (context === undefined) {
    throw new Error('useAutocomplete must be used within an AutocompleteProvider');
  }
  return context;
};
