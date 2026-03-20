/**
 * Time Travel API service — temporal simulation, cron jobs, mutation rules.
 */
import { fetchJsonWithErrorBody } from './client';

class TimeTravelApiService {
  // Time Travel Status
  async getStatus(): Promise<{
    enabled: boolean;
    current_time?: string;
    scale_factor: number;
    real_time: string;
  }> {
    return fetchJsonWithErrorBody('/__mockforge/time-travel/status') as Promise<{
      enabled: boolean;
      current_time?: string;
      scale_factor: number;
      real_time: string;
    }>;
  }

  async enable(time?: string, scale?: number): Promise<{
    success: boolean;
    status: {
      enabled: boolean;
      current_time?: string;
      scale_factor: number;
    };
  }> {
    return fetchJsonWithErrorBody('/__mockforge/time-travel/enable', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ time, scale }),
    }) as Promise<{
      success: boolean;
      status: {
        enabled: boolean;
        current_time?: string;
        scale_factor: number;
      };
    }>;
  }

  async disable(): Promise<{ success: boolean }> {
    return fetchJsonWithErrorBody('/__mockforge/time-travel/disable', {
      method: 'POST',
    }) as Promise<{ success: boolean }>;
  }

  async advance(duration: string): Promise<{
    success: boolean;
    status: {
      enabled: boolean;
      current_time?: string;
      scale_factor: number;
    };
  }> {
    return fetchJsonWithErrorBody('/__mockforge/time-travel/advance', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ duration }),
    }) as Promise<{
      success: boolean;
      status: {
        enabled: boolean;
        current_time?: string;
        scale_factor: number;
      };
    }>;
  }

  async setTime(time: string): Promise<{
    success: boolean;
    status: {
      enabled: boolean;
      current_time?: string;
      scale_factor: number;
    };
  }> {
    return fetchJsonWithErrorBody('/__mockforge/time-travel/set', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ time }),
    }) as Promise<{
      success: boolean;
      status: {
        enabled: boolean;
        current_time?: string;
        scale_factor: number;
      };
    }>;
  }

  async setScale(scale: number): Promise<{ success: boolean }> {
    return fetchJsonWithErrorBody('/__mockforge/time-travel/scale', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ scale }),
    }) as Promise<{ success: boolean }>;
  }

  async reset(): Promise<{ success: boolean }> {
    return fetchJsonWithErrorBody('/__mockforge/time-travel/reset', {
      method: 'POST',
    }) as Promise<{ success: boolean }>;
  }

  // Cron Jobs
  async listCronJobs(): Promise<{ success: boolean; jobs: unknown[] }> {
    return fetchJsonWithErrorBody('/__mockforge/time-travel/cron') as Promise<{
      success: boolean;
      jobs: unknown[];
    }>;
  }

  async getCronJob(id: string): Promise<{ success: boolean; job: unknown }> {
    return fetchJsonWithErrorBody(`/__mockforge/time-travel/cron/${id}`) as Promise<{
      success: boolean;
      job: unknown;
    }>;
  }

  async createCronJob(job: {
    id: string;
    name: string;
    schedule: string;
    description?: string;
    action_type: string;
    action_metadata: unknown;
  }): Promise<{ success: boolean; message: string }> {
    return fetchJsonWithErrorBody('/__mockforge/time-travel/cron', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(job),
    }) as Promise<{ success: boolean; message: string }>;
  }

  async deleteCronJob(id: string): Promise<{ success: boolean; message: string }> {
    return fetchJsonWithErrorBody(`/__mockforge/time-travel/cron/${id}`, {
      method: 'DELETE',
    }) as Promise<{ success: boolean; message: string }>;
  }

  async setCronJobEnabled(id: string, enabled: boolean): Promise<{
    success: boolean;
    message: string;
  }> {
    return fetchJsonWithErrorBody(`/__mockforge/time-travel/cron/${id}/enable`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ enabled }),
    }) as Promise<{ success: boolean; message: string }>;
  }

  // Mutation Rules
  async listMutationRules(): Promise<{ success: boolean; rules: unknown[] }> {
    return fetchJsonWithErrorBody('/__mockforge/time-travel/mutations') as Promise<{
      success: boolean;
      rules: unknown[];
    }>;
  }

  async getMutationRule(id: string): Promise<{ success: boolean; rule: unknown }> {
    return fetchJsonWithErrorBody(`/__mockforge/time-travel/mutations/${id}`) as Promise<{
      success: boolean;
      rule: unknown;
    }>;
  }

  async createMutationRule(rule: {
    id: string;
    entity_name: string;
    trigger: unknown;
    operation: unknown;
    description?: string;
    condition?: string;
  }): Promise<{ success: boolean; message: string }> {
    return fetchJsonWithErrorBody('/__mockforge/time-travel/mutations', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(rule),
    }) as Promise<{ success: boolean; message: string }>;
  }

  async deleteMutationRule(id: string): Promise<{ success: boolean; message: string }> {
    return fetchJsonWithErrorBody(`/__mockforge/time-travel/mutations/${id}`, {
      method: 'DELETE',
    }) as Promise<{ success: boolean; message: string }>;
  }

  async setMutationRuleEnabled(id: string, enabled: boolean): Promise<{
    success: boolean;
    message: string;
  }> {
    return fetchJsonWithErrorBody(`/__mockforge/time-travel/mutations/${id}/enable`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ enabled }),
    }) as Promise<{ success: boolean; message: string }>;
  }
}

export { TimeTravelApiService };
