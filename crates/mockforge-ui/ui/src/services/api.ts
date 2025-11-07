import type {
  ChainListResponse,
  ChainDefinition,
  ChainCreationResponse,
  ChainExecutionResponse,
  ChainValidationResponse,
  ImportToWorkspaceRequest,
  ImportResponse,
  RequestHistoryResponse,
  EnvironmentListResponse,
  EnvironmentVariablesResponse,
  CreateEnvironmentRequest,
  CreateEnvironmentResponse,
  UpdateEnvironmentRequest,
  SetVariableRequest,
  AutocompleteRequest,
  AutocompleteResponse,
  SyncStatus,
  ConfigureSyncRequest,
  SyncChange,
  ConfirmSyncChangesRequest,
  ImportRequest,
  ImportHistoryResponse,
  RequestLog,
  MetricsData,
  ValidationSettings,
  LatencyProfile,
  FaultConfig,
  ProxyConfig,
  DashboardData,
  WorkspaceResponse,
  CreateWorkspaceRequest,
  CreateWorkspaceResponse,
  FolderResponse,
  CreateFolderRequest,
  CreateFolderResponse,
  CreateRequestRequest,
  CreateRequestResponse,
  ExecuteRequestRequest,
  ExecuteRequestResponse,
  HealthCheck,
  RestartStatus,
  ServerConfiguration,
  SmokeTestResult,
  SmokeTestContext,
  RouteInfo,
  ServerInfo,
  FileContentRequest,
  FileContentResponse,
  SaveFileRequest,
  GraphData,
  EncryptionStatus,
  AutoEncryptionConfig,
  SecurityCheckResult,
  FixtureInfo,
  PluginListResponse,
  VerificationRequest,
  VerificationCount,
  VerificationResult
} from '../types';

import {
  WorkspaceListResponseSchema,
  LogsResponseSchema,
  DashboardResponseSchema,
  FixturesResponseSchema,
  safeValidateApiResponse,
  type WorkspaceSummary,
} from '../schemas/api';
import { logger } from '@/utils/logger';

// Admin API type definitions
export type { RequestLog, MetricsData, ValidationSettings, LatencyProfile, FaultConfig, ProxyConfig, DashboardData } from '../types';
export type { HealthCheck, RestartStatus, SmokeTestResult, SmokeTestContext } from '../types';
export type { ImportRequest, ImportResponse, ImportHistoryResponse, ImportHistoryEntry } from '../types';

// FixtureInfo moved to types/index.ts - import from there
export type { FixtureInfo } from '../types';

// Workspace API types
export type { WorkspaceListResponse, WorkspaceResponse, CreateWorkspaceRequest, CreateWorkspaceResponse } from '../types';
export type { FolderResponse, CreateFolderRequest, CreateFolderResponse } from '../types';
export type { CreateRequestRequest, CreateRequestResponse, ExecuteRequestRequest, ExecuteRequestResponse } from '../types';

const API_BASE = '/__mockforge/chains';
const WORKSPACE_API_BASE = '/__mockforge/workspaces';

class ApiService {
  private async fetchJson(url: string, options?: RequestInit): Promise<unknown> {
    const response = await fetch(url, options);
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    const json = await response.json();
    return json.data || json;
  }

  private async fetchJsonWithValidation<T>(
    url: string,
    schema: Parameters<typeof safeValidateApiResponse>[0],
    options?: RequestInit
  ): Promise<T> {
    const data = await this.fetchJson(url, options);
    const result = safeValidateApiResponse(schema, data);

    if (!result.success) {
      if (import.meta.env.DEV) {
        logger.error('API validation error', result.error.format());
      }
      throw new Error(`API response validation failed: ${result.error.message}`);
    }

    return result.data as T;
  }

  async listChains(): Promise<ChainListResponse> {
    return this.fetchJson(API_BASE) as Promise<ChainListResponse>;
  }

  async getChain(chainId: string): Promise<ChainDefinition> {
    return this.fetchJson(`${API_BASE}/${chainId}`) as Promise<ChainDefinition>;
  }

  async getGraph(): Promise<GraphData> {
    const response = await this.fetchJson('/__mockforge/graph') as { data: GraphData; success: boolean };
    // Handle ApiResponse wrapper
    if (response.success && response.data) {
      return response.data;
    }
    // Fallback: assume response is GraphData directly
    return response as unknown as GraphData;
  }

  // State Machine API methods
  async getStateMachines(): Promise<{ state_machines: Array<{ resource_type: string; state_count: number; transition_count: number; sub_scenario_count: number; has_visual_layout: boolean }>; total: number }> {
    return this.fetchJson('/__mockforge/api/state-machines') as Promise<{ state_machines: Array<{ resource_type: string; state_count: number; transition_count: number; sub_scenario_count: number; has_visual_layout: boolean }>; total: number }>;
  }

  async getStateMachine(resourceType: string): Promise<{ state_machine: unknown; visual_layout?: unknown }> {
    return this.fetchJson(`/__mockforge/api/state-machines/${encodeURIComponent(resourceType)}`) as Promise<{ state_machine: unknown; visual_layout?: unknown }>;
  }

