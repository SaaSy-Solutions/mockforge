/**
 * Cloud observability API client (#2).
 *
 * Wraps saved queries + dashboards + the cross-deployment trace query.
 */
import { fetchJsonWithErrorBody } from './client';
import { isCloudMode } from '../../utils/cloudMode';

export interface ObservabilitySavedQuery {
  id: string;
  org_id: string;
  workspace_id: string | null;
  name: string;
  description: string | null;
  kind: string;
  filters: Record<string, unknown>;
  created_by: string | null;
  created_at: string;
  updated_at: string;
}

export interface CreateSavedQueryRequest {
  name: string;
  description?: string;
  kind: string;
  filters: Record<string, unknown>;
  workspace_id?: string;
}

export interface ObservabilityDashboard {
  id: string;
  org_id: string;
  workspace_id: string | null;
  name: string;
  description: string | null;
  layout: Record<string, unknown>;
  queries: Record<string, unknown>;
  created_by: string | null;
  created_at: string;
  updated_at: string;
}

export interface TraceSpanRow {
  deployment_id: string;
  trace_id: string;
  span_id: string;
  parent_span_id: string | null;
  service_name: string | null;
  name: string;
  kind: number | null;
  start_unix_nano: number;
  end_unix_nano: number;
  occurred_at: string;
  status_code: number | null;
  status_message: string | null;
  attributes: Record<string, unknown>;
}

export interface TraceQueryRequest {
  deployment_id?: string;
  service_name?: string;
  name_contains?: string;
  status?: 'ok' | 'error' | 'any';
  since?: string;
  until?: string;
  limit?: number;
}

class CloudObservabilityApi {
  private guard(method: string): void {
    if (!isCloudMode()) {
      throw new Error(`Cloud observability ${method} only works in cloud mode.`);
    }
  }

  async listSavedQueries(
    orgId: string,
    kind?: string,
  ): Promise<ObservabilitySavedQuery[]> {
    this.guard('listSavedQueries');
    const qs = kind ? `?kind=${encodeURIComponent(kind)}` : '';
    return fetchJsonWithErrorBody(
      `/api/v1/organizations/${orgId}/observability/saved-queries${qs}`,
    ) as Promise<ObservabilitySavedQuery[]>;
  }

  async createSavedQuery(
    orgId: string,
    body: CreateSavedQueryRequest,
  ): Promise<ObservabilitySavedQuery> {
    this.guard('createSavedQuery');
    return fetchJsonWithErrorBody(
      `/api/v1/organizations/${orgId}/observability/saved-queries`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      },
    ) as Promise<ObservabilitySavedQuery>;
  }

  async listDashboards(orgId: string): Promise<ObservabilityDashboard[]> {
    this.guard('listDashboards');
    return fetchJsonWithErrorBody(
      `/api/v1/organizations/${orgId}/observability/dashboards`,
    ) as Promise<ObservabilityDashboard[]>;
  }

  /** Cross-deployment trace search. POSTed JSON because the filter set is too wide for a query string. */
  async queryTraces(
    orgId: string,
    body: TraceQueryRequest = {},
  ): Promise<TraceSpanRow[]> {
    this.guard('queryTraces');
    return fetchJsonWithErrorBody(
      `/api/v1/organizations/${orgId}/observability/traces/query`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      },
    ) as Promise<TraceSpanRow[]>;
  }
}

export const cloudObservabilityApi = new CloudObservabilityApi();
