/**
 * Cloud resilience API client (#468).
 *
 * Backs the registry's `/api/v1/hosted-mocks/{deployment_id}/resilience/*`
 * routes. The registry proxies over Fly 6PN to the running mockforge
 * instance's admin port (`{fly-app}.internal:9080`) and tags the response
 * with `runtime_state`.
 *
 * `runtime_state` values:
 * * `'live'` — proxy succeeded; `data` is the deployment's current state.
 * * `'unreachable'` — registry could not reach the deployment (not yet
 *   deployed, connection refused, timeout, etc.); `data` is empty. Show
 *   an empty state with a "deployment not reachable" hint.
 */
import { fetchJsonWithErrorBody } from './client';
import { isCloudMode } from '../../utils/cloudMode';

export type RuntimeState = 'live' | 'unreachable';

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
    deploymentId: string,
  ): Promise<ResilienceEnvelope<CloudCircuitBreakerState>> {
    this.guard('listCircuitBreakers');
    return fetchJsonWithErrorBody(
      `/api/v1/hosted-mocks/${encodeURIComponent(deploymentId)}/resilience/circuit-breakers`,
    ) as Promise<ResilienceEnvelope<CloudCircuitBreakerState>>;
  }

  async listBulkheads(deploymentId: string): Promise<ResilienceEnvelope<CloudBulkheadState>> {
    this.guard('listBulkheads');
    return fetchJsonWithErrorBody(
      `/api/v1/hosted-mocks/${encodeURIComponent(deploymentId)}/resilience/bulkheads`,
    ) as Promise<ResilienceEnvelope<CloudBulkheadState>>;
  }

  async getSummary(deploymentId: string): Promise<CloudResilienceSummary> {
    this.guard('getSummary');
    return fetchJsonWithErrorBody(
      `/api/v1/hosted-mocks/${encodeURIComponent(deploymentId)}/resilience/summary`,
    ) as Promise<CloudResilienceSummary>;
  }

  async resetCircuitBreaker(
    deploymentId: string,
    endpoint: string,
  ): Promise<CloudResilienceResetResult> {
    this.guard('resetCircuitBreaker');
    return fetchJsonWithErrorBody(
      `/api/v1/hosted-mocks/${encodeURIComponent(deploymentId)}/resilience/circuit-breakers/${encodeURIComponent(endpoint)}/reset`,
      { method: 'POST' },
    ) as Promise<CloudResilienceResetResult>;
  }

  async resetBulkhead(
    deploymentId: string,
    service: string,
  ): Promise<CloudResilienceResetResult> {
    this.guard('resetBulkhead');
    return fetchJsonWithErrorBody(
      `/api/v1/hosted-mocks/${encodeURIComponent(deploymentId)}/resilience/bulkheads/${encodeURIComponent(service)}/reset`,
      { method: 'POST' },
    ) as Promise<CloudResilienceResetResult>;
  }
}

export const cloudResilienceApi = new CloudResilienceApi();
