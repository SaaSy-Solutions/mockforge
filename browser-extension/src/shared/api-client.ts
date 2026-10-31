/**
 * MockForge API Client for Extension
 */

import { MockConfig, ConnectionStatus, Environment } from './types';

/** Header carrying the management credential on control-plane requests. */
export const MANAGEMENT_TOKEN_HEADER = 'X-MockForge-Management-Token';

/**
 * chrome.storage.local key for the management token. Kept in local (not sync)
 * storage so the credential never leaves this browser profile.
 */
export const MANAGEMENT_TOKEN_STORAGE_KEY = 'mockForgeManagementToken';

/**
 * Headers carrying the management token, or none when it is unset.
 */
export function managementHeaders(token?: string): Record<string, string> {
    return token ? { [MANAGEMENT_TOKEN_HEADER]: token } : {};
}

/**
 * Read the configured management token ('' when unset).
 */
export async function loadManagementToken(): Promise<string> {
    const result = await chrome.storage.local.get([MANAGEMENT_TOKEN_STORAGE_KEY]);
    const token = result[MANAGEMENT_TOKEN_STORAGE_KEY];
    return typeof token === 'string' ? token.trim() : '';
}

export class MockForgeApiClient {
    private baseUrl: string;
    private managementToken: string;

    constructor(baseUrl: string, managementToken: string = '') {
        this.baseUrl = baseUrl.replace(/\/$/, '');
        this.managementToken = managementToken;
    }

    /**
     * The management token this client sends ('' when unset)
     */
    getManagementToken(): string {
        return this.managementToken;
    }

    /**
     * fetch() with the management token attached to every request
     */
    private send(url: string, init: RequestInit = {}): Promise<Response> {
        return fetch(url, {
            ...init,
            headers: {
                ...(init.headers as Record<string, string> | undefined),
                ...managementHeaders(this.managementToken),
            },
        });
    }

    async healthCheck(): Promise<boolean> {
        try {
            const response = await this.send(`${this.baseUrl}/health`, {
                method: 'GET',
                headers: { 'Accept': 'application/json' },
                signal: AbortSignal.timeout(2000),
            });
            return response.ok;
        } catch {
            // Try mocks endpoint as fallback
            try {
                const response = await this.send(`${this.baseUrl}/mocks`, {
                    method: 'GET',
                    headers: { 'Accept': 'application/json' },
                    signal: AbortSignal.timeout(2000),
                });
                return response.ok || response.status === 401;
            } catch {
                return false;
            }
        }
    }

    async listMocks(): Promise<MockConfig[]> {
        const response = await this.send(`${this.baseUrl}/mocks`, {
            method: 'GET',
            headers: { 'Accept': 'application/json' },
        });

        if (!response.ok) {
            throw new Error(`Failed to list mocks: ${response.status}`);
        }

        const data = await response.json();
        return data.mocks || [];
    }

    async createMock(mock: MockConfig): Promise<MockConfig> {
        const response = await this.send(`${this.baseUrl}/mocks`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                'Accept': 'application/json',
            },
            body: JSON.stringify(mock),
        });

        if (!response.ok) {
            const errorText = await response.text();
            throw new Error(`Failed to create mock: ${response.status} - ${errorText}`);
        }

        return await response.json();
    }

    async updateMock(id: string, mock: MockConfig): Promise<MockConfig> {
        const response = await this.send(`${this.baseUrl}/mocks/${id}`, {
            method: 'PUT',
            headers: {
                'Content-Type': 'application/json',
                'Accept': 'application/json',
            },
            body: JSON.stringify(mock),
        });

        if (!response.ok) {
            const errorText = await response.text();
            throw new Error(`Failed to update mock: ${response.status} - ${errorText}`);
        }

        return await response.json();
    }

    async deleteMock(id: string): Promise<void> {
        const response = await this.send(`${this.baseUrl}/mocks/${id}`, {
            method: 'DELETE',
        });

        if (!response.ok && response.status !== 204) {
            throw new Error(`Failed to delete mock: ${response.status}`);
        }
    }

    getBaseUrl(): string {
        return this.baseUrl;
    }

    private getDefaultWorkspaceId(): string {
        return 'default';
    }

    async listEnvironments(workspaceId?: string): Promise<Environment[]> {
        const wsId = workspaceId || this.getDefaultWorkspaceId();
        const response = await this.send(`${this.baseUrl}/__mockforge/workspaces/${wsId}/environments`, {
            method: 'GET',
            headers: { 'Accept': 'application/json' },
        });

        if (!response.ok) {
            throw new Error(`Failed to list environments: ${response.status}`);
        }

        const data = await response.json();
        return data.data?.environments || data.environments || [];
    }

    async getActiveEnvironment(workspaceId?: string): Promise<Environment | null> {
        const environments = await this.listEnvironments(workspaceId);
        return environments.find((env) => env.active) || environments[0] || null;
    }

    async setActiveEnvironment(workspaceId: string | undefined, environmentId: string): Promise<void> {
        const wsId = workspaceId || this.getDefaultWorkspaceId();
        const response = await this.send(
            `${this.baseUrl}/__mockforge/workspaces/${wsId}/environments/${environmentId}/activate`,
            {
                method: 'POST',
                headers: { 'Accept': 'application/json' },
            }
        );

        if (!response.ok) {
            throw new Error(`Failed to set active environment: ${response.status}`);
        }
    }

    async getEnvironmentVariables(workspaceId: string | undefined, environmentId: string): Promise<Record<string, string>> {
        const wsId = workspaceId || this.getDefaultWorkspaceId();
        const response = await this.send(
            `${this.baseUrl}/__mockforge/workspaces/${wsId}/environments/${environmentId}/variables`,
            {
                method: 'GET',
                headers: { 'Accept': 'application/json' },
            }
        );

        if (!response.ok) {
            throw new Error(`Failed to get environment variables: ${response.status}`);
        }

        const data = await response.json();
        const variables = data.data?.variables || data.variables || [];

        if (Array.isArray(variables)) {
            const result: Record<string, string> = {};
            variables.forEach((v: { key: string; value: string }) => {
                result[v.key] = v.value;
            });
            return result;
        }

        return variables;
    }

    async getWorkspaceState(workspaceId?: string): Promise<any> {
        const wsId = workspaceId || this.getDefaultWorkspaceId();
        const response = await this.send(`${this.baseUrl}/api/v1/consistency/state?workspace=${wsId}`, {
            method: 'GET',
            headers: { 'Accept': 'application/json' },
        });

        if (!response.ok) {
            throw new Error(`Failed to get workspace state: ${response.status}`);
        }

        return await response.json();
    }

    async setActivePersona(workspaceId: string | undefined, persona: { id: string; name?: string; domain?: string; traits?: Record<string, string> }): Promise<void> {
        const wsId = workspaceId || this.getDefaultWorkspaceId();
        const response = await this.send(`${this.baseUrl}/api/v1/consistency/persona?workspace=${wsId}`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                'Accept': 'application/json',
            },
            body: JSON.stringify({ persona }),
        });

        if (!response.ok) {
            throw new Error(`Failed to set active persona: ${response.status}`);
        }
    }

    async setActiveScenario(workspaceId: string | undefined, scenarioId: string): Promise<void> {
        const wsId = workspaceId || this.getDefaultWorkspaceId();
        const response = await this.send(`${this.baseUrl}/api/v1/consistency/scenario?workspace=${wsId}`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                'Accept': 'application/json',
            },
            body: JSON.stringify({ scenario_id: scenarioId }),
        });

        if (!response.ok) {
            throw new Error(`Failed to set active scenario: ${response.status}`);
        }
    }
}
