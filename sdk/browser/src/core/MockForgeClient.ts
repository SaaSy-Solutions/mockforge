/**
 * MockForge API Client
 *
 * Handles communication with MockForge server management API
 */

import { MockConfig, ConnectionStatus, Environment, EnvironmentVariable } from '../types';

/**
 * Client for interacting with MockForge management API
 */
export class MockForgeClient {
    private baseUrl: string;
    private connected: boolean = false;
    private lastError?: string;

    constructor(baseUrl: string) {
        this.baseUrl = baseUrl.replace(/\/$/, ''); // Remove trailing slash
    }

    /**
     * Check if MockForge server is available
     */
    async healthCheck(): Promise<boolean> {
        try {
            // Try multiple health check endpoints
            const healthEndpoints = ['/health', '/api/health', '/api/v1/health'];

            for (const endpoint of healthEndpoints) {
                try {
                    const response = await fetch(`${this.baseUrl}${endpoint}`, {
                        method: 'GET',
                        headers: {
                            'Accept': 'application/json',
                        },
                        signal: AbortSignal.timeout(2000), // 2 second timeout
                    });

                    if (response.ok) {
                        this.connected = true;
                        this.lastError = undefined;
                        return true;
                    }
                } catch (error) {
                    // Try next endpoint
                    continue;
                }
            }

            // If no health endpoint works, try the mocks endpoint
            const response = await fetch(`${this.baseUrl}/mocks`, {
                method: 'GET',
                headers: {
                    'Accept': 'application/json',
                },
                signal: AbortSignal.timeout(2000),
            });

            if (response.ok || response.status === 401) {
                // 401 means server is there but auth required - still counts as connected
                this.connected = true;
                this.lastError = undefined;
                return true;
            }

            this.connected = false;
            this.lastError = `Health check failed with status ${response.status}`;
            return false;
        } catch (error) {
            this.connected = false;
            this.lastError = error instanceof Error ? error.message : 'Unknown error';
            return false;
        }
    }

    /**
     * Get connection status
     */
    getConnectionStatus(): ConnectionStatus {
        return {
            connected: this.connected,
            url: this.baseUrl,
            error: this.lastError,
            lastConnected: this.connected ? Date.now() : undefined,
        };
    }

    /**
     * List all mocks
     */
    async listMocks(): Promise<MockConfig[]> {
        const response = await this.request('/mocks', {
            method: 'GET',
        });

        if (!response.ok) {
            throw new Error(`Failed to list mocks: ${response.status} ${response.statusText}`);
        }

        const data = await response.json();
        return data.mocks || [];
    }

    /**
     * Get a specific mock by ID
     */
    async getMock(id: string): Promise<MockConfig> {
        const response = await this.request(`/mocks/${id}`, {
            method: 'GET',
        });

        if (!response.ok) {
            throw new Error(`Failed to get mock: ${response.status} ${response.statusText}`);
        }

        return await response.json();
    }

    /**
     * Create a new mock
     */
    async createMock(mock: MockConfig): Promise<MockConfig> {
        const response = await this.request('/mocks', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify(mock),
        });

        if (!response.ok) {
            const errorText = await response.text();
            throw new Error(`Failed to create mock: ${response.status} ${response.statusText} - ${errorText}`);
        }

