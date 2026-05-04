/**
 * Cloud tunnels API client (#5).
 *
 * Wraps the registry-server `/api/v1/organizations/{org_id}/tunnels` +
 * `/api/v1/tunnels/{id}` routes including DNS-backed custom-domain
 * verification.
 */
import { fetchJsonWithErrorBody } from './client';
import { isCloudMode } from '../../utils/cloudMode';

export interface TunnelReservation {
  id: string;
  org_id: string;
  workspace_id: string | null;
  name: string;
  subdomain: string;
  custom_domain: string | null;
  custom_domain_verified: boolean;
  custom_domain_verified_at: string | null;
  status: string;
  created_by: string | null;
  created_at: string;
  updated_at: string;
}

export interface CreateTunnelRequest {
  name: string;
  subdomain: string;
  workspace_id?: string;
  custom_domain?: string;
}

export interface UpdateTunnelRequest {
  name?: string;
  /** Set to null to clear the custom domain. */
  custom_domain?: string | null;
}

export interface CustomDomainProof {
  txt_record_name: string;
  txt_record_value: string;
  zone_file_line: string;
}

class CloudTunnelsApi {
  private guard(method: string): void {
    if (!isCloudMode()) {
      throw new Error(`Cloud tunnels ${method} only works in cloud mode.`);
    }
  }

  async listForOrg(orgId: string): Promise<TunnelReservation[]> {
    this.guard('listForOrg');
    return fetchJsonWithErrorBody(
      `/api/v1/organizations/${orgId}/tunnels`,
    ) as Promise<TunnelReservation[]>;
  }

  async create(
    orgId: string,
    body: CreateTunnelRequest,
  ): Promise<TunnelReservation> {
    this.guard('create');
    return fetchJsonWithErrorBody(`/api/v1/organizations/${orgId}/tunnels`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    }) as Promise<TunnelReservation>;
  }

  async get(id: string): Promise<TunnelReservation> {
    this.guard('get');
    return fetchJsonWithErrorBody(
      `/api/v1/tunnels/${id}`,
    ) as Promise<TunnelReservation>;
  }

  async update(id: string, body: UpdateTunnelRequest): Promise<TunnelReservation> {
    this.guard('update');
    return fetchJsonWithErrorBody(`/api/v1/tunnels/${id}`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    }) as Promise<TunnelReservation>;
  }

  async delete(id: string): Promise<{ deleted: boolean }> {
    this.guard('delete');
    return fetchJsonWithErrorBody(`/api/v1/tunnels/${id}`, {
      method: 'DELETE',
    }) as Promise<{ deleted: boolean }>;
  }

  async verifyCustomDomain(id: string): Promise<TunnelReservation> {
    this.guard('verifyCustomDomain');
    return fetchJsonWithErrorBody(
      `/api/v1/tunnels/${id}/verify-custom-domain`,
      { method: 'POST' },
    ) as Promise<TunnelReservation>;
  }

  async getCustomDomainProof(id: string): Promise<CustomDomainProof> {
    this.guard('getCustomDomainProof');
    return fetchJsonWithErrorBody(
      `/api/v1/tunnels/${id}/custom-domain-proof`,
    ) as Promise<CustomDomainProof>;
  }
}

export const cloudTunnelsApi = new CloudTunnelsApi();
