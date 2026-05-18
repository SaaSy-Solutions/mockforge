/**
 * Cloud world-state API client (#464) — Phase 1.
 *
 * Backs the registry's `/api/v1/hosted-mocks/{deployment_id}/world-state/*`
 * routes. The registry proxies over Fly 6PN to the running mockforge
 * instance's main HTTP port (`{fly-app}.internal:3000/api/world-state/*`)
 * — same reachability story as cloudTimeTravel (the admin port isn't
 * always exposed publicly on hosted mocks).
 *
 * The runtime exposes 6 endpoints; this client covers the 5 HTTP ones.
 * The WebSocket `/stream` endpoint is deferred to Phase 2 — for now
 * cloud users get polling-based refresh (the local UI defaults to
 * `refetchInterval: 5000` anyway, so behaviour is parity).
 *
 * `runtime_state` mirrors cloudResilience / cloudTimeTravel:
 * * `'live'`        — proxy succeeded; `data` is the deployment's response.
 * * `'unreachable'` — proxy failed; `data` is `null`. UI renders empty state.
 */
import { fetchJsonWithErrorBody } from './client';

export type WorldStateRuntimeState = 'live' | 'unreachable';

export interface CloudWorldStateEnvelope<T = unknown> {
  runtime_state: WorldStateRuntimeState;
  data: T | null;
}

class CloudWorldStateApiService {
  private base(deploymentId: string): string {
    return `/api/v1/hosted-mocks/${encodeURIComponent(deploymentId)}/world-state`;
  }

  /** Current snapshot of the deployment's world state. */
  async getSnapshot<T = unknown>(deploymentId: string): Promise<CloudWorldStateEnvelope<T>> {
    return fetchJsonWithErrorBody(`${this.base(deploymentId)}/snapshot`) as Promise<
      CloudWorldStateEnvelope<T>
    >;
  }

  /** Historical snapshot by id. */
  async getSnapshotById<T = unknown>(
    deploymentId: string,
    snapshotId: string,
  ): Promise<CloudWorldStateEnvelope<T>> {
    return fetchJsonWithErrorBody(
      `${this.base(deploymentId)}/snapshot/${encodeURIComponent(snapshotId)}`,
    ) as Promise<CloudWorldStateEnvelope<T>>;
  }

  /**
   * State graph (nodes + edges + layers). Optional `layers` is a comma-
   * separated list of layer ids; the runtime owns the filter semantics.
   */
  async getGraph<T = unknown>(
    deploymentId: string,
    layers?: string,
  ): Promise<CloudWorldStateEnvelope<T>> {
    const qs = layers && layers.length > 0 ? `?layers=${encodeURIComponent(layers)}` : '';
    return fetchJsonWithErrorBody(`${this.base(deploymentId)}/graph${qs}`) as Promise<
      CloudWorldStateEnvelope<T>
    >;
  }

  /** List of layers available on this deployment. */
  async getLayers<T = unknown>(deploymentId: string): Promise<CloudWorldStateEnvelope<T>> {
    return fetchJsonWithErrorBody(`${this.base(deploymentId)}/layers`) as Promise<
      CloudWorldStateEnvelope<T>
    >;
  }

  /**
   * Slice / filter query. Body is forwarded verbatim to the runtime's
   * `WorldStateQueryRequest` schema (optional `node_type`, `layer`,
   * `since` fields today; the runtime is the source of truth).
   */
  async query<T = unknown>(
    deploymentId: string,
    request: unknown,
  ): Promise<CloudWorldStateEnvelope<T>> {
    return fetchJsonWithErrorBody(`${this.base(deploymentId)}/query`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request ?? {}),
    }) as Promise<CloudWorldStateEnvelope<T>>;
  }
}

export { CloudWorldStateApiService };
export const cloudWorldStateApi = new CloudWorldStateApiService();