        return await response.json();
    }

    /**
     * Update an existing mock
     */
    async updateMock(id: string, mock: MockConfig): Promise<MockConfig> {
        const response = await this.request(`/mocks/${id}`, {
            method: 'PUT',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify(mock),
        });

        if (!response.ok) {
            throw new Error(`Failed to update mock: ${response.status} ${response.statusText}`);
        }

        return await response.json();
    }

    /**
     * Delete a mock
     */
    async deleteMock(id: string): Promise<void> {
        const response = await this.request(`/mocks/${id}`, {
            method: 'DELETE',
        });

        if (!response.ok && response.status !== 204) {
            throw new Error(`Failed to delete mock: ${response.status} ${response.statusText}`);
        }
    }

    /**
     * Get server statistics
     */
    async getStats(): Promise<any> {
        const response = await this.request('/stats', {
            method: 'GET',
        });

        if (!response.ok) {
            throw new Error(`Failed to get stats: ${response.status} ${response.statusText}`);
        }

        return await response.json();
    }

    /**
     * Get server configuration
     */
    async getConfig(): Promise<any> {
        const response = await this.request('/config', {
            method: 'GET',
        });

        if (!response.ok) {
            throw new Error(`Failed to get config: ${response.status} ${response.statusText}`);
        }

        return await response.json();
    }

    /**
     * Make a request to the MockForge API
     */
    private async request(path: string, options: RequestInit = {}): Promise<Response> {
        const url = `${this.baseUrl}${path}`;

        const response = await fetch(url, {
            ...options,
            headers: {
                'Accept': 'application/json',
                ...options.headers,
            },
        });

        // Update connection status based on response
        if (response.ok || response.status === 401 || response.status === 403) {
            this.connected = true;
            this.lastError = undefined;
        } else if (response.status >= 500) {
            this.connected = false;
            this.lastError = `Server error: ${response.status}`;
        }

        return response;
    }

    /**
     * Get the base URL
     */
    getBaseUrl(): string {
        return this.baseUrl;
    }

    /**
     * Set the base URL
     */
    setBaseUrl(url: string): void {
        this.baseUrl = url.replace(/\/$/, '');
        this.connected = false; // Reset connection status
    }

    /**
     * Get default workspace ID (for now, use 'default')
     * In the future, this could be retrieved from the API
     */
    private getDefaultWorkspaceId(): string {
        return 'default';
    }

    /**
     * List all environments for a workspace
     */
    async listEnvironments(workspaceId?: string): Promise<Environment[]> {
        const wsId = workspaceId || this.getDefaultWorkspaceId();
        const response = await this.request(`/__mockforge/workspaces/${wsId}/environments`, {
            method: 'GET',
        });

        if (!response.ok) {
            throw new Error(`Failed to list environments: ${response.status} ${response.statusText}`);
        }

        const data = await response.json();
        return data.data?.environments || data.environments || [];
    }

    /**
     * Get the active environment for a workspace
     */
    async getActiveEnvironment(workspaceId?: string): Promise<Environment | null> {
        const environments = await this.listEnvironments(workspaceId);
        return environments.find((env) => env.active) || environments[0] || null;
    }

    /**
     * Set the active environment
     */
    async setActiveEnvironment(workspaceId: string | undefined, environmentId: string): Promise<void> {
        const wsId = workspaceId || this.getDefaultWorkspaceId();
        const response = await this.request(
            `/__mockforge/workspaces/${wsId}/environments/${environmentId}/activate`,
            {
                method: 'POST',
            }
        );

        if (!response.ok) {
            throw new Error(`Failed to set active environment: ${response.status} ${response.statusText}`);
        }
    }

    /**
     * Get environment variables for an environment
     */
    async getEnvironmentVariables(workspaceId: string | undefined, environmentId: string): Promise<Record<string, string>> {
        const wsId = workspaceId || this.getDefaultWorkspaceId();
        const response = await this.request(
            `/__mockforge/workspaces/${wsId}/environments/${environmentId}/variables`,
            {
                method: 'GET',
            }
        );

        if (!response.ok) {
            throw new Error(`Failed to get environment variables: ${response.status} ${response.statusText}`);
        }

        const data = await response.json();
        const variables = data.data?.variables || data.variables || [];

        // Convert array to object if needed
        if (Array.isArray(variables)) {
            const result: Record<string, string> = {};
            variables.forEach((v: EnvironmentVariable) => {
                result[v.key] = v.value;
            });
            return result;
        }

        return variables;
    }

    /**
     * Set an environment variable
     */
    async setEnvironmentVariable(
        workspaceId: string | undefined,
        environmentId: string,
        key: string,
        value: string
    ): Promise<void> {
        const wsId = workspaceId || this.getDefaultWorkspaceId();
        const response = await this.request(
            `/__mockforge/workspaces/${wsId}/environments/${environmentId}/variables`,
            {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({ key, value }),
            }
        );

        if (!response.ok) {
            throw new Error(`Failed to set environment variable: ${response.status} ${response.statusText}`);
        }
    }
}