  async createStateMachine(stateMachine: unknown, visualLayout?: unknown): Promise<{ state_machine: unknown; visual_layout?: unknown }> {
    return this.fetchJson('/__mockforge/api/state-machines', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ state_machine: stateMachine, visual_layout: visualLayout }),
    }) as Promise<{ state_machine: unknown; visual_layout?: unknown }>;
  }

  async updateStateMachine(resourceType: string, stateMachine: unknown, visualLayout?: unknown): Promise<{ state_machine: unknown; visual_layout?: unknown }> {
    return this.fetchJson(`/__mockforge/api/state-machines/${encodeURIComponent(resourceType)}`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ state_machine: stateMachine, visual_layout: visualLayout }),
    }) as Promise<{ state_machine: unknown; visual_layout?: unknown }>;
  }

  async deleteStateMachine(resourceType: string): Promise<void> {
    await this.fetchJson(`/__mockforge/api/state-machines/${encodeURIComponent(resourceType)}`, {
      method: 'DELETE',
    });
  }

  async getStateInstances(): Promise<{ instances: Array<{ resource_id: string; current_state: string; resource_type: string; history_count: number; state_data: Record<string, unknown> }>; total: number }> {
    return this.fetchJson('/__mockforge/api/state-machines/instances') as Promise<{ instances: Array<{ resource_id: string; current_state: string; resource_type: string; history_count: number; state_data: Record<string, unknown> }>; total: number }>;
  }

  async getStateInstance(resourceId: string): Promise<{ resource_id: string; current_state: string; resource_type: string; history_count: number; state_data: Record<string, unknown> }> {
    return this.fetchJson(`/__mockforge/api/state-machines/instances/${encodeURIComponent(resourceId)}`) as Promise<{ resource_id: string; current_state: string; resource_type: string; history_count: number; state_data: Record<string, unknown> }>;
  }

  async createStateInstance(resourceId: string, resourceType: string): Promise<{ resource_id: string; current_state: string; resource_type: string; history_count: number; state_data: Record<string, unknown> }> {
    return this.fetchJson('/__mockforge/api/state-machines/instances', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ resource_id: resourceId, resource_type: resourceType }),
    }) as Promise<{ resource_id: string; current_state: string; resource_type: string; history_count: number; state_data: Record<string, unknown> }>;
  }

  async executeTransition(resourceId: string, toState: string, context?: Record<string, unknown>): Promise<{ resource_id: string; current_state: string; resource_type: string; history_count: number; state_data: Record<string, unknown> }> {
    return this.fetchJson(`/__mockforge/api/state-machines/instances/${encodeURIComponent(resourceId)}/transition`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ resource_id: resourceId, to_state: toState, context }),
    }) as Promise<{ resource_id: string; current_state: string; resource_type: string; history_count: number; state_data: Record<string, unknown> }>;
  }

  async getNextStates(resourceId: string): Promise<{ next_states: string[] }> {
    return this.fetchJson(`/__mockforge/api/state-machines/instances/${encodeURIComponent(resourceId)}/next-states`) as Promise<{ next_states: string[] }>;
  }

  async getCurrentState(resourceId: string): Promise<{ resource_id: string; current_state: string }> {
    return this.fetchJson(`/__mockforge/api/state-machines/instances/${encodeURIComponent(resourceId)}/state`) as Promise<{ resource_id: string; current_state: string }>;
  }

  async exportStateMachines(): Promise<{ state_machines: unknown[]; visual_layouts: Record<string, unknown> }> {
    return this.fetchJson('/__mockforge/api/state-machines/export') as Promise<{ state_machines: unknown[]; visual_layouts: Record<string, unknown> }>;
  }

  async importStateMachines(data: { state_machines: unknown[]; visual_layouts: Record<string, unknown> }): Promise<void> {
    await this.fetchJson('/__mockforge/api/state-machines/import', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
  }

  async createChain(definition: string): Promise<ChainCreationResponse> {
    return this.fetchJson(API_BASE, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ definition }),
    }) as Promise<ChainCreationResponse>;
  }

  async updateChain(chainId: string, definition: string): Promise<ChainCreationResponse> {
    return this.fetchJson(`${API_BASE}/${chainId}`, {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ definition }),
    }) as Promise<ChainCreationResponse>;
  }

  async deleteChain(chainId: string): Promise<{ message: string }> {
    return this.fetchJson(`${API_BASE}/${chainId}`, {
      method: 'DELETE',
    }) as Promise<{ message: string }>;
  }

  async executeChain(chainId: string, variables?: unknown): Promise<ChainExecutionResponse> {
    return this.fetchJson(`${API_BASE}/${chainId}/execute`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ variables: variables || {} }),
    }) as Promise<ChainExecutionResponse>;
  }

  async validateChain(chainId: string): Promise<ChainValidationResponse> {
    return this.fetchJson(`${API_BASE}/${chainId}/validate`, {
      method: 'POST',
    }) as Promise<ChainValidationResponse>;
  }

  // ==================== WORKSPACE API METHODS ====================

  async listWorkspaces(): Promise<WorkspaceSummary[]> {
    return this.fetchJsonWithValidation<WorkspaceSummary[]>(
      WORKSPACE_API_BASE,
      WorkspaceListResponseSchema
    );
  }

  async getWorkspace(workspaceId: string): Promise<WorkspaceResponse> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}`) as Promise<WorkspaceResponse>;
  }

  async createWorkspace(request: CreateWorkspaceRequest): Promise<CreateWorkspaceResponse> {
    return this.fetchJson(WORKSPACE_API_BASE, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(request),
    }) as Promise<CreateWorkspaceResponse>;
  }

  async openWorkspaceFromDirectory(directory: string): Promise<CreateWorkspaceResponse> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/open-from-directory`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ directory }),
    }) as Promise<CreateWorkspaceResponse>;
  }

  async deleteWorkspace(workspaceId: string): Promise<{ message: string }> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}`, {
      method: 'DELETE',
    }) as Promise<{ message: string }>;
  }

  async setActiveWorkspace(workspaceId: string): Promise<{ message: string }> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/activate`, {
      method: 'POST',
    }) as Promise<{ message: string }>;
  }

  async getFolder(workspaceId: string, folderId: string): Promise<FolderResponse> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/folders/${folderId}`) as Promise<FolderResponse>;
  }

  async createFolder(workspaceId: string, request: CreateFolderRequest): Promise<CreateFolderResponse> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/folders`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(request),
    }) as Promise<CreateFolderResponse>;
  }

  async createRequest(workspaceId: string, request: CreateRequestRequest): Promise<CreateRequestResponse> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/requests`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(request),
    }) as Promise<CreateRequestResponse>;
  }

  async importToWorkspace(workspaceId: string, request: ImportToWorkspaceRequest): Promise<ImportResponse> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/import`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(request),
    }) as Promise<ImportResponse>;
  }

  async previewImport(request: ImportToWorkspaceRequest): Promise<ImportResponse> {
    return this.fetchJson('/__mockforge/import/preview', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(request),
    }) as Promise<ImportResponse>;
  }

  async executeRequest(workspaceId: string, requestId: string, executionRequest?: ExecuteRequestRequest): Promise<ExecuteRequestResponse> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/requests/${requestId}/execute`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(executionRequest || {}),
    }) as Promise<ExecuteRequestResponse>;
  }

  async getRequestHistory(workspaceId: string, requestId: string): Promise<RequestHistoryResponse> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/requests/${requestId}/history`) as Promise<RequestHistoryResponse>;
  }

  // ==================== ENVIRONMENT API METHODS ====================

  async getEnvironments(workspaceId: string): Promise<EnvironmentListResponse> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/environments`) as Promise<EnvironmentListResponse>;
  }

  async createEnvironment(workspaceId: string, request: CreateEnvironmentRequest): Promise<CreateEnvironmentResponse> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/environments`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as Promise<CreateEnvironmentResponse>;
  }

  async updateEnvironment(workspaceId: string, environmentId: string, request: UpdateEnvironmentRequest): Promise<{ message: string }> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/environments/${environmentId}`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as Promise<{ message: string }>;
  }

  async deleteEnvironment(workspaceId: string, environmentId: string): Promise<{ message: string }> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/environments/${environmentId}`, {
      method: 'DELETE',
    }) as Promise<{ message: string }>;
  }

  async setActiveEnvironment(workspaceId: string, environmentId: string): Promise<{ message: string }> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/environments/${environmentId}/activate`, {
      method: 'POST',
    }) as Promise<{ message: string }>;
  }

  async getEnvironmentVariables(workspaceId: string, environmentId: string): Promise<EnvironmentVariablesResponse> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/environments/${environmentId}/variables`) as Promise<EnvironmentVariablesResponse>;
  }

  async setEnvironmentVariable(workspaceId: string, environmentId: string, request: SetVariableRequest): Promise<{ message: string }> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/environments/${environmentId}/variables`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as Promise<{ message: string }>;
  }

  async removeEnvironmentVariable(workspaceId: string, environmentId: string, variableName: string): Promise<{ message: string }> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/environments/${environmentId}/variables/${encodeURIComponent(variableName)}`, {
      method: 'DELETE',
    }) as Promise<{ message: string }>;
  }

  async getAutocompleteSuggestions(workspaceId: string, request: AutocompleteRequest): Promise<AutocompleteResponse> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/autocomplete`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as Promise<AutocompleteResponse>;
  }

  // ==================== ORDERING API METHODS ====================

  async updateWorkspacesOrder(workspaceIds: string[]): Promise<{ message: string }> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/order`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ workspace_ids: workspaceIds }),
    }) as Promise<{ message: string }>;
  }

  async updateEnvironmentsOrder(workspaceId: string, environmentIds: string[]): Promise<{ message: string }> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/environments/order`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ environment_ids: environmentIds }),
    }) as Promise<{ message: string }>;
  }

  // ==================== SYNC API METHODS ====================

  async getSyncStatus(workspaceId: string): Promise<SyncStatus> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/sync/status`) as Promise<SyncStatus>;
  }

  async configureSync(workspaceId: string, request: ConfigureSyncRequest): Promise<{ message: string }> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/sync/configure`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as Promise<{ message: string }>;
  }

  async disableSync(workspaceId: string): Promise<{ message: string }> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/sync/disable`, {
      method: 'POST',
    }) as Promise<{ message: string }>;
  }

  async triggerSync(workspaceId: string): Promise<{ message: string }> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/sync/trigger`, {
      method: 'POST',
    }) as Promise<{ message: string }>;
  }

  async getSyncChanges(workspaceId: string): Promise<SyncChange[]> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/sync/changes`) as Promise<SyncChange[]>;
  }

  async confirmSyncChanges(workspaceId: string, request: ConfirmSyncChangesRequest): Promise<{ message: string }> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/sync/confirm`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as Promise<{ message: string }>;
  }

  // ==================== ENCRYPTION API METHODS ====================

  async getWorkspaceEncryptionStatus(workspaceId: string): Promise<EncryptionStatus> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/encryption/status`) as Promise<EncryptionStatus>;
  }

  async getWorkspaceEncryptionConfig(workspaceId: string): Promise<AutoEncryptionConfig> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/encryption/config`) as Promise<AutoEncryptionConfig>;
  }

  async enableWorkspaceEncryption(workspaceId: string): Promise<{ message: string }> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/encryption/enable`, {
      method: 'POST',
    }) as Promise<{ message: string }>;
  }

  async disableWorkspaceEncryption(workspaceId: string): Promise<{ message: string }> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/encryption/disable`, {
      method: 'POST',
    }) as Promise<{ message: string }>;
  }

  async checkWorkspaceSecurity(workspaceId: string): Promise<SecurityCheckResult> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/encryption/security-check`, {
      method: 'POST',
    }) as Promise<SecurityCheckResult>;
  }

  async exportWorkspaceEncrypted(workspaceId: string, exportPath: string): Promise<{ message: string }> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/encryption/export`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ export_path: exportPath }),
    }) as Promise<{ message: string }>;
  }

  async importWorkspaceEncrypted(importPath: string, workspaceId: string, backupKey: string): Promise<{ message: string }> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/encryption/import`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ import_path: importPath, backup_key: backupKey }),
    }) as Promise<{ message: string }>;
  }

  async updateWorkspaceEncryptionConfig(workspaceId: string, config: unknown): Promise<{ message: string }> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/encryption/config`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(config),
    }) as Promise<{ message: string }>;
  }
}

class ImportApiService {
  private async fetchJson(url: string, options?: RequestInit): Promise<unknown> {
    const response = await fetch(url, options);
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    const json = await response.json();
    return json.data || json;
  }

  async importPostman(request: ImportRequest): Promise<ImportResponse> {
    return this.fetchJson('/__mockforge/import/postman', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(request),
    }) as Promise<ImportResponse>;
  }

  async importInsomnia(request: ImportRequest): Promise<ImportResponse> {
    return this.fetchJson('/__mockforge/import/insomnia', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(request),
    }) as Promise<ImportResponse>;
  }

  async importCurl(request: ImportRequest): Promise<ImportResponse> {
    return this.fetchJson('/__mockforge/import/curl', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(request),
    }) as Promise<ImportResponse>;
  }

  async importOpenApi(request: ImportRequest): Promise<ImportResponse> {
    return this.fetchJson('/__mockforge/import/openapi', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(request),
    }) as Promise<ImportResponse>;
  }

  async previewImport(request: ImportRequest): Promise<ImportResponse> {
    return this.fetchJson('/__mockforge/import/preview', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(request),
    }) as Promise<ImportResponse>;
  }

  async getImportHistory(): Promise<ImportHistoryResponse> {
    return this.fetchJson('/__mockforge/import/history') as Promise<ImportHistoryResponse>;
  }

  async clearImportHistory(): Promise<void> {
    return this.fetchJson('/__mockforge/import/history/clear', {
      method: 'POST',
    }) as Promise<void>;
  }
}

class FixturesApiService {
  constructor() {
    // Bind all methods to ensure 'this' context is preserved
    this.getFixtures = this.getFixtures.bind(this);
    this.deleteFixture = this.deleteFixture.bind(this);
    this.deleteFixturesBulk = this.deleteFixturesBulk.bind(this);
    this.downloadFixture = this.downloadFixture.bind(this);
    this.renameFixture = this.renameFixture.bind(this);
    this.moveFixture = this.moveFixture.bind(this);
  }

  private async fetchJson(url: string, options?: RequestInit): Promise<unknown> {
    const response = await fetch(url, options);
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    const json = await response.json();
    return json.data || json;
  }

  private async fetchJsonWithValidation<T>(
    url: string,
    schema: Parameters<typeof safeValidateApiResponse>[0],
    options?: RequestInit
  ): Promise<T> {
    const data = await this.fetchJson(url, options);
    const result = safeValidateApiResponse(schema, data);

    if (!result.success) {
      if (import.meta.env.DEV) {
        logger.error('API validation error', result.error.format());
      }
      throw new Error(`API response validation failed: ${result.error.message}`);
    }

    return result.data as T;
  }

  async getFixtures(): Promise<import('../types').FixtureInfo[]> {
    return this.fetchJsonWithValidation<FixtureInfo[]>(
      '/__mockforge/fixtures',
      FixturesResponseSchema
    );
  }

  async deleteFixture(fixtureId: string): Promise<void> {
    return this.fetchJson(`/__mockforge/fixtures/${fixtureId}`, {
      method: 'DELETE',
    }) as Promise<void>;
  }

  async deleteFixturesBulk(fixtureIds: string[]): Promise<void> {
    return this.fetchJson('/__mockforge/fixtures/bulk', {
      method: 'DELETE',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ fixture_ids: fixtureIds }),
    }) as Promise<void>;
  }

  async downloadFixture(fixtureId: string): Promise<Blob> {
    const response = await fetch(`/__mockforge/fixtures/${fixtureId}/download`);
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    return response.blob();
  }

  async renameFixture(fixtureId: string, newName: string): Promise<void> {
    return this.fetchJson(`/__mockforge/fixtures/${fixtureId}/rename`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ new_name: newName }),
    }) as Promise<void>;
  }

  async moveFixture(fixtureId: string, newPath: string): Promise<void> {
    return this.fetchJson(`/__mockforge/fixtures/${fixtureId}/move`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ new_path: newPath }),
    }) as Promise<void>;
  }
}

// ==================== ADMIN API METHODS ====================

class DashboardApiService {
  private async fetchJson(url: string, options?: RequestInit): Promise<unknown> {
    const response = await fetch(url, options);
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    const json = await response.json();
    return json.data || json;
  }

  private async fetchJsonWithValidation<T>(
    url: string,
    schema: Parameters<typeof safeValidateApiResponse>[0],
    options?: RequestInit
  ): Promise<T> {
    const data = await this.fetchJson(url, options);
    const result = safeValidateApiResponse(schema, data);

    if (!result.success) {
      if (import.meta.env.DEV) {
        logger.error('API validation error', result.error.format());
      }
      throw new Error(`API response validation failed: ${result.error.message}`);
    }

    return result.data as T;
  }

  async getDashboard(): Promise<DashboardData> {
    return this.fetchJsonWithValidation<DashboardData>(
      '/__mockforge/dashboard',
      DashboardResponseSchema
    );
  }

  async getHealth(): Promise<HealthCheck> {
    return this.fetchJson('/__mockforge/health') as Promise<HealthCheck>;
  }
}

class ServerApiService {
  private async fetchJson(url: string, options?: RequestInit): Promise<unknown> {
    const response = await fetch(url, options);
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    const json = await response.json();
    return json.data || json;
  }

  async getServerInfo(): Promise<ServerInfo> {
    return this.fetchJson('/__mockforge/server-info') as Promise<ServerInfo>;
  }

  async restartServer(reason?: string): Promise<RestartStatus> {
    return this.fetchJson('/__mockforge/servers/restart', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ reason: reason || 'Manual restart' }),
    }) as Promise<RestartStatus>;
  }

  async getRestartStatus(): Promise<RestartStatus> {
    return this.fetchJson('/__mockforge/servers/restart/status') as Promise<RestartStatus>;
  }
}

class RoutesApiService {
  private async fetchJson(url: string, options?: RequestInit): Promise<unknown> {
    const response = await fetch(url, options);
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    const json = await response.json();
    return json.data || json;
  }

  async getRoutes(): Promise<RouteInfo[]> {
    return this.fetchJson('/__mockforge/routes') as Promise<RouteInfo[]>;
  }
}

class LogsApiService {
  constructor() {
    this.getLogs = this.getLogs.bind(this);
    this.clearLogs = this.clearLogs.bind(this);
  }

  private async fetchJson(url: string, options?: RequestInit): Promise<unknown> {
    const response = await fetch(url, options);
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    const json = await response.json();
    return json.data || json;
  }

  private async fetchJsonWithValidation<T>(
    url: string,
    schema: Parameters<typeof safeValidateApiResponse>[0],
    options?: RequestInit
  ): Promise<T> {
    const data = await this.fetchJson(url, options);
    const result = safeValidateApiResponse(schema, data);

    if (!result.success) {
      if (import.meta.env.DEV) {
        logger.error('API validation error', result.error.format());
      }
      throw new Error(`API response validation failed: ${result.error.message}`);
    }

    return result.data as T;
  }

  async getLogs(params?: Record<string, string | number>): Promise<RequestLog[]> {
    let url = '/__mockforge/logs';

    if (params && Object.keys(params).length > 0) {
      // Convert all values to strings for URLSearchParams
      const stringParams: Record<string, string> = {};
      for (const [key, value] of Object.entries(params)) {
        if (value !== undefined && value !== null) {
          stringParams[key] = String(value);
        }
      }
      if (Object.keys(stringParams).length > 0) {
        const queryString = '?' + new URLSearchParams(stringParams).toString();
        url = `/__mockforge/logs${queryString}`;
      }
    }

    return this.fetchJsonWithValidation<RequestLog[]>(url, LogsResponseSchema);
  }

  async clearLogs(): Promise<{ message: string }> {
    return this.fetchJson('/__mockforge/logs', {
      method: 'DELETE',
    }) as Promise<{ message: string }>;
  }
}

class MetricsApiService {
  constructor() {
    this.getMetrics = this.getMetrics.bind(this);
  }

  private async fetchJson(url: string, options?: RequestInit): Promise<unknown> {
    const response = await fetch(url, options);
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    const json = await response.json();
    return json.data || json;
  }

  async getMetrics(): Promise<MetricsData> {
    return this.fetchJson('/__mockforge/metrics') as Promise<MetricsData>;
  }
}

class ConfigApiService {
  private async fetchJson(url: string, options?: RequestInit): Promise<unknown> {
    const response = await fetch(url, options);
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    const json = await response.json();
    return json.data || json;
  }

  async getConfig(): Promise<ServerConfiguration> {
    return this.fetchJson('/__mockforge/config') as Promise<ServerConfiguration>;
  }

  async updateLatency(config: LatencyProfile): Promise<{ message: string }> {
    return this.fetchJson('/__mockforge/config/latency', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(config),
    }) as Promise<{ message: string }>;
  }

  async updateFaults(config: FaultConfig): Promise<{ message: string }> {
    return this.fetchJson('/__mockforge/config/faults', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(config),
    }) as Promise<{ message: string }>;
  }

  async updateProxy(config: ProxyConfig): Promise<{ message: string }> {
    return this.fetchJson('/__mockforge/config/proxy', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(config),
    }) as Promise<{ message: string }>;
  }
}

class ValidationApiService {
  private async fetchJson(url: string, options?: RequestInit): Promise<unknown> {
    const response = await fetch(url, options);
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    const json = await response.json();
    return json.data || json;
  }

  async getValidation(): Promise<ValidationSettings> {
    return this.fetchJson('/__mockforge/validation') as Promise<ValidationSettings>;
  }

  async updateValidation(config: ValidationSettings): Promise<{ message: string }> {
    return this.fetchJson('/__mockforge/validation', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(config),
    }) as Promise<{ message: string }>;
  }
}

class EnvApiService {
  private async fetchJson(url: string, options?: RequestInit): Promise<unknown> {
    const response = await fetch(url, options);
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    const json = await response.json();
    return json.data || json;
  }

  async getEnvVars(): Promise<Record<string, string>> {
    return this.fetchJson('/__mockforge/env') as Promise<Record<string, string>>;
  }

  async updateEnvVar(key: string, value: string): Promise<{ message: string }> {
    return this.fetchJson('/__mockforge/env', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ key, value }),
    }) as Promise<{ message: string }>;
  }
}

class FilesApiService {
  private async fetchJson(url: string, options?: RequestInit): Promise<unknown> {
    const response = await fetch(url, options);
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    const json = await response.json();
    return json.data || json;
  }

  async getFileContent(request: FileContentRequest): Promise<FileContentResponse> {
    return this.fetchJson('/__mockforge/files/content', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as Promise<FileContentResponse>;
  }

  async saveFileContent(request: SaveFileRequest): Promise<{ message: string }> {
    return this.fetchJson('/__mockforge/files/save', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as Promise<{ message: string }>;
  }
}

class SmokeTestsApiService {
  private async fetchJson(url: string, options?: RequestInit): Promise<unknown> {
    const response = await fetch(url, options);
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    const json = await response.json();
    return json.data || json;
  }

  async getSmokeTests(): Promise<SmokeTestResult[]> {
    return this.fetchJson('/__mockforge/smoke') as Promise<SmokeTestResult[]>;
  }

  async runSmokeTests(): Promise<SmokeTestContext> {
    return this.fetchJson('/__mockforge/smoke/run', {
      method: 'GET',
    }) as Promise<SmokeTestContext>;
  }
}

class ChaosApiService {
  private async fetchJson(url: string, options?: RequestInit): Promise<unknown> {
    const response = await fetch(url, options);
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    const json = await response.json();
    return json.data || json;
  }

  /**
   * Get current chaos configuration
   */
  async getChaosConfig(): Promise<any> {
    return this.fetchJson('/api/chaos/config') as Promise<any>;
  }

  /**
   * Get current chaos status
   */
  async getChaosStatus(): Promise<any> {
    return this.fetchJson('/api/chaos/status') as Promise<any>;
  }

  /**
   * Update latency configuration
   */
  async updateChaosLatency(config: any): Promise<{ message: string }> {
    return this.fetchJson('/api/chaos/config/latency', {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(config),
    }) as Promise<{ message: string }>;
  }

  /**
   * Update fault injection configuration
   */
  async updateChaosFaults(config: any): Promise<{ message: string }> {
    return this.fetchJson('/api/chaos/config/faults', {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(config),
    }) as Promise<{ message: string }>;
  }

  /**
   * Update traffic shaping configuration
   */
  async updateChaosTraffic(config: any): Promise<{ message: string }> {
    return this.fetchJson('/api/chaos/config/traffic', {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(config),
    }) as Promise<{ message: string }>;
  }

  /**
   * Enable chaos engineering
   */
  async enableChaos(): Promise<{ message: string }> {
    return this.fetchJson('/api/chaos/enable', {
      method: 'POST',
    }) as Promise<{ message: string }>;
  }

  /**
   * Disable chaos engineering
   */
  async disableChaos(): Promise<{ message: string }> {
    return this.fetchJson('/api/chaos/disable', {
      method: 'POST',
    }) as Promise<{ message: string }>;
  }

  /**
   * Reset chaos configuration to defaults
   */
  async resetChaos(): Promise<{ message: string }> {
    return this.fetchJson('/api/chaos/reset', {
      method: 'POST',
    }) as Promise<{ message: string }>;
  }
}

/**
 * Time Travel API Service
 * Handles all time travel and temporal simulation operations
 */
class TimeTravelApiService {
  private async fetchJson(url: string, options?: RequestInit): Promise<unknown> {
    const response = await fetch(url, options);
    if (!response.ok) {
      const error = await response.json().catch(() => ({ error: `HTTP ${response.status}` }));
      throw new Error(error.error || `HTTP error! status: ${response.status}`);
    }
    const json = await response.json();
    return json.data || json;
  }

  // Time Travel Status
  async getStatus(): Promise<{
    enabled: boolean;
    current_time?: string;
    scale_factor: number;
    real_time: string;
  }> {
    return this.fetchJson('/__mockforge/time-travel/status') as Promise<{
      enabled: boolean;
      current_time?: string;
      scale_factor: number;
      real_time: string;
    }>;
  }

  async enable(time?: string, scale?: number): Promise<{
    success: boolean;
    status: {
      enabled: boolean;
      current_time?: string;
      scale_factor: number;
    };
  }> {
    return this.fetchJson('/__mockforge/time-travel/enable', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ time, scale }),
    }) as Promise<{
      success: boolean;
      status: {
        enabled: boolean;
        current_time?: string;
        scale_factor: number;
      };
    }>;
  }

  async disable(): Promise<{ success: boolean }> {
    return this.fetchJson('/__mockforge/time-travel/disable', {
      method: 'POST',
    }) as Promise<{ success: boolean }>;
  }

  async advance(duration: string): Promise<{
    success: boolean;
    status: {
      enabled: boolean;
      current_time?: string;
      scale_factor: number;
    };
  }> {
    return this.fetchJson('/__mockforge/time-travel/advance', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ duration }),
    }) as Promise<{
      success: boolean;
      status: {
        enabled: boolean;
        current_time?: string;
        scale_factor: number;
      };
    }>;
  }

  async setScale(scale: number): Promise<{ success: boolean }> {
    return this.fetchJson('/__mockforge/time-travel/scale', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ scale }),
    }) as Promise<{ success: boolean }>;
  }

  async reset(): Promise<{ success: boolean }> {
    return this.fetchJson('/__mockforge/time-travel/reset', {
      method: 'POST',
    }) as Promise<{ success: boolean }>;
  }

  // Cron Jobs
  async listCronJobs(): Promise<{ success: boolean; jobs: unknown[] }> {
    return this.fetchJson('/__mockforge/time-travel/cron') as Promise<{
      success: boolean;
      jobs: unknown[];
    }>;
  }

  async getCronJob(id: string): Promise<{ success: boolean; job: unknown }> {
    return this.fetchJson(`/__mockforge/time-travel/cron/${id}`) as Promise<{
      success: boolean;
      job: unknown;
    }>;
  }

  async createCronJob(job: {
    id: string;
    name: string;
    schedule: string;
    description?: string;
    action_type: string;
    action_metadata: unknown;
  }): Promise<{ success: boolean; message: string }> {
    return this.fetchJson('/__mockforge/time-travel/cron', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(job),
    }) as Promise<{ success: boolean; message: string }>;
  }

  async deleteCronJob(id: string): Promise<{ success: boolean; message: string }> {
    return this.fetchJson(`/__mockforge/time-travel/cron/${id}`, {
      method: 'DELETE',
    }) as Promise<{ success: boolean; message: string }>;
  }

  async setCronJobEnabled(id: string, enabled: boolean): Promise<{
    success: boolean;
    message: string;
  }> {
    return this.fetchJson(`/__mockforge/time-travel/cron/${id}/enable`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ enabled }),
    }) as Promise<{ success: boolean; message: string }>;
  }

  // Mutation Rules
  async listMutationRules(): Promise<{ success: boolean; rules: unknown[] }> {
    return this.fetchJson('/__mockforge/time-travel/mutations') as Promise<{
      success: boolean;
      rules: unknown[];
    }>;
  }

  async getMutationRule(id: string): Promise<{ success: boolean; rule: unknown }> {
    return this.fetchJson(`/__mockforge/time-travel/mutations/${id}`) as Promise<{
      success: boolean;
      rule: unknown;
    }>;
  }

  async createMutationRule(rule: {
    id: string;
    entity_name: string;
    trigger: unknown;
    operation: unknown;
    description?: string;
    condition?: string;
  }): Promise<{ success: boolean; message: string }> {
    return this.fetchJson('/__mockforge/time-travel/mutations', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(rule),
    }) as Promise<{ success: boolean; message: string }>;
  }

  async deleteMutationRule(id: string): Promise<{ success: boolean; message: string }> {
    return this.fetchJson(`/__mockforge/time-travel/mutations/${id}`, {
      method: 'DELETE',
    }) as Promise<{ success: boolean; message: string }>;
  }

  async setMutationRuleEnabled(id: string, enabled: boolean): Promise<{
    success: boolean;
    message: string;
  }> {
    return this.fetchJson(`/__mockforge/time-travel/mutations/${id}/enable`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ enabled }),
    }) as Promise<{ success: boolean; message: string }>;
  }
}

class PluginsApiService {
  private async fetchJson(url: string, options?: RequestInit): Promise<unknown> {
    const response = await fetch(url, options);
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    const json = await response.json();
    return json.data || json;
  }

  async getPlugins(params?: { type?: string; status?: string }): Promise<PluginListResponse> {
    const queryParams = new URLSearchParams();
    if (params?.type) queryParams.append('type', params.type);
    if (params?.status) queryParams.append('status', params.status);

    const queryString = queryParams.toString() ? `?${queryParams.toString()}` : '';
    return this.fetchJson(`/__mockforge/plugins${queryString}`) as Promise<PluginListResponse>;
  }

  async getPluginStatus(): Promise<unknown> {
    return this.fetchJson('/__mockforge/plugins/status');
  }

  async getPluginDetails(pluginId: string): Promise<unknown> {
    return this.fetchJson(`/__mockforge/plugins/${pluginId}`);
  }

  async deletePlugin(pluginId: string): Promise<{ message: string }> {
    return this.fetchJson(`/__mockforge/plugins/${pluginId}`, {
      method: 'DELETE',
    }) as Promise<{ message: string }>;
  }

  async reloadPlugin(pluginId: string): Promise<{ message: string; status: string }> {
    return this.fetchJson('/__mockforge/plugins/reload', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ plugin_id: pluginId }),
    }) as Promise<{ message: string; status: string }>;
  }

  async reloadAllPlugins(): Promise<{ message: string }> {
    // Get all plugins first
    const { plugins } = await this.getPlugins() as { plugins: Array<{ id: string }> };

    // Reload each plugin
    const results = await Promise.allSettled(
      plugins.map(plugin => this.reloadPlugin(plugin.id))
    );

    const failed = results.filter(r => r.status === 'rejected').length;

    if (failed > 0) {
      throw new Error(`Failed to reload ${failed} plugin(s)`);
    }

    return { message: `Successfully reloaded ${plugins.length} plugin(s)` };
  }
}

class VerificationApiService {
  private async fetchJson(url: string, options?: RequestInit): Promise<unknown> {
    const response = await fetch(url, options);
    if (!response.ok) {
      const errorData = await response.json().catch(() => ({}));
      throw new Error(errorData.message || `HTTP error! status: ${response.status}`);
    }
    const json = await response.json();
    return json.data || json;
  }

  async verify(pattern: VerificationRequest, expected: VerificationCount): Promise<VerificationResult> {
    return this.fetchJson('/__mockforge/verification/verify', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ pattern, expected }),
    }) as Promise<VerificationResult>;
  }

  async count(pattern: VerificationRequest): Promise<{ count: number }> {
    return this.fetchJson('/__mockforge/verification/count', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ pattern }),
    }) as Promise<{ count: number }>;
  }

  async verifySequence(patterns: VerificationRequest[]): Promise<VerificationResult> {
    return this.fetchJson('/__mockforge/verification/sequence', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ patterns }),
    }) as Promise<VerificationResult>;
  }

  async verifyNever(pattern: VerificationRequest): Promise<VerificationResult> {
    return this.fetchJson('/__mockforge/verification/never', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(pattern),
    }) as Promise<VerificationResult>;
  }

  async verifyAtLeast(pattern: VerificationRequest, min: number): Promise<VerificationResult> {
    return this.fetchJson('/__mockforge/verification/at-least', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ pattern, min }),
    }) as Promise<VerificationResult>;
  }
}

export const apiService = new ApiService();
export const importApi = new ImportApiService();
export const fixturesApi = new FixturesApiService();

// Admin API services
export const dashboardApi = new DashboardApiService();
export const serverApi = new ServerApiService();
export const routesApi = new RoutesApiService();
export const logsApi = new LogsApiService();
export const metricsApi = new MetricsApiService();
export const configApi = new ConfigApiService();
export const validationApi = new ValidationApiService();
export const envApi = new EnvApiService();
export const filesApi = new FilesApiService();
export const smokeTestsApi = new SmokeTestsApiService();
export const pluginsApi = new PluginsApiService();
export const chaosApi = new ChaosApiService();
export const timeTravelApi = new TimeTravelApiService();
export const verificationApi = new VerificationApiService();

// Debug: Log to verify services are created
logger.info('API Services initialized', {
  apiService: !!apiService,
  importApi: !!importApi,
  fixturesApi: !!fixturesApi,
  fixturesApiGetFixtures: typeof fixturesApi?.getFixtures,
  dashboardApi: !!dashboardApi,
  serverApi: !!serverApi,
  routesApi: !!routesApi,
  logsApi: !!logsApi,
  metricsApi: !!metricsApi,
  configApi: !!configApi,
  validationApi: !!validationApi,
  envApi: !!envApi,
  filesApi: !!filesApi,
  smokeTestsApi: !!smokeTestsApi,
  pluginsApi: !!pluginsApi,
  chaosApi: !!chaosApi,
  timeTravelApi: !!timeTravelApi,
});

// Type exports for backwards compatibility
