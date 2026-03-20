/**
 * Reality Slider API service — reality level management and preset operations.
 */
import { fetchJsonWithErrorBody } from './client';

class RealityApiService {
  /**
   * Get current reality level and configuration
   */
  async getRealityLevel(): Promise<{
    level: number;
    level_name: string;
    description: string;
    chaos: {
      enabled: boolean;
      error_rate: number;
      delay_rate: number;
    };
    latency: {
      base_ms: number;
      jitter_ms: number;
    };
    mockai: {
      enabled: boolean;
    };
  }> {
    return fetchJsonWithErrorBody('/__mockforge/reality/level') as Promise<{
      level: number;
      level_name: string;
      description: string;
      chaos: {
        enabled: boolean;
        error_rate: number;
        delay_rate: number;
      };
      latency: {
        base_ms: number;
        jitter_ms: number;
      };
      mockai: {
        enabled: boolean;
      };
    }>;
  }

  /**
   * Set reality level (1-5)
   */
  async setRealityLevel(level: number): Promise<{
    level: number;
    level_name: string;
    description: string;
    chaos: {
      enabled: boolean;
      error_rate: number;
      delay_rate: number;
    };
    latency: {
      base_ms: number;
      jitter_ms: number;
    };
    mockai: {
      enabled: boolean;
    };
  }> {
    return fetchJsonWithErrorBody('/__mockforge/reality/level', {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ level }),
    }) as Promise<{
      level: number;
      level_name: string;
      description: string;
      chaos: {
        enabled: boolean;
        error_rate: number;
        delay_rate: number;
      };
      latency: {
        base_ms: number;
        jitter_ms: number;
      };
      mockai: {
        enabled: boolean;
      };
    }>;
  }

  /**
   * List all available reality presets
   */
  async listPresets(): Promise<Array<{
    id: string;
    path: string;
    name: string;
  }>> {
    return fetchJsonWithErrorBody('/__mockforge/reality/presets') as Promise<Array<{
      id: string;
      path: string;
      name: string;
    }>>;
  }

  /**
   * Import a reality preset
   */
  async importPreset(path: string): Promise<{
    name: string;
    description?: string;
    level: number;
    level_name: string;
  }> {
    return fetchJsonWithErrorBody('/__mockforge/reality/presets/import', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ path }),
    }) as Promise<{
      name: string;
      description?: string;
      level: number;
      level_name: string;
    }>;
  }

  /**
   * Export current reality configuration as a preset
   */
  async exportPreset(name: string, description?: string): Promise<{
    name: string;
    description?: string;
    path: string;
    level: number;
  }> {
    return fetchJsonWithErrorBody('/__mockforge/reality/presets/export', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ name, description }),
    }) as Promise<{
      name: string;
      description?: string;
      path: string;
      level: number;
    }>;
  }
}

export { RealityApiService };
