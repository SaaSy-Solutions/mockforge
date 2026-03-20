/**
 * Workspaces API service — workspace CRUD, folders, requests, environments, ordering, sync, encryption.
 */
import type {
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
  EncryptionStatus,
  AutoEncryptionConfig,
  SecurityCheckResult,
} from '../../types';
import {
  WorkspaceListResponseSchema,
  type WorkspaceSummary,
} from '../../schemas/api';
import { fetchJson, fetchJsonWithValidation } from './client';

const WORKSPACE_API_BASE = '/__mockforge/workspaces';

class WorkspacesApiMixin {
  // ==================== WORKSPACE CRUD ====================

  async listWorkspaces(): Promise<WorkspaceSummary[]> {
    return fetchJsonWithValidation<WorkspaceSummary[]>(
      WORKSPACE_API_BASE,
      WorkspaceListResponseSchema
    );
  }

  async getWorkspace(workspaceId: string): Promise<WorkspaceResponse> {
    return fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}`) as Promise<WorkspaceResponse>;
  }

  async createWorkspace(request: CreateWorkspaceRequest): Promise<CreateWorkspaceResponse> {
    return fetchJson(WORKSPACE_API_BASE, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as Promise<CreateWorkspaceResponse>;
  }

  async openWorkspaceFromDirectory(directory: string): Promise<CreateWorkspaceResponse> {
    return fetchJson(`${WORKSPACE_API_BASE}/open-from-directory`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ directory }),
    }) as Promise<CreateWorkspaceResponse>;
  }

  async deleteWorkspace(workspaceId: string): Promise<{ message: string }> {
    return fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}`, {
      method: 'DELETE',
    }) as Promise<{ message: string }>;
  }

  async setActiveWorkspace(workspaceId: string): Promise<{ message: string }> {
    return fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/activate`, {
      method: 'POST',
    }) as Promise<{ message: string }>;
  }

  // ==================== FOLDERS & REQUESTS ====================

  async getFolder(workspaceId: string, folderId: string): Promise<FolderResponse> {
    return fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/folders/${folderId}`) as Promise<FolderResponse>;
  }

  async createFolder(workspaceId: string, request: CreateFolderRequest): Promise<CreateFolderResponse> {
    return fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/folders`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as Promise<CreateFolderResponse>;
  }

  async createRequest(workspaceId: string, request: CreateRequestRequest): Promise<CreateRequestResponse> {
    return fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/requests`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as Promise<CreateRequestResponse>;
  }

  async importToWorkspace(workspaceId: string, request: ImportToWorkspaceRequest): Promise<ImportResponse> {
    return fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/import`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as Promise<ImportResponse>;
  }

  async previewImport(request: ImportToWorkspaceRequest): Promise<ImportResponse> {
    return fetchJson('/__mockforge/import/preview', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as Promise<ImportResponse>;
  }

  async executeRequest(workspaceId: string, requestId: string, executionRequest?: ExecuteRequestRequest): Promise<ExecuteRequestResponse> {
    return fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/requests/${requestId}/execute`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(executionRequest || {}),
    }) as Promise<ExecuteRequestResponse>;
  }

  async getRequestHistory(workspaceId: string, requestId: string): Promise<RequestHistoryResponse> {
    return fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/requests/${requestId}/history`) as Promise<RequestHistoryResponse>;
  }

  // ==================== ENVIRONMENTS ====================

  async getEnvironments(workspaceId: string): Promise<EnvironmentListResponse> {
    return fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/environments`) as Promise<EnvironmentListResponse>;
  }

  async createEnvironment(workspaceId: string, request: CreateEnvironmentRequest): Promise<CreateEnvironmentResponse> {
    return fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/environments`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as Promise<CreateEnvironmentResponse>;
  }

  async updateEnvironment(workspaceId: string, environmentId: string, request: UpdateEnvironmentRequest): Promise<{ message: string }> {
    return fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/environments/${environmentId}`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as Promise<{ message: string }>;
  }

  async deleteEnvironment(workspaceId: string, environmentId: string): Promise<{ message: string }> {
    return fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/environments/${environmentId}`, {
      method: 'DELETE',
    }) as Promise<{ message: string }>;
  }

  async setActiveEnvironment(workspaceId: string, environmentId: string): Promise<{ message: string }> {
    return fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/environments/${environmentId}/activate`, {
      method: 'POST',
    }) as Promise<{ message: string }>;
  }

  async getEnvironmentVariables(workspaceId: string, environmentId: string): Promise<EnvironmentVariablesResponse> {
    return fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/environments/${environmentId}/variables`) as Promise<EnvironmentVariablesResponse>;
  }

  async setEnvironmentVariable(workspaceId: string, environmentId: string, request: SetVariableRequest): Promise<{ message: string }> {
    return fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/environments/${environmentId}/variables`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as Promise<{ message: string }>;
  }

  async removeEnvironmentVariable(workspaceId: string, environmentId: string, variableName: string): Promise<{ message: string }> {
    return fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/environments/${environmentId}/variables/${encodeURIComponent(variableName)}`, {
      method: 'DELETE',
    }) as Promise<{ message: string }>;
  }

  async getAutocompleteSuggestions(workspaceId: string, request: AutocompleteRequest): Promise<AutocompleteResponse> {
    return fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/autocomplete`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as Promise<AutocompleteResponse>;
  }

  // ==================== ORDERING ====================

  async updateWorkspacesOrder(workspaceIds: string[]): Promise<{ message: string }> {
    return fetchJson(`${WORKSPACE_API_BASE}/order`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ workspace_ids: workspaceIds }),
    }) as Promise<{ message: string }>;
  }

  async updateEnvironmentsOrder(workspaceId: string, environmentIds: string[]): Promise<{ message: string }> {
    return fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/environments/order`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ environment_ids: environmentIds }),
    }) as Promise<{ message: string }>;
  }

  // ==================== SYNC ====================

  async getSyncStatus(workspaceId: string): Promise<SyncStatus> {
    return fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/sync/status`) as Promise<SyncStatus>;
  }

  async configureSync(workspaceId: string, request: ConfigureSyncRequest): Promise<{ message: string }> {
    return fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/sync/configure`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as Promise<{ message: string }>;
  }

  async disableSync(workspaceId: string): Promise<{ message: string }> {
    return fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/sync/disable`, {
      method: 'POST',
    }) as Promise<{ message: string }>;
  }

  async triggerSync(workspaceId: string): Promise<{ message: string }> {
    return fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/sync/trigger`, {
      method: 'POST',
    }) as Promise<{ message: string }>;
  }

  async getSyncChanges(workspaceId: string): Promise<SyncChange[]> {
    return fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/sync/changes`) as Promise<SyncChange[]>;
  }

  async confirmSyncChanges(workspaceId: string, request: ConfirmSyncChangesRequest): Promise<{ message: string }> {
    return fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/sync/confirm`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    }) as Promise<{ message: string }>;
  }

  // ==================== ENCRYPTION ====================

  async getWorkspaceEncryptionStatus(workspaceId: string): Promise<EncryptionStatus> {
    return fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/encryption/status`) as Promise<EncryptionStatus>;
  }

  async getWorkspaceEncryptionConfig(workspaceId: string): Promise<AutoEncryptionConfig> {
    return fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/encryption/config`) as Promise<AutoEncryptionConfig>;
  }

  async enableWorkspaceEncryption(workspaceId: string): Promise<{ message: string }> {
    return fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/encryption/enable`, {
      method: 'POST',
    }) as Promise<{ message: string }>;
  }

  async disableWorkspaceEncryption(workspaceId: string): Promise<{ message: string }> {
    return fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/encryption/disable`, {
      method: 'POST',
    }) as Promise<{ message: string }>;
  }

  async checkWorkspaceSecurity(workspaceId: string): Promise<SecurityCheckResult> {
    return fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/encryption/security-check`, {
      method: 'POST',
    }) as Promise<SecurityCheckResult>;
  }

  async exportWorkspaceEncrypted(workspaceId: string, exportPath: string): Promise<{ message: string }> {
    return fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/encryption/export`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ export_path: exportPath }),
    }) as Promise<{ message: string }>;
  }

  async importWorkspaceEncrypted(importPath: string, workspaceId: string, backupKey: string): Promise<{ message: string }> {
    return fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/encryption/import`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ import_path: importPath, backup_key: backupKey }),
    }) as Promise<{ message: string }>;
  }

  async updateWorkspaceEncryptionConfig(workspaceId: string, config: unknown): Promise<{ message: string }> {
    return fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/encryption/config`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(config),
    }) as Promise<{ message: string }>;
  }
}

export { WorkspacesApiMixin };
