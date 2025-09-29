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
  ImportHistoryResponse
} from '../types';

// Re-export FixtureInfo from types for backwards compatibility
export type { FixtureInfo } from '../types/index';

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

class FixturesApiService {
  private async fetchJson(url: string, options?: RequestInit): Promise<any> {
    const response = await fetch(url, options);
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    return response.json();
  }

  async getFixtures(): Promise<FixtureInfo[]> {
    return this.fetchJson('/__mockforge/fixtures');
  }

  async deleteFixture(fixtureId: string): Promise<void> {
    return this.fetchJson(`/__mockforge/fixtures/${fixtureId}`, {
      method: 'DELETE',
    });
  }

  async deleteFixturesBulk(fixtureIds: string[]): Promise<void> {
    return this.fetchJson('/__mockforge/fixtures/bulk', {
      method: 'DELETE',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ fixture_ids: fixtureIds }),
    });
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
    });
  }

  async moveFixture(fixtureId: string, newPath: string): Promise<void> {
    return this.fetchJson(`/__mockforge/fixtures/${fixtureId}/move`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ new_path: newPath }),
    });
  }
}

// ==================== ADMIN API METHODS ====================

class DashboardApiService {
  private async fetchJson(url: string, options?: RequestInit): Promise<any> {
    const response = await fetch(url, options);
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    return response.json();
  }

  async getDashboard(): Promise<any> {
    return this.fetchJson('/__mockforge/dashboard');
  }

  async getHealth(): Promise<any> {
    return this.fetchJson('/__mockforge/health');
  }
}

class ServerApiService {
  private async fetchJson(url: string, options?: RequestInit): Promise<any> {
    const response = await fetch(url, options);
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    return response.json();
  }

  async getServerInfo(): Promise<any> {
    return this.fetchJson('/__mockforge/server-info');
  }

  async restartServer(reason?: string): Promise<any> {
    return this.fetchJson('/__mockforge/servers/restart', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ reason: reason || 'Manual restart' }),
    });
  }

  async getRestartStatus(): Promise<any> {
    return this.fetchJson('/__mockforge/servers/restart/status');
  }
}

class RoutesApiService {
  private async fetchJson(url: string, options?: RequestInit): Promise<any> {
    const response = await fetch(url, options);
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    return response.json();
  }

  async getRoutes(): Promise<any> {
    return this.fetchJson('/__mockforge/routes');
  }
}

class LogsApiService {
  private async fetchJson(url: string, options?: RequestInit): Promise<any> {
    const response = await fetch(url, options);
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    return response.json();
  }

  async getLogs(params?: any): Promise<any> {
    const queryString = params ? '?' + new URLSearchParams(params).toString() : '';
    return this.fetchJson(`/__mockforge/logs${queryString}`);
  }

  async clearLogs(): Promise<any> {
    return this.fetchJson('/__mockforge/logs', {
      method: 'DELETE',
    });
  }
}

class MetricsApiService {
  private async fetchJson(url: string, options?: RequestInit): Promise<any> {
    const response = await fetch(url, options);
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    return response.json();
  }

  async getMetrics(): Promise<any> {
    return this.fetchJson('/__mockforge/metrics');
  }
}

class ConfigApiService {
  private async fetchJson(url: string, options?: RequestInit): Promise<any> {
    const response = await fetch(url, options);
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    return response.json();
  }

  async getConfig(): Promise<any> {
    return this.fetchJson('/__mockforge/config');
  }

  async updateLatency(config: any): Promise<any> {
    return this.fetchJson('/__mockforge/config/latency', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(config),
    });
  }

  async updateFaults(config: any): Promise<any> {
    return this.fetchJson('/__mockforge/config/faults', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(config),
    });
  }

  async updateProxy(config: any): Promise<any> {
    return this.fetchJson('/__mockforge/config/proxy', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(config),
    });
  }
}

class ValidationApiService {
  private async fetchJson(url: string, options?: RequestInit): Promise<any> {
    const response = await fetch(url, options);
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    return response.json();
  }

  async updateValidation(config: any): Promise<any> {
    return this.fetchJson('/__mockforge/validation', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(config),
    });
  }
}

class EnvApiService {
  private async fetchJson(url: string, options?: RequestInit): Promise<any> {
    const response = await fetch(url, options);
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    return response.json();
  }

  async getEnv(): Promise<any> {
    return this.fetchJson('/__mockforge/env');
  }

  async updateEnv(key: string, value: string): Promise<any> {
    return this.fetchJson('/__mockforge/env', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ key, value }),
    });
  }
}

class FilesApiService {
  private async fetchJson(url: string, options?: RequestInit): Promise<any> {
    const response = await fetch(url, options);
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    return response.json();
  }

  async getFileContent(request: any): Promise<any> {
    return this.fetchJson('/__mockforge/files/content', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    });
  }

  async saveFileContent(request: any): Promise<any> {
    return this.fetchJson('/__mockforge/files/save', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    });
  }
}

class SmokeTestsApiService {
  private async fetchJson(url: string, options?: RequestInit): Promise<any> {
    const response = await fetch(url, options);
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    return response.json();
  }

  async getSmokeTests(): Promise<any> {
    return this.fetchJson('/__mockforge/smoke');
  }

  async runSmokeTests(): Promise<any> {
    return this.fetchJson('/__mockforge/smoke/run', {
      method: 'GET',
    });
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

// Type exports for backwards compatibility
export type DashboardData = any;
