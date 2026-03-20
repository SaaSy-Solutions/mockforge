/**
 * Chaos API service — chaos engineering configuration, network profiles, error patterns.
 */
import { fetchJson, authenticatedFetch } from './client';

export interface ChaosConfig {
  latency?: Record<string, unknown>;
  fault_injection?: Record<string, unknown>;
  traffic_shaping?: Record<string, unknown>;
  [key: string]: unknown;
}

export interface ChaosStatus {
  enabled: boolean;
  [key: string]: unknown;
}

export interface NetworkProfile {
  name: string;
  description: string;
  chaos_config: Record<string, unknown>;
  tags: string[];
  builtin: boolean;
}

class ChaosApiService {
  /**
   * Get current chaos configuration
   */
  async getChaosConfig(): Promise<ChaosConfig> {
    return fetchJson('/api/chaos/config') as Promise<ChaosConfig>;
  }

  /**
   * Get current chaos status
   */
  async getChaosStatus(): Promise<ChaosStatus> {
    return fetchJson('/api/chaos/status') as Promise<ChaosStatus>;
  }

  /**
   * Update latency configuration
   */
  async updateChaosLatency(config: Record<string, unknown>): Promise<{ message: string }> {
    return fetchJson('/api/chaos/config/latency', {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(config),
    }) as Promise<{ message: string }>;
  }

  /**
   * Update fault injection configuration
   */
  async updateChaosFaults(config: Record<string, unknown>): Promise<{ message: string }> {
    return fetchJson('/api/chaos/config/faults', {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(config),
    }) as Promise<{ message: string }>;
  }

  /**
   * Update traffic shaping configuration
   */
  async updateChaosTraffic(config: Record<string, unknown>): Promise<{ message: string }> {
    return fetchJson('/api/chaos/config/traffic', {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(config),
    }) as Promise<{ message: string }>;
  }

  /**
   * Enable chaos engineering
   */
  async enableChaos(): Promise<{ message: string }> {
    return fetchJson('/api/chaos/enable', {
      method: 'POST',
    }) as Promise<{ message: string }>;
  }

  /**
   * Disable chaos engineering
   */
  async disableChaos(): Promise<{ message: string }> {
    return fetchJson('/api/chaos/disable', {
      method: 'POST',
    }) as Promise<{ message: string }>;
  }

  /**
   * Reset chaos configuration to defaults
   */
  async resetChaos(): Promise<{ message: string }> {
    return fetchJson('/api/chaos/reset', {
      method: 'POST',
    }) as Promise<{ message: string }>;
  }

  /**
   * Get latency metrics (time-series data)
   */
  async getLatencyMetrics(): Promise<{ samples: Array<{ timestamp: number; latency_ms: number }> }> {
    return fetchJson('/api/chaos/metrics/latency') as Promise<{ samples: Array<{ timestamp: number; latency_ms: number }> }>;
  }

  /**
   * Get latency statistics
   */
  async getLatencyStats(): Promise<{
    count: number;
    min_ms: number;
    max_ms: number;
    avg_ms: number;
    p50_ms: number;
    p95_ms: number;
    p99_ms: number;
  }> {
    return fetchJson('/api/chaos/metrics/latency/stats') as Promise<{
      count: number;
      min_ms: number;
      max_ms: number;
      avg_ms: number;
      p50_ms: number;
      p95_ms: number;
      p99_ms: number;
    }>;
  }

  /**
   * List all network profiles (built-in + custom)
   */
  async getNetworkProfiles(): Promise<NetworkProfile[]> {
    return fetchJson('/api/chaos/profiles') as Promise<NetworkProfile[]>;
  }

  /**
   * Get a specific network profile by name
   */
  async getNetworkProfile(name: string): Promise<NetworkProfile> {
    return fetchJson(`/api/chaos/profiles/${encodeURIComponent(name)}`) as Promise<NetworkProfile>;
  }

  /**
   * Apply a network profile
   */
  async applyNetworkProfile(name: string): Promise<{ message: string }> {
    return fetchJson(`/api/chaos/profiles/${encodeURIComponent(name)}/apply`, {
      method: 'POST',
    }) as Promise<{ message: string }>;
  }

  /**
   * Create a custom network profile
   */
  async createNetworkProfile(profile: {
    name: string;
    description: string;
    chaos_config: Record<string, unknown>;
    tags?: string[];
  }): Promise<{ message: string }> {
    return fetchJson('/api/chaos/profiles', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(profile),
    }) as Promise<{ message: string }>;
  }

  /**
   * Delete a custom network profile
   */
  async deleteNetworkProfile(name: string): Promise<{ message: string }> {
    return fetchJson(`/api/chaos/profiles/${encodeURIComponent(name)}`, {
      method: 'DELETE',
    }) as Promise<{ message: string }>;
  }

  /**
   * Export a network profile (JSON or YAML)
   */
  async exportNetworkProfile(name: string, format: 'json' | 'yaml' = 'json'): Promise<string | Record<string, unknown>> {
    const response = await authenticatedFetch(`/api/chaos/profiles/${encodeURIComponent(name)}/export?format=${format}`);
    if (!response.ok) {
      if (response.status === 401) {
        throw new Error('Authentication required');
      }
      if (response.status === 403) {
        throw new Error('Access denied');
      }
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    if (format === 'yaml') {
      return response.text();
    }
    return response.json();
  }

  /**
   * Import a network profile from JSON or YAML
   */
  async importNetworkProfile(content: string, format: 'json' | 'yaml'): Promise<{ message: string }> {
    return fetchJson('/api/chaos/profiles/import', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ content, format }),
    }) as Promise<{ message: string }>;
  }

  /**
   * Update error pattern configuration
   */
  async updateErrorPattern(pattern: {
    type: 'burst' | 'random' | 'sequential';
    count?: number;
    interval_ms?: number;
    probability?: number;
    sequence?: number[];
  }): Promise<{ message: string }> {
    // Get current fault config, update pattern, then save
    const currentConfig = await this.getChaosConfig();
    const faultConfig = (currentConfig.fault_injection || {}) as Record<string, unknown>;
    faultConfig.error_pattern = pattern;

    return this.updateChaosFaults(faultConfig);
  }
}

export { ChaosApiService };
