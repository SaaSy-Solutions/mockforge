/**
 * Dashboard API service — dashboard data and health checks.
 */
import type { DashboardData, HealthCheck } from '../../types';
import { DashboardResponseSchema } from '../../schemas/api';
import { fetchJson, fetchJsonWithValidation } from './client';

const isCloud = !!import.meta.env.VITE_API_BASE_URL;

class DashboardApiService {
  async getDashboard(): Promise<DashboardData> {
    if (isCloud) {
      return fetchJson('/api/v1/dashboard') as Promise<DashboardData>;
    }
    return fetchJsonWithValidation<DashboardData>(
      '/__mockforge/dashboard',
      DashboardResponseSchema
    );
  }

  async getHealth(): Promise<HealthCheck> {
    if (isCloud) {
      return fetchJson('/api/v1/dashboard/health') as Promise<HealthCheck>;
    }
    return fetchJson('/__mockforge/health') as Promise<HealthCheck>;
  }
}

export { DashboardApiService };
