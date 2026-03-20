/**
 * Server API service — server info, restart operations.
 */
import type { ServerInfo, RestartStatus } from '../../types';
import { fetchJson } from './client';

class ServerApiService {
  async getServerInfo(): Promise<ServerInfo> {
    return fetchJson('/__mockforge/server-info') as Promise<ServerInfo>;
  }

  async restartServer(reason?: string): Promise<RestartStatus> {
    return fetchJson('/__mockforge/servers/restart', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ reason: reason || 'Manual restart' }),
    }) as Promise<RestartStatus>;
  }

  async getRestartStatus(): Promise<RestartStatus> {
    return fetchJson('/__mockforge/servers/restart/status') as Promise<RestartStatus>;
  }
}

export { ServerApiService };
