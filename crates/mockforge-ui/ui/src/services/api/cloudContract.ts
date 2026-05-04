/**
 * Cloud contract verification + diff API client (#8).
 *
 * Wraps MonitoredService CRUD + diff trigger + findings + fitness
 * functions read paths.
 */
import { fetchJsonWithErrorBody } from './client';
import { isCloudMode } from '../../utils/cloudMode';

export interface MonitoredService {
  id: string;
  workspace_id: string;
  name: string;
  base_url: string;
  openapi_spec_url: string;
  traffic_source: string;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface CreateMonitoredServiceRequest {
  name: string;
  base_url: string;
  openapi_spec_url: string;
  traffic_source: string;
}

export interface ContractDiffRun {
  id: string;
  monitored_service_id: string;
  test_run_id: string;
  status: string;
  triggered_at: string;
  completed_at: string | null;
}

export interface ContractDiffFinding {
  id: string;
  diff_run_id: string;
  severity: string;
  endpoint: string | null;
  description: string;
  payload: Record<string, unknown> | null;
  created_at: string;
}

export interface FitnessFunction {
  id: string;
  workspace_id: string;
  name: string;
  description: string | null;
  kind: string;
  config: Record<string, unknown>;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

class CloudContractApi {
  private guard(method: string): void {
    if (!isCloudMode()) {
      throw new Error(`Cloud contract ${method} only works in cloud mode.`);
    }
  }

  async listMonitoredServices(workspaceId: string): Promise<MonitoredService[]> {
    this.guard('listMonitoredServices');
    return fetchJsonWithErrorBody(
      `/api/v1/workspaces/${workspaceId}/monitored-services`,
    ) as Promise<MonitoredService[]>;
  }

  async createMonitoredService(
    workspaceId: string,
    body: CreateMonitoredServiceRequest,
  ): Promise<MonitoredService> {
    this.guard('createMonitoredService');
    return fetchJsonWithErrorBody(
      `/api/v1/workspaces/${workspaceId}/monitored-services`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      },
    ) as Promise<MonitoredService>;
  }

  async deleteMonitoredService(id: string): Promise<{ deleted: boolean }> {
    this.guard('deleteMonitoredService');
    return fetchJsonWithErrorBody(`/api/v1/monitored-services/${id}`, {
      method: 'DELETE',
    }) as Promise<{ deleted: boolean }>;
  }

  async triggerDiff(id: string): Promise<{ id: string; status: string; kind: string }> {
    this.guard('triggerDiff');
    return fetchJsonWithErrorBody(`/api/v1/monitored-services/${id}/diff`, {
      method: 'POST',
    }) as Promise<{ id: string; status: string; kind: string }>;
  }

  async listDiffRuns(serviceId: string): Promise<ContractDiffRun[]> {
    this.guard('listDiffRuns');
    return fetchJsonWithErrorBody(
      `/api/v1/monitored-services/${serviceId}/diff-runs`,
    ) as Promise<ContractDiffRun[]>;
  }

  async getDiffRun(id: string): Promise<ContractDiffRun> {
    this.guard('getDiffRun');
    return fetchJsonWithErrorBody(
      `/api/v1/diff-runs/${id}`,
    ) as Promise<ContractDiffRun>;
  }

  async listFindings(diffRunId: string): Promise<ContractDiffFinding[]> {
    this.guard('listFindings');
    return fetchJsonWithErrorBody(
      `/api/v1/diff-runs/${diffRunId}/findings`,
    ) as Promise<ContractDiffFinding[]>;
  }

  async listFitnessFunctions(workspaceId: string): Promise<FitnessFunction[]> {
    this.guard('listFitnessFunctions');
    return fetchJsonWithErrorBody(
      `/api/v1/workspaces/${workspaceId}/fitness-functions`,
    ) as Promise<FitnessFunction[]>;
  }
}

export const cloudContractApi = new CloudContractApi();
