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
  fitness_test_results?: FitnessTestResult[];
  affected_consumers?: ConsumerImpact;
  protocol?: 'http' | 'graphql' | 'grpc' | 'websocket' | 'smtp' | 'mqtt' | 'ftp' | 'kafka' | 'rabbitmq' | 'amqp' | 'tcp' | 'udp';
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

// Fitness Function Types
export type FitnessFunctionType =
  | { type: 'response_size'; max_increase_percent: number }
  | { type: 'required_field'; path_pattern: string; allow_new_required: boolean }
  | { type: 'field_count'; max_fields: number }
  | { type: 'schema_complexity'; max_depth: number }
  | { type: 'custom'; evaluator: string };

export type FitnessScope =
  | { type: 'global' }
  | { type: 'workspace'; workspace_id: string }
  | { type: 'service'; service_name: string }
  | { type: 'endpoint'; pattern: string };

export interface FitnessFunction {
  id: string;
  name: string;
  description: string;
  function_type: FitnessFunctionType;
  config: Record<string, unknown>;
  scope: FitnessScope;
  enabled: boolean;
  created_at: number;
  updated_at: number;
}

export interface FitnessTestResult {
  function_id: string;
  function_name: string;
  passed: boolean;
  message: string;
  metrics: Record<string, number>;
}

export interface CreateFitnessFunctionRequest {
  name: string;
  description: string;
  function_type: FitnessFunctionType;
  config: Record<string, unknown>;
  scope: FitnessScope;
  enabled?: boolean;
}

export interface FitnessFunctionResponse {
  function: FitnessFunction;
}

export interface ListFitnessFunctionsResponse {
  functions: FitnessFunction[];
}

// Consumer Mapping Types
export type AppType = 'web' | 'mobile_ios' | 'mobile_android' | 'internal_tool' | 'cli' | 'other';

export interface ConsumingApp {
  app_id: string;
  app_name: string;
  app_type: AppType;
  repository_url?: string;
  last_updated?: number;
  description?: string;
}

export interface SDKMethod {
  sdk_name: string;
  method_name: string;
  consuming_apps: ConsumingApp[];
}

export interface ConsumerMapping {
  endpoint: string;
  method: string;
  sdk_methods: SDKMethod[];
  created_at: number;
  updated_at: number;
}

export interface ConsumerImpact {
  endpoint: string;
  method: string;
  affected_sdk_methods: SDKMethod[];
  affected_apps: ConsumingApp[];
  impact_summary: string;
}

export interface CreateConsumingAppRequest {
  app_id: string;
  app_name: string;
  app_type: string;
  repository_url?: string;
  description?: string;
}

export interface CreateSDKMethodRequest {
  sdk_name: string;
  method_name: string;
  consuming_apps: CreateConsumingAppRequest[];
}

export interface CreateConsumerMappingRequest {
  endpoint: string;
  method: string;
  sdk_methods: CreateSDKMethodRequest[];
}

export interface ConsumerMappingResponse {
  mapping: ConsumerMapping;
}

export interface ListConsumerMappingsResponse {
  mappings: ConsumerMapping[];
}

export interface IncidentImpactResponse {
  incident_id: string;
  impact: ConsumerImpact | null;
  message?: string;
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

  /**
   * List fitness functions
   * GET /api/v1/drift/fitness-functions
   */
  async listFitnessFunctions(): Promise<ListFitnessFunctionsResponse> {
    return this.fetchJson('/api/v1/drift/fitness-functions') as Promise<ListFitnessFunctionsResponse>;
  }

  /**
   * Get a specific fitness function
   * GET /api/v1/drift/fitness-functions/{id}
   */
  async getFitnessFunction(id: string): Promise<FitnessFunctionResponse> {
    return this.fetchJson(`/api/v1/drift/fitness-functions/${id}`) as Promise<FitnessFunctionResponse>;
  }

  /**
   * Create a fitness function
   * POST /api/v1/drift/fitness-functions
   */
  async createFitnessFunction(request: CreateFitnessFunctionRequest): Promise<FitnessFunctionResponse> {
    return this.fetchJson('/api/v1/drift/fitness-functions', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as Promise<FitnessFunctionResponse>;
  }

  /**
   * Update a fitness function
   * PATCH /api/v1/drift/fitness-functions/{id}
   */
  async updateFitnessFunction(id: string, request: CreateFitnessFunctionRequest): Promise<FitnessFunctionResponse> {
    return this.fetchJson(`/api/v1/drift/fitness-functions/${id}`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as Promise<FitnessFunctionResponse>;
  }

  /**
   * Delete a fitness function
   * DELETE /api/v1/drift/fitness-functions/{id}
   */
  async deleteFitnessFunction(id: string): Promise<{ success: boolean; message: string }> {
    return this.fetchJson(`/api/v1/drift/fitness-functions/${id}`, {
      method: 'DELETE',
    }) as Promise<{ success: boolean; message: string }>;
  }

  /**
   * Test a fitness function
   * POST /api/v1/drift/fitness-functions/{id}/test
   */
  async testFitnessFunction(
    id: string,
    request: {
      endpoint?: string;
      method?: string;
      workspace_id?: string;
      service_name?: string;
    }
  ): Promise<{ results: FitnessTestResult[] }> {
    return this.fetchJson(`/api/v1/drift/fitness-functions/${id}/test`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as Promise<{ results: FitnessTestResult[] }>;
  }

  /**
   * List consumer mappings
   * GET /api/v1/drift/consumer-mappings
   */
  async listConsumerMappings(): Promise<ListConsumerMappingsResponse> {
    return this.fetchJson('/api/v1/drift/consumer-mappings') as Promise<ListConsumerMappingsResponse>;
  }

  /**
   * Get consumer mapping for a specific endpoint
   * GET /api/v1/drift/consumer-mappings/lookup?endpoint=...&method=...
   */
  async getConsumerMapping(endpoint: string, method: string): Promise<ConsumerMappingResponse> {
    const queryParams = new URLSearchParams({ endpoint, method });
    return this.fetchJson(`/api/v1/drift/consumer-mappings/lookup?${queryParams}`) as Promise<ConsumerMappingResponse>;
  }

  /**
   * Create or update a consumer mapping
   * POST /api/v1/drift/consumer-mappings
   */
  async createConsumerMapping(request: CreateConsumerMappingRequest): Promise<ConsumerMappingResponse> {
    return this.fetchJson('/api/v1/drift/consumer-mappings', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as Promise<ConsumerMappingResponse>;
  }

  /**
   * Get consumer impact for an incident
   * GET /api/v1/drift/incidents/{id}/impact
   */
  async getIncidentImpact(id: string): Promise<IncidentImpactResponse> {
    return this.fetchJson(`/api/v1/drift/incidents/${id}/impact`) as Promise<IncidentImpactResponse>;
  }
}

export const driftApi = new DriftBudgetApiService();
