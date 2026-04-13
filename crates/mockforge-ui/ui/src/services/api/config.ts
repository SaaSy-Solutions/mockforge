/**
 * Config API service — server configuration, latency, faults, and proxy config.
 */
import type { ServerConfiguration, LatencyProfile, FaultConfig, ProxyConfig } from '../../types';
import { fetchJson } from './client';

class ConfigApiService {
  async getConfig(): Promise<ServerConfiguration> {
    return fetchJson('/__mockforge/config') as Promise<ServerConfiguration>;
  }

  async updateLatency(config: LatencyProfile): Promise<{ message: string }> {
    return fetchJson('/__mockforge/config/latency', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ config_type: 'latency', data: config }),
    }) as Promise<{ message: string }>;
  }

  async updateFaults(config: FaultConfig): Promise<{ message: string }> {
    return fetchJson('/__mockforge/config/faults', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ config_type: 'faults', data: config }),
    }) as Promise<{ message: string }>;
  }

  async updateProxy(config: ProxyConfig): Promise<{ message: string }> {
    return fetchJson('/__mockforge/config/proxy', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ config_type: 'proxy', data: config }),
    }) as Promise<{ message: string }>;
  }

  async updateProtocols(config: Record<string, boolean>): Promise<{ message: string }> {
    return fetchJson('/__mockforge/config/protocols', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ config_type: 'protocols', data: config }),
    }) as Promise<{ message: string }>;
  }
}

export { ConfigApiService };
