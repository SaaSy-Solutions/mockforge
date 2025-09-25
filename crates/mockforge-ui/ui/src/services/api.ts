import {
  ChainSummary,
  ChainListResponse,
  ChainDefinition,
  ChainCreationResponse,
  ChainExecutionResponse
} from '../types';
import {
  WorkspaceSummary,
  WorkspaceDetail,
  FolderDetail,
  CreateWorkspaceRequest,
  CreateFolderRequest,
  CreateRequestRequest,
  ImportToWorkspaceRequest,
  WorkspaceListResponse,
  WorkspaceResponse,
  FolderResponse,
  CreateWorkspaceResponse,
  CreateFolderResponse,
  CreateRequestResponse,
  ImportResponse,
  ExecuteRequestRequest,
  ExecuteRequestResponse,
  RequestHistoryResponse,
  EnvironmentListResponse,
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
  ImportHistoryResponse
} from '@/types';

const API_BASE = '/__mockforge/chains';
const WORKSPACE_API_BASE = '/__mockforge/workspaces';

class ApiService {
  private async fetchJson(url: string, options?: RequestInit): Promise<any> {
    const response = await fetch(url, options);
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    return response.json();
  }

  async listChains(): Promise<ChainListResponse> {
    return this.fetchJson(API_BASE);
  }

  async getChain(chainId: string): Promise<ChainDefinition> {
    return this.fetchJson(`${API_BASE}/${chainId}`);
  }

