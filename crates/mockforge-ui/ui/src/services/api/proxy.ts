/**
 * Proxy API service — proxy rules, inspection, playground, behavioral cloning, scenarios.
 */
import { fetchJsonUnauthenticated } from './client';
import type { Flow, TagFlowRequest, CompileFlowRequest, CompileFlowResponse, Scenario, ScenarioDetail } from '../../types';

// Proxy replacement rules API types
export interface ProxyRule {
  id: number;
  pattern: string;
  type: 'request' | 'response';
  status_codes: number[];
  body_transforms: Array<{
    path: string;
    replace: string;
    operation: 'replace' | 'add' | 'remove';
  }>;
  enabled: boolean;
}

export interface ProxyRuleRequest {
  pattern: string;
  type: 'request' | 'response';
  status_codes?: number[];
  body_transforms: Array<{
    path: string;
    replace: string;
    operation?: 'replace' | 'add' | 'remove';
  }>;
  enabled?: boolean;
}

export interface ProxyRulesResponse {
  rules: ProxyRule[];
}

export interface ProxyInspectResponse {
  requests: Array<{
    id: string;
    timestamp: string;
    method: string;
    url: string;
    headers: Record<string, string>;
    body?: string;
  }>;
  responses: Array<{
    id: string;
    timestamp: string;
    status_code: number;
    headers: Record<string, string>;
    body?: string;
  }>;
  limit: number;
  message?: string;
}

class ProxyApiService {
  async getProxyRules(): Promise<ProxyRulesResponse> {
    return fetchJsonUnauthenticated('/__mockforge/api/proxy/rules') as Promise<ProxyRulesResponse>;
  }

  async getProxyRule(id: number): Promise<ProxyRule> {
    return fetchJsonUnauthenticated(`/__mockforge/api/proxy/rules/${id}`) as Promise<ProxyRule>;
  }

