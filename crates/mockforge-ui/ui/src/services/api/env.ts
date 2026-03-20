/**
 * Environment variables API service.
 */
import { fetchJson } from './client';

class EnvApiService {
  async getEnvVars(): Promise<Record<string, string>> {
    return fetchJson('/__mockforge/env') as Promise<Record<string, string>>;
  }

  async updateEnvVar(key: string, value: string): Promise<{ message: string }> {
    return fetchJson('/__mockforge/env', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ key, value }),
    }) as Promise<{ message: string }>;
  }
}

export { EnvApiService };
