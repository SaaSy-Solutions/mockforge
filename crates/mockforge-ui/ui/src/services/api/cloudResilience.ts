/**
 * Cloud resilience API client (#468 Phase 1 — cloud scaffold).
 *
 * Backs the registry's `/api/v1/workspaces/{id}/resilience/*` routes.
 *
 * Phase 1 always returns `runtime_state: 'pending'` with empty lists because
 * the hosted-mock runtime doesn't yet install the circuit-breaker / bulkhead
 * middleware. The wire-up is the bulk of #468 and lives in a follow-up;
 * shipping the API surface now means `ResiliencePage` can stop calling the
 * never-mounted `/api/resilience/*` routes and instead render an honest
 * empty state.
 *
 * Reset endpoints return `{ accepted: false, runtime_state: 'pending',
 * reason }` so the page can surface a "no-op while pending" toast instead
 * of treating the reset as failed.
 */
import { fetchJsonWithErrorBody } from './client';
import { isCloudMode } from '../../utils/cloudMode';

export type RuntimeState = 'pending' | 'live';

export interface ResilienceEnvelope<T> {
  runtime_state: RuntimeState;
  data: T[];
}

export interface CloudCircuitBreakerState {
  endpoint: string;
  state: string;
  stats: {
    total_requests: number;
    successful_requests: number;
    failed_requests: number;
    rejected_requests: number;
    consecutive_failures: number;
    consecutive_successes: number;
    success_rate: number;
    failure_rate: number;
  };
}

export interface CloudBulkheadState {
  service: string;
  stats: {
    active_requests: number;
    queued_requests: number;
    total_requests: number;
    rejected_requests: number;
    timeout_requests: number;
    utilization_percent: number;
  };
}

export interface CloudResilienceSummary {
  runtime_state: RuntimeState;
  circuit_breakers: { total: number; open: number; half_open: number; closed: number };
  bulkheads: { total: number; active_requests: number; queued_requests: number };
}

export interface CloudResilienceResetResult {
  accepted: boolean;
  runtime_state: RuntimeState;
  reason?: string;
}

class CloudResilienceApi {
  private guard(method: string): void {
    if (!isCloudMode()) {
      throw new Error(`Cloud resilience ${method} only works in cloud mode.`);
    }
  }

  async listCircuitBreakers(
    workspaceId: string,
  ): Promise<ResilienceEnvelope<CloudCircuitBreakerState>> {
    this.guard('listCircuitBreakers');
    return fetchJsonWithErrorBody(
      `/api/v1/workspaces/${workspaceId}/resilience/circuit-breakers`,
    ) as Promise<ResilienceEnvelope<CloudCircuitBreakerState>>;
  }

  async listBulkheads(workspaceId: string): Promise<ResilienceEnvelope<CloudBulkheadState>> {
    this.guard('listBulkheads');
    return fetchJsonWithErrorBody(
      `/api/v1/workspaces/${workspaceId}/resilience/bulkheads`,
    ) as Promise<ResilienceEnvelope<CloudBulkheadState>>;
  }

  async getSummary(workspaceId: string): Promise<CloudResilienceSummary> {
    this.guard('getSummary');
    return fetchJsonWithErrorBody(
      `/api/v1/workspaces/${workspaceId}/resilience/summary`,
    ) as Promise<CloudResilienceSummary>;
  }

  async resetCircuitBreaker(
    workspaceId: string,
    endpoint: string,
  ): Promise<CloudResilienceResetResult> {
    this.guard('resetCircuitBreaker');
    return fetchJsonWithErrorBody(
      `/api/v1/workspaces/${workspaceId}/resilience/circuit-breakers/${encodeURIComponent(endpoint)}/reset`,
      { method: 'POST' },
    ) as Promise<CloudResilienceResetResult>;
  }

  async resetBulkhead(
    workspaceId: string,
    service: string,
  ): Promise<CloudResilienceResetResult> {
    this.guard('resetBulkhead');
    return fetchJsonWithErrorBody(
      `/api/v1/workspaces/${workspaceId}/resilience/bulkheads/${encodeURIComponent(service)}/reset`,
      { method: 'POST' },
    ) as Promise<CloudResilienceResetResult>;
  }
}

export const cloudResilienceApi = new CloudResilienceApi();