  async createChain(definition: string): Promise<ChainCreationResponse> {
    return this.fetchJson(API_BASE, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ definition }),
    });
  }

  async updateChain(chainId: string, definition: string): Promise<ChainCreationResponse> {
    return this.fetchJson(`${API_BASE}/${chainId}`, {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ definition }),
    });
  }

  async deleteChain(chainId: string): Promise<any> {
    return this.fetchJson(`${API_BASE}/${chainId}`, {
      method: 'DELETE',
    });
  }

  async executeChain(chainId: string, variables?: any): Promise<ChainExecutionResponse> {
    return this.fetchJson(`${API_BASE}/${chainId}/execute`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ variables: variables || {} }),
    });
  }

  async validateChain(chainId: string): Promise<any> {
    return this.fetchJson(`${API_BASE}/${chainId}/validate`, {
      method: 'POST',
    });
  }

  // ==================== WORKSPACE API METHODS ====================

  async listWorkspaces(): Promise<WorkspaceListResponse> {
    return this.fetchJson(WORKSPACE_API_BASE);
  }

  async getWorkspace(workspaceId: string): Promise<WorkspaceResponse> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}`);
  }

  async createWorkspace(request: CreateWorkspaceRequest): Promise<CreateWorkspaceResponse> {
    return this.fetchJson(WORKSPACE_API_BASE, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(request),
    });
  }

  async openWorkspaceFromDirectory(directory: string): Promise<CreateWorkspaceResponse> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/open-from-directory`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ directory }),
    });
  }

  async deleteWorkspace(workspaceId: string): Promise<{ message: string }> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}`, {
      method: 'DELETE',
    });
  }

  async setActiveWorkspace(workspaceId: string): Promise<{ message: string }> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/activate`, {
      method: 'POST',
    });
  }

  async getFolder(workspaceId: string, folderId: string): Promise<FolderResponse> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/folders/${folderId}`);
  }

  async createFolder(workspaceId: string, request: CreateFolderRequest): Promise<CreateFolderResponse> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/folders`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(request),
    });
  }

  async createRequest(workspaceId: string, request: CreateRequestRequest): Promise<CreateRequestResponse> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/requests`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(request),
    });
  }

  async importToWorkspace(workspaceId: string, request: ImportToWorkspaceRequest): Promise<ImportResponse> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/import`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(request),
    });
  }

  async previewImport(request: ImportToWorkspaceRequest): Promise<ImportResponse> {
    return this.fetchJson('/__mockforge/import/preview', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(request),
    });
  }

  async executeRequest(workspaceId: string, requestId: string, executionRequest?: ExecuteRequestRequest): Promise<ExecuteRequestResponse> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/requests/${requestId}/execute`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(executionRequest || {}),
    });
  }

  async getRequestHistory(workspaceId: string, requestId: string): Promise<RequestHistoryResponse> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/requests/${requestId}/history`);
  }

  // ==================== ENVIRONMENT API METHODS ====================

  async getEnvironments(workspaceId: string): Promise<EnvironmentListResponse> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/environments`);
  }

  async createEnvironment(workspaceId: string, request: CreateEnvironmentRequest): Promise<CreateEnvironmentResponse> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/environments`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    });
  }

  async updateEnvironment(workspaceId: string, environmentId: string, request: UpdateEnvironmentRequest): Promise<{ message: string }> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/environments/${environmentId}`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    });
  }

  async deleteEnvironment(workspaceId: string, environmentId: string): Promise<{ message: string }> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/environments/${environmentId}`, {
      method: 'DELETE',
    });
  }

  async setActiveEnvironment(workspaceId: string, environmentId: string): Promise<{ message: string }> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/environments/${environmentId}/activate`, {
      method: 'POST',
    });
  }

  async getEnvironmentVariables(workspaceId: string, environmentId: string): Promise<EnvironmentVariablesResponse> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/environments/${environmentId}/variables`);
  }

  async setEnvironmentVariable(workspaceId: string, environmentId: string, request: SetVariableRequest): Promise<{ message: string }> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/environments/${environmentId}/variables`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    });
  }

  async removeEnvironmentVariable(workspaceId: string, environmentId: string, variableName: string): Promise<{ message: string }> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/environments/${environmentId}/variables/${encodeURIComponent(variableName)}`, {
      method: 'DELETE',
    });
  }

  async getAutocompleteSuggestions(workspaceId: string, request: AutocompleteRequest): Promise<AutocompleteResponse> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/autocomplete`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    });
  }

  // ==================== ORDERING API METHODS ====================

  async updateWorkspacesOrder(workspaceIds: string[]): Promise<{ message: string }> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/order`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ workspace_ids: workspaceIds }),
    });
  }

  async updateEnvironmentsOrder(workspaceId: string, environmentIds: string[]): Promise<{ message: string }> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/environments/order`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ environment_ids: environmentIds }),
    });
  }

  // ==================== SYNC API METHODS ====================

  async getSyncStatus(workspaceId: string): Promise<SyncStatus> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/sync/status`);
  }

  async configureSync(workspaceId: string, request: ConfigureSyncRequest): Promise<{ message: string }> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/sync/configure`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    });
  }

  async disableSync(workspaceId: string): Promise<{ message: string }> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/sync/disable`, {
      method: 'POST',
    });
  }

  async triggerSync(workspaceId: string): Promise<{ message: string }> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/sync/trigger`, {
      method: 'POST',
    });
  }

  async getSyncChanges(workspaceId: string): Promise<SyncChange[]> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/sync/changes`);
  }

  async confirmSyncChanges(workspaceId: string, request: ConfirmSyncChangesRequest): Promise<{ message: string }> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/sync/confirm`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    });
  }

  // ==================== ENCRYPTION API METHODS ====================

  async getWorkspaceEncryptionStatus(workspaceId: string): Promise<any> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/encryption/status`);
  }

  async getWorkspaceEncryptionConfig(workspaceId: string): Promise<any> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/encryption/config`);
  }

  async enableWorkspaceEncryption(workspaceId: string): Promise<{ message: string }> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/encryption/enable`, {
      method: 'POST',
    });
  }

  async disableWorkspaceEncryption(workspaceId: string): Promise<{ message: string }> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/encryption/disable`, {
      method: 'POST',
    });
  }

  async checkWorkspaceSecurity(workspaceId: string): Promise<any> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/encryption/security-check`, {
      method: 'POST',
    });
  }

  async exportWorkspaceEncrypted(workspaceId: string, exportPath: string): Promise<{ message: string }> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/encryption/export`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ export_path: exportPath }),
    });
  }

  async importWorkspaceEncrypted(importPath: string, workspaceId: string, backupKey: string): Promise<{ message: string }> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/encryption/import`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ import_path: importPath, backup_key: backupKey }),
    });
  }

  async updateWorkspaceEncryptionConfig(workspaceId: string, config: any): Promise<{ message: string }> {
    return this.fetchJson(`${WORKSPACE_API_BASE}/${workspaceId}/encryption/config`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(config),
    });
  }
}

class ImportApiService {
  private async fetchJson(url: string, options?: RequestInit): Promise<any> {
    const response = await fetch(url, options);
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    return response.json();
  }

  async importPostman(request: ImportRequest): Promise<ImportResponse> {
    return this.fetchJson('/__mockforge/import/postman', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(request),
    });
  }

  async importInsomnia(request: ImportRequest): Promise<ImportResponse> {
    return this.fetchJson('/__mockforge/import/insomnia', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(request),
    });
  }

  async importCurl(request: ImportRequest): Promise<ImportResponse> {
    return this.fetchJson('/__mockforge/import/curl', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(request),
    });
  }

  async importOpenApi(request: ImportRequest): Promise<ImportResponse> {
    return this.fetchJson('/__mockforge/import/openapi', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(request),
    });
  }

  async previewImport(request: ImportRequest): Promise<ImportResponse> {
    return this.fetchJson('/__mockforge/import/preview', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(request),
    });
  }

  async getImportHistory(): Promise<ImportHistoryResponse> {
    return this.fetchJson('/__mockforge/import/history');
  }

  async clearImportHistory(): Promise<void> {
    return this.fetchJson('/__mockforge/import/history/clear', {
      method: 'POST',
    });
  }
}

export const apiService = new ApiService();
export const importApi = new ImportApiService();
