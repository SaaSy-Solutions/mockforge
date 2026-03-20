/**
 * Validation API service — validation settings.
 */
import type { ValidationSettings } from '../../types';
import { fetchJson } from './client';

class ValidationApiService {
  async getValidation(): Promise<ValidationSettings> {
    return fetchJson('/__mockforge/validation') as Promise<ValidationSettings>;
  }

  async updateValidation(config: ValidationSettings): Promise<{ message: string }> {
    return fetchJson('/__mockforge/validation', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(config),
    }) as Promise<{ message: string }>;
  }
}

export { ValidationApiService };
