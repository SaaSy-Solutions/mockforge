/**
 * Dashboard API service — dashboard data and health checks.
 */
import type { DashboardData, HealthCheck } from '../../types';
import { DashboardResponseSchema } from '../../schemas/api';
import { fetchJson, fetchJsonWithValidation } from './client';

class DashboardApiService {
  async getDashboard(): Promise<DashboardData> {
    return fetchJsonWithValidation<DashboardData>(
      '/__mockforge/dashboard',
      DashboardResponseSchema
    );
  }

  async getHealth(): Promise<HealthCheck> {
    return fetchJson('/__mockforge/health') as Promise<HealthCheck>;
  }
}

export { DashboardApiService };
