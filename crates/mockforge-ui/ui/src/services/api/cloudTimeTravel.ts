/**
 * Cloud time-travel API client (#466).
 *
 * Backs the registry's `/api/v1/hosted-mocks/{deployment_id}/time-travel/*`
 * routes. The registry proxies over Fly 6PN to the running mockforge
 * instance's main HTTP port (`{fly-app}.internal:3000/__mockforge/time-travel/*`)
 * — not the admin port like resilience, because the time-travel routes
 * are mounted on the main HTTP app for reachability on hosted mocks where
 * the admin port isn't exposed publicly.
 *
 * Only the 7 clock-control endpoints are exposed in cloud — cron jobs and
 * mutation rules stay local-only (they don't belong to a hosted mock's
 * single-process clock).
 *
 * `runtime_state` values mirror cloudResilience:
 * * `'live'` — proxy succeeded; `data` is the deployment's current state.
 * * `'unreachable'` — registry could not reach the deployment; `data` is a
 *   synthesized "disabled" state so the existing UI rendering path keeps
 *   working. Show an "unreachable" banner alongside.
 */
import { fetchJsonWithErrorBody } from './client';

export type TimeTravelRuntimeState = 'live' | 'unreachable';

export interface CloudTimeTravelStatus {
  enabled: boolean;
  current_time?: string;
  scale_factor: number;
  real_time: string;
}

export interface CloudTimeTravelStatusEnvelope {
  runtime_state: TimeTravelRuntimeState;
  data: CloudTimeTravelStatus;
}

/** Mutation result shape (enable / disable / advance / set / scale / reset). */
export interface CloudTimeTravelMutationResult {
  accepted: boolean;
  runtime_state: TimeTravelRuntimeState;
  /** Present on `unreachable`; describes the upstream failure. */
  reason?: string;
  /** Present on `live`; the runtime's response body (status / success / etc.). */
  upstream?: unknown;
}

class CloudTimeTravelApiService {
  private base(deploymentId: string): string {
    return `/api/v1/hosted-mocks/${encodeURIComponent(deploymentId)}/time-travel`;
  }

  async getStatus(deploymentId: string): Promise<CloudTimeTravelStatusEnvelope> {
    return fetchJsonWithErrorBody(`${this.base(deploymentId)}/status`) as Promise<CloudTimeTravelStatusEnvelope>;
  }

  async enable(
    deploymentId: string,
    time?: string,
    scale?: number,
  ): Promise<CloudTimeTravelMutationResult> {
    return fetchJsonWithErrorBody(`${this.base(deploymentId)}/enable`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ time, scale }),
    }) as Promise<CloudTimeTravelMutationResult>;
  }

  async disable(deploymentId: string): Promise<CloudTimeTravelMutationResult> {
    return fetchJsonWithErrorBody(`${this.base(deploymentId)}/disable`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: '{}',
    }) as Promise<CloudTimeTravelMutationResult>;
  }

  async advance(deploymentId: string, duration: string): Promise<CloudTimeTravelMutationResult> {
    return fetchJsonWithErrorBody(`${this.base(deploymentId)}/advance`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ duration }),
    }) as Promise<CloudTimeTravelMutationResult>;
  }

  async setTime(deploymentId: string, time: string): Promise<CloudTimeTravelMutationResult> {
    return fetchJsonWithErrorBody(`${this.base(deploymentId)}/set`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ time }),
    }) as Promise<CloudTimeTravelMutationResult>;
  }

  async setScale(deploymentId: string, scale: number): Promise<CloudTimeTravelMutationResult> {
    return fetchJsonWithErrorBody(`${this.base(deploymentId)}/scale`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ scale }),
    }) as Promise<CloudTimeTravelMutationResult>;
  }

  async reset(deploymentId: string): Promise<CloudTimeTravelMutationResult> {
    return fetchJsonWithErrorBody(`${this.base(deploymentId)}/reset`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: '{}',
    }) as Promise<CloudTimeTravelMutationResult>;
  }
}

export { CloudTimeTravelApiService };
export const cloudTimeTravelApi = new CloudTimeTravelApiService();
