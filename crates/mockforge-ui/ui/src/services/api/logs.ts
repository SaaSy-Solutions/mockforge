/**
 * Logs API service — request log listing and clearing.
 */
import type { RequestLog } from '../../types';
import { LogsResponseSchema } from '../../schemas/api';
import { fetchJson, fetchJsonWithValidation } from './client';

class LogsApiService {
  constructor() {
    this.getLogs = this.getLogs.bind(this);
    this.clearLogs = this.clearLogs.bind(this);
  }

  async getLogs(params?: Record<string, string | number>): Promise<RequestLog[]> {
    let url = '/__mockforge/logs';

    if (params && Object.keys(params).length > 0) {
      // Convert all values to strings for URLSearchParams
      const stringParams: Record<string, string> = {};
      for (const [key, value] of Object.entries(params)) {
        if (value !== undefined && value !== null) {
          stringParams[key] = String(value);
        }
      }
      if (Object.keys(stringParams).length > 0) {
        const queryString = '?' + new URLSearchParams(stringParams).toString();
        url = `/__mockforge/logs${queryString}`;
      }
    }

    return fetchJsonWithValidation<RequestLog[]>(url, LogsResponseSchema);
  }

  async clearLogs(): Promise<{ message: string }> {
    return fetchJson('/__mockforge/logs', {
      method: 'DELETE',
    }) as Promise<{ message: string }>;
  }
}

export { LogsApiService };
