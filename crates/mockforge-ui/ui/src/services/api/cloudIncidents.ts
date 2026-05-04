/**
 * Incidents API client — wraps the registry-server endpoints under
 * `/api/v1/organizations/{org_id}/incidents` and `/api/v1/incidents/{id}`.
 *
 * See docs/cloud/CLOUD_INCIDENTS_DESIGN.md.
 */
import { fetchJsonWithErrorBody } from './client';
import { isCloudMode } from '../../utils/cloudMode';

export type IncidentSeverity = 'critical' | 'high' | 'medium' | 'low';
export type IncidentStatus =
  | 'open'
  | 'acknowledged'
  | 'resolved'
  | 'reopened';

export interface Incident {
  id: string;
  org_id: string;
  workspace_id: string | null;
  source: string;
  source_ref: string | null;
  dedupe_key: string;
  severity: IncidentSeverity;
  status: IncidentStatus;
  title: string;
  description: string | null;
  postmortem_url: string | null;
  assigned_to: string | null;
  acknowledged_at: string | null;
  acknowledged_by: string | null;
  resolved_at: string | null;
  resolved_by: string | null;
  created_at: string;
  updated_at: string;
}

export interface IncidentEvent {
  id: string;
  incident_id: string;
  event_type: string;
  actor_id: string | null;
  payload: Record<string, unknown> | null;
  created_at: string;
}

export interface RaiseIncidentRequest {
  source: string;
  source_ref?: string;
  dedupe_key: string;
  severity: IncidentSeverity;
  title: string;
  description?: string;
  workspace_id?: string;
}

export interface IncidentSeverityBreakdown {
  total: number;
  critical: number;
  high: number;
  medium: number;
  low: number;
}

export interface IncidentStats {
  open: IncidentSeverityBreakdown;
  resolved_30d: IncidentSeverityBreakdown;
  mttr_seconds_30d: number | null;
  notification_attempts_24h: number;
}

class CloudIncidentsApi {
  private guard(method: string): void {
    if (!isCloudMode()) {
      throw new Error(
        `Cloud incidents ${method} is only available in cloud mode.`,
      );
    }
  }

  async listForOrg(
    orgId: string,
    opts?: { status?: IncidentStatus; severity?: IncidentSeverity; limit?: number },
  ): Promise<Incident[]> {
    this.guard('listForOrg');
    const params = new URLSearchParams();
    if (opts?.status) params.set('status', opts.status);
    if (opts?.severity) params.set('severity', opts.severity);
    if (opts?.limit) params.set('limit', String(opts.limit));
    const qs = params.toString() ? `?${params}` : '';
    return fetchJsonWithErrorBody(
      `/api/v1/organizations/${orgId}/incidents${qs}`,
    ) as Promise<Incident[]>;
  }

  async raise(orgId: string, body: RaiseIncidentRequest): Promise<Incident> {
    this.guard('raise');
    return fetchJsonWithErrorBody(`/api/v1/organizations/${orgId}/incidents`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    }) as Promise<Incident>;
  }

  async get(id: string): Promise<Incident> {
    this.guard('get');
    return fetchJsonWithErrorBody(
      `/api/v1/incidents/${id}`,
    ) as Promise<Incident>;
  }

  async listEvents(id: string): Promise<IncidentEvent[]> {
    this.guard('listEvents');
    return fetchJsonWithErrorBody(
      `/api/v1/incidents/${id}/events`,
    ) as Promise<IncidentEvent[]>;
  }

  async acknowledge(id: string): Promise<Incident> {
    this.guard('acknowledge');
    return fetchJsonWithErrorBody(`/api/v1/incidents/${id}/acknowledge`, {
      method: 'POST',
    }) as Promise<Incident>;
  }

  async resolve(id: string): Promise<Incident> {
    this.guard('resolve');
    return fetchJsonWithErrorBody(`/api/v1/incidents/${id}/resolve`, {
      method: 'POST',
    }) as Promise<Incident>;
  }

  async getStats(orgId: string): Promise<IncidentStats> {
    this.guard('getStats');
    return fetchJsonWithErrorBody(
      `/api/v1/organizations/${orgId}/incidents/stats`,
    ) as Promise<IncidentStats>;
  }
}

export const cloudIncidentsApi = new CloudIncidentsApi();