  async createProxyRule(rule: ProxyRuleRequest): Promise<{ id: number; message: string }> {
    return fetchJsonUnauthenticated('/__mockforge/api/proxy/rules', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(rule),
    }) as Promise<{ id: number; message: string }>;
  }

  async updateProxyRule(id: number, rule: ProxyRuleRequest): Promise<{ id: number; message: string }> {
    return fetchJsonUnauthenticated(`/__mockforge/api/proxy/rules/${id}`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(rule),
    }) as Promise<{ id: number; message: string }>;
  }

  async deleteProxyRule(id: number): Promise<{ id: number; message: string }> {
    return fetchJsonUnauthenticated(`/__mockforge/api/proxy/rules/${id}`, {
      method: 'DELETE',
    }) as Promise<{ id: number; message: string }>;
  }

  async getProxyInspect(limit?: number): Promise<ProxyInspectResponse> {
    const url = limit ? `/__mockforge/api/proxy/inspect?limit=${limit}` : '/__mockforge/api/proxy/inspect';
    return fetchJsonUnauthenticated(url) as Promise<ProxyInspectResponse>;
  }

  // ==================== PLAYGROUND API METHODS ====================

  /**
   * List available endpoints for playground
   */
  async listPlaygroundEndpoints(workspaceId?: string): Promise<{
    protocol: string;
    method: string;
    path: string;
    description?: string;
    enabled: boolean;
  }[]> {
    const url = workspaceId
      ? `/?workspace_id=${encodeURIComponent(workspaceId)}`
      : '';
    return fetchJsonUnauthenticated(`/__mockforge/playground/endpoints${url}`) as Promise<{
      protocol: string;
      method: string;
      path: string;
      description?: string;
      enabled: boolean;
    }[]>;
  }

  /**
   * Execute a REST request
   */
  async executeRestRequest(request: {
    method: string;
    path: string;
    headers?: Record<string, string>;
    body?: unknown;
    base_url?: string;
    use_mockai?: boolean;
    workspace_id?: string;
  }): Promise<{
    status_code: number;
    headers: Record<string, string>;
    body: unknown;
    response_time_ms: number;
    request_id: string;
    error?: string;
  }> {
    return fetchJsonUnauthenticated('/__mockforge/playground/execute', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as Promise<{
      status_code: number;
      headers: Record<string, string>;
      body: unknown;
      response_time_ms: number;
      request_id: string;
      error?: string;
    }>;
  }

  /**
   * Execute a GraphQL query
   */
  async executeGraphQLQuery(request: {
    query: string;
    variables?: Record<string, unknown>;
    operation_name?: string;
    base_url?: string;
    workspace_id?: string;
  }): Promise<{
    status_code: number;
    headers: Record<string, string>;
    body: unknown;
    response_time_ms: number;
    request_id: string;
    error?: string;
  }> {
    return fetchJsonUnauthenticated('/__mockforge/playground/graphql', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as Promise<{
      status_code: number;
      headers: Record<string, string>;
      body: unknown;
      response_time_ms: number;
      request_id: string;
      error?: string;
    }>;
  }

  /**
   * Perform GraphQL introspection
   */
  async graphQLIntrospect(): Promise<{
    schema: unknown;
    query_types: string[];
    mutation_types: string[];
    subscription_types: string[];
  }> {
    return fetchJsonUnauthenticated('/__mockforge/playground/graphql/introspect') as Promise<{
      schema: unknown;
      query_types: string[];
      mutation_types: string[];
      subscription_types: string[];
    }>;
  }

  /**
   * Get request history
   */
  async getPlaygroundHistory(params?: {
    limit?: number;
    protocol?: string;
    workspace_id?: string;
  }): Promise<{
    id: string;
    protocol: string;
    method: string;
    path: string;
    status_code: number;
    response_time_ms: number;
    timestamp: string;
    request_headers?: Record<string, string>;
    request_body?: unknown;
    graphql_query?: string;
    graphql_variables?: Record<string, unknown>;
  }[]> {
    const queryParams = new URLSearchParams();
    if (params?.limit) {
      queryParams.append('limit', params.limit.toString());
    }
    if (params?.protocol) {
      queryParams.append('protocol', params.protocol);
    }
    if (params?.workspace_id) {
      queryParams.append('workspace_id', params.workspace_id);
    }
    const url = queryParams.toString()
      ? `/__mockforge/playground/history?${queryParams.toString()}`
      : '/__mockforge/playground/history';
    return fetchJsonUnauthenticated(url) as Promise<{
      id: string;
      protocol: string;
      method: string;
      path: string;
      status_code: number;
      response_time_ms: number;
      timestamp: string;
      request_headers?: Record<string, string>;
      request_body?: unknown;
      graphql_query?: string;
      graphql_variables?: Record<string, unknown>;
    }[]>;
  }

  /**
   * Replay a request from history
   */
  async replayRequest(requestId: string): Promise<{
    status_code: number;
    headers: Record<string, string>;
    body: unknown;
    response_time_ms: number;
    request_id: string;
    error?: string;
  }> {
    return fetchJsonUnauthenticated(`/__mockforge/playground/history/${requestId}/replay`, {
      method: 'POST',
    }) as Promise<{
      status_code: number;
      headers: Record<string, string>;
      body: unknown;
      response_time_ms: number;
      request_id: string;
      error?: string;
    }>;
  }

  /**
   * Generate code snippets
   */
  async generateCodeSnippet(request: {
    protocol: string;
    method?: string;
    path: string;
    headers?: Record<string, string>;
    body?: unknown;
    graphql_query?: string;
    graphql_variables?: Record<string, unknown>;
    base_url: string;
  }): Promise<{
    snippets: Record<string, string>;
  }> {
    return fetchJsonUnauthenticated('/__mockforge/playground/snippets', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as Promise<{
      snippets: Record<string, string>;
    }>;
  }

  // ==================== BEHAVIORAL CLONING API METHODS ====================

  /**
   * List all recorded flows
   */
  async getFlows(params?: { limit?: number; db_path?: string }): Promise<{
    flows: Flow[];
    total: number;
  }> {
    const queryParams = new URLSearchParams();
    if (params?.limit) {
      queryParams.append('limit', params.limit.toString());
    }
    if (params?.db_path) {
      queryParams.append('db_path', params.db_path);
    }
    const url = queryParams.toString()
      ? `/__mockforge/flows?${queryParams.toString()}`
      : '/__mockforge/flows';
    const response = await fetchJsonUnauthenticated(url) as {
      success: boolean;
      data: { flows: Flow[]; total: number } | null;
      error: string | null;
    };
    if (!response.success || !response.data) {
      throw new Error(response.error || 'Failed to fetch flows');
    }
    return response.data;
  }

  /**
   * Get flow details with timeline
   */
  async getFlow(flowId: string, params?: { db_path?: string }): Promise<Flow> {
    const queryParams = new URLSearchParams();
    if (params?.db_path) {
      queryParams.append('db_path', params.db_path);
    }
    const url = queryParams.toString()
      ? `/__mockforge/flows/${flowId}?${queryParams.toString()}`
      : `/__mockforge/flows/${flowId}`;
    const response = await fetchJsonUnauthenticated(url) as {
      success: boolean;
      data: Flow | null;
      error: string | null;
    };
    if (!response.success || !response.data) {
      throw new Error(response.error || 'Failed to fetch flow');
    }
    return response.data;
  }

  /**
   * Tag a flow
   */
  async tagFlow(
    flowId: string,
    request: TagFlowRequest,
    params?: { db_path?: string }
  ): Promise<{ message: string; flow_id: string }> {
    const queryParams = new URLSearchParams();
    if (params?.db_path) {
      queryParams.append('db_path', params.db_path);
    }
    const url = queryParams.toString()
      ? `/__mockforge/flows/${flowId}/tag?${queryParams.toString()}`
      : `/__mockforge/flows/${flowId}/tag`;
    const response = await fetchJsonUnauthenticated(url, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as {
      success: boolean;
      data: { message: string; flow_id: string } | null;
      error: string | null;
    };
    if (!response.success || !response.data) {
      throw new Error(response.error || 'Failed to tag flow');
    }
    return response.data;
  }

  /**
   * Compile flow to scenario
   */
  async compileFlow(
    flowId: string,
    request: CompileFlowRequest,
    params?: { db_path?: string }
  ): Promise<CompileFlowResponse> {
    const queryParams = new URLSearchParams();
    if (params?.db_path) {
      queryParams.append('db_path', params.db_path);
    }
    const url = queryParams.toString()
      ? `/__mockforge/flows/${flowId}/compile?${queryParams.toString()}`
      : `/__mockforge/flows/${flowId}/compile`;
    const response = await fetchJsonUnauthenticated(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as {
      success: boolean;
      data: CompileFlowResponse | null;
      error: string | null;
    };
    if (!response.success || !response.data) {
      throw new Error(response.error || 'Failed to compile flow');
    }
    return response.data;
  }

  /**
   * List all scenarios
   */
  async getScenarios(params?: { limit?: number; db_path?: string }): Promise<{
    scenarios: Scenario[];
    total: number;
  }> {
    const queryParams = new URLSearchParams();
    if (params?.limit) {
      queryParams.append('limit', params.limit.toString());
    }
    if (params?.db_path) {
      queryParams.append('db_path', params.db_path);
    }
    const url = queryParams.toString()
      ? `/__mockforge/scenarios?${queryParams.toString()}`
      : '/__mockforge/scenarios';
    const response = await fetchJsonUnauthenticated(url) as {
      success: boolean;
      data: { scenarios: Scenario[]; total: number } | null;
      error: string | null;
    };
    if (!response.success || !response.data) {
      throw new Error(response.error || 'Failed to fetch scenarios');
    }
    return response.data;
  }

  /**
   * Get scenario details
   */
  async getScenario(scenarioId: string, params?: { db_path?: string }): Promise<ScenarioDetail> {
    const queryParams = new URLSearchParams();
    if (params?.db_path) {
      queryParams.append('db_path', params.db_path);
    }
    const url = queryParams.toString()
      ? `/__mockforge/scenarios/${scenarioId}?${queryParams.toString()}`
      : `/__mockforge/scenarios/${scenarioId}`;
    const response = await fetchJsonUnauthenticated(url) as {
      success: boolean;
      data: ScenarioDetail | null;
      error: string | null;
    };
    if (!response.success || !response.data) {
      throw new Error(response.error || 'Failed to fetch scenario');
    }
    return response.data;
  }

  /**
   * Export scenario
   */
  async exportScenario(
    scenarioId: string,
    format: 'yaml' | 'json' = 'yaml',
    params?: { db_path?: string }
  ): Promise<string> {
    const queryParams = new URLSearchParams();
    queryParams.append('format', format);
    if (params?.db_path) {
      queryParams.append('db_path', params.db_path);
    }
    const url = `/__mockforge/scenarios/${scenarioId}/export?${queryParams.toString()}`;
    const response = await fetchJsonUnauthenticated(url) as {
      success: boolean;
      data: { content: string } | null;
      error: string | null;
    };
    if (!response.success || !response.data) {
      throw new Error(response.error || 'Failed to export scenario');
    }
    return response.data.content;
  }
}

export { ProxyApiService };
