/**
 * MockForge API Client for Extension
 */

import { MockConfig, ConnectionStatus } from './types';

export class MockForgeApiClient {
    private baseUrl: string;

    constructor(baseUrl: string) {
        this.baseUrl = baseUrl.replace(/\/$/, '');
    }

    async healthCheck(): Promise<boolean> {
        try {
            const response = await fetch(`${this.baseUrl}/health`, {
                method: 'GET',
                headers: { 'Accept': 'application/json' },
                signal: AbortSignal.timeout(2000),
            });
            return response.ok;
        } catch {
            // Try mocks endpoint as fallback
            try {
                const response = await fetch(`${this.baseUrl}/mocks`, {
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
        const response = await fetch(`${this.baseUrl}/mocks`, {
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
        const response = await fetch(`${this.baseUrl}/mocks`, {
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

    async deleteMock(id: string): Promise<void> {
        const response = await fetch(`${this.baseUrl}/mocks/${id}`, {
            method: 'DELETE',
        });

        if (!response.ok && response.status !== 204) {
            throw new Error(`Failed to delete mock: ${response.status}`);
        }
    }

    getBaseUrl(): string {
        return this.baseUrl;
    }
}
