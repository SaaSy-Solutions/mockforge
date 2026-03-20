/**
 * Plugins API service — plugin listing, status, reload, deletion.
 */
import type { PluginListResponse } from '../../types';
import { fetchJson } from './client';

class PluginsApiService {
  async getPlugins(params?: { type?: string; status?: string }): Promise<PluginListResponse> {
    const queryParams = new URLSearchParams();
    if (params?.type) queryParams.append('type', params.type);
    if (params?.status) queryParams.append('status', params.status);

    const queryString = queryParams.toString() ? `?${queryParams.toString()}` : '';
    return fetchJson(`/__mockforge/plugins${queryString}`) as Promise<PluginListResponse>;
  }

  async getPluginStatus(): Promise<unknown> {
    return fetchJson('/__mockforge/plugins/status');
  }

  async getPluginDetails(pluginId: string): Promise<unknown> {
    return fetchJson(`/__mockforge/plugins/${pluginId}`);
  }

  async deletePlugin(pluginId: string): Promise<{ message: string }> {
    return fetchJson(`/__mockforge/plugins/${pluginId}`, {
      method: 'DELETE',
    }) as Promise<{ message: string }>;
  }

  async reloadPlugin(pluginId: string): Promise<{ message: string; status: string }> {
    return fetchJson('/__mockforge/plugins/reload', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ plugin_id: pluginId }),
    }) as Promise<{ message: string; status: string }>;
  }

  async reloadAllPlugins(): Promise<{ message: string }> {
    // Get all plugins first
    const { plugins } = await this.getPlugins() as { plugins: Array<{ id: string }> };

    // Reload each plugin
    const results = await Promise.allSettled(
      plugins.map(plugin => this.reloadPlugin(plugin.id))
    );

    const failed = results.filter(r => r.status === 'rejected').length;

    if (failed > 0) {
      throw new Error(`Failed to reload ${failed} plugin(s)`);
    }

    return { message: `Successfully reloaded ${plugins.length} plugin(s)` };
  }
}

export { PluginsApiService };
