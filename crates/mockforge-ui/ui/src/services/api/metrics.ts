/**
 * Metrics API service — metrics data retrieval.
 */
import type { MetricsData } from '../../types';
import { fetchJson } from './client';

class MetricsApiService {
  constructor() {
    this.getMetrics = this.getMetrics.bind(this);
  }

  async getMetrics(): Promise<MetricsData> {
    return fetchJson('/__mockforge/metrics') as Promise<MetricsData>;
  }
}

export { MetricsApiService };
