//! Drift Budget and Incident Management API
//!
//! This module provides API client functions for drift budget and incident management.

import { authenticatedFetch } from '../utils/apiClient';

// Type definitions matching backend types
export interface DriftBudget {
  id: string;
  endpoint: string;
  method: string;
  max_breaking_changes: number;
  max_non_breaking_changes: number;
  breaking_change_rules: unknown[];
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface CreateDriftBudgetRequest {
  endpoint: string;
  method: string;
  max_breaking_changes?: number;
  max_non_breaking_changes?: number;
  severity_threshold?: string;
  enabled?: boolean;
  workspace_id?: string;
}

export interface DriftBudgetResponse {
  id: string;
  endpoint: string;
  method: string;
  budget: DriftBudget;
  workspace_id?: string;
}

export type IncidentStatus = 'open' | 'acknowledged' | 'resolved' | 'closed';
export type IncidentType = 'breaking_change' | 'threshold_exceeded';
export type IncidentSeverity = 'low' | 'medium' | 'high' | 'critical';

export interface DriftIncident {
  id: string;
  budget_id?: string;
  workspace_id?: string;
  endpoint: string;
  method: string;
  incident_type: IncidentType;
  severity: IncidentSeverity;
  status: IncidentStatus;
  detected_at: number;
  resolved_at?: number;
  details: Record<string, unknown>;
  external_ticket_id?: string;
  external_ticket_url?: string;
  created_at: number;
  updated_at: number;
}

export interface ListIncidentsRequest {
  status?: IncidentStatus;
  severity?: IncidentSeverity;
  endpoint?: string;
  method?: string;
  incident_type?: IncidentType;
  workspace_id?: string;
  limit?: number;
  offset?: number;
}

export interface ListIncidentsResponse {
  incidents: DriftIncident[];
  total: number;
}

export interface UpdateIncidentRequest {
  status?: IncidentStatus;
  external_ticket_id?: string;
  external_ticket_url?: string;
}

export interface IncidentStatistics {
  total: number;
  by_status: Record<IncidentStatus, number>;
  by_severity: Record<IncidentSeverity, number>;
  by_type: Record<IncidentType, number>;
}

class DriftBudgetApiService {
  private async fetchJson(url: string, options?: RequestInit): Promise<unknown> {
    const response = await authenticatedFetch(url, options);
    if (!response.ok) {
      if (response.status === 401) {
        throw new Error('Authentication required');
      }
      if (response.status === 403) {
        throw new Error('Access denied');
      }
      const errorText = await response.text();
      let errorMessage = `HTTP error! status: ${response.status}`;
      try {
        const errorJson = JSON.parse(errorText);
        errorMessage = errorJson.error || errorMessage;
      } catch {
        // Not JSON, use default message
      }
      throw new Error(errorMessage);
    }
    const json = await response.json();
    return json.data || json;
  }

  /**
   * Create or update a drift budget
   * POST /api/v1/drift/budgets
   */
  async createOrUpdateBudget(request: CreateDriftBudgetRequest): Promise<DriftBudgetResponse> {
    return this.fetchJson('/api/v1/drift/budgets', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as Promise<DriftBudgetResponse>;
  }

  /**
   * List drift budgets
   * GET /api/v1/drift/budgets
   */
  async listBudgets(params?: {
    endpoint?: string;
    method?: string;
    workspace_id?: string;
  }): Promise<{ budgets: DriftBudgetResponse[] }> {
    const queryParams = new URLSearchParams();
    if (params) {
      Object.entries(params).forEach(([key, value]) => {
        if (value !== undefined) {
          queryParams.append(key, String(value));
        }
      });
    }
    const url = `/api/v1/drift/budgets${queryParams.toString() ? `?${queryParams}` : ''}`;
    return this.fetchJson(url) as Promise<{ budgets: DriftBudgetResponse[] }>;
  }

  /**
   * Get a specific drift budget
   * GET /api/v1/drift/budgets/{id}
   */
  async getBudget(id: string): Promise<DriftBudgetResponse> {
    return this.fetchJson(`/api/v1/drift/budgets/${id}`) as Promise<DriftBudgetResponse>;
  }

  /**
   * List incidents
   * GET /api/v1/drift/incidents
   */
  async listIncidents(params?: ListIncidentsRequest): Promise<ListIncidentsResponse> {
    const queryParams = new URLSearchParams();
    if (params) {
      Object.entries(params).forEach(([key, value]) => {
        if (value !== undefined) {
          queryParams.append(key, String(value));
        }
      });
    }
    const url = `/api/v1/drift/incidents${queryParams.toString() ? `?${queryParams}` : ''}`;
    return this.fetchJson(url) as Promise<ListIncidentsResponse>;
  }

  /**
   * Get a specific incident
   * GET /api/v1/drift/incidents/{id}
   */
  async getIncident(id: string): Promise<{ incident: DriftIncident }> {
    return this.fetchJson(`/api/v1/drift/incidents/${id}`) as Promise<{ incident: DriftIncident }>;
  }

  /**
   * Update an incident
   * PATCH /api/v1/drift/incidents/{id}
   */
  async updateIncident(id: string, request: UpdateIncidentRequest): Promise<{ incident: DriftIncident }> {
    return this.fetchJson(`/api/v1/drift/incidents/${id}`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as Promise<{ incident: DriftIncident }>;
  }

  /**
   * Resolve an incident
   * POST /api/v1/drift/incidents/{id}/resolve
   */
  async resolveIncident(id: string): Promise<{ incident: DriftIncident }> {
    return this.fetchJson(`/api/v1/drift/incidents/${id}/resolve`, {
      method: 'POST',
    }) as Promise<{ incident: DriftIncident }>;
  }

  /**
   * Get incident statistics
   * GET /api/v1/drift/incidents/stats
   */
  async getIncidentStatistics(): Promise<{ statistics: IncidentStatistics }> {
    return this.fetchJson('/api/v1/drift/incidents/stats') as Promise<{ statistics: IncidentStatistics }>;
  }
}

export const driftApi = new DriftBudgetApiService();
