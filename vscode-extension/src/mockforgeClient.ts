import axios, { AxiosInstance } from 'axios';
import WebSocket from 'ws';

export interface MockConfig {
    id: string;
    name: string;
    method: string;
    path: string;
    response: {
        body: any;
        headers?: Record<string, string>;
    };
    enabled: boolean;
    latency_ms?: number;
    status_code?: number;
}

export interface ServerStats {
    uptime_seconds: number;
    total_requests: number;
    active_mocks: number;
    enabled_mocks: number;
    registered_routes: number;
}

export interface ServerConfig {
    version: string;
    port: number;
    has_openapi_spec: boolean;
    spec_path?: string;
}

export class MockForgeClient {
    private http: AxiosInstance;
    private ws?: WebSocket;
    private listeners: ((event: any) => void)[] = [];

    constructor(private serverUrl: string) {
        this.http = axios.create({
            baseURL: `${serverUrl}/__mockforge/api`,
            timeout: 5000
        });
    }

    async connect(): Promise<void> {
        // Test HTTP connection
        await this.http.get('/health');

        // Connect WebSocket
        const wsUrl = this.serverUrl.replace('http', 'ws') + '/__mockforge/ws';
        this.ws = new WebSocket(wsUrl);

        this.ws.on('open', () => {
            console.log('WebSocket connected');
        });

        this.ws.on('message', (data) => {
            try {
                const event = JSON.parse(data.toString());
                this.listeners.forEach(listener => listener(event));
            } catch (error) {
                console.error('Failed to parse WebSocket message:', error);
            }
        });

        this.ws.on('error', (error) => {
            console.error('WebSocket error:', error);
        });

        this.ws.on('close', () => {
            console.log('WebSocket disconnected');
        });
    }

    disconnect(): void {
        if (this.ws) {
            this.ws.close();
            this.ws = undefined;
        }
    }

    onEvent(listener: (event: any) => void): void {
        this.listeners.push(listener);
    }

    async getMocks(): Promise<MockConfig[]> {
        const response = await this.http.get('/mocks');
        return response.data.mocks;
    }

    async getMock(id: string): Promise<MockConfig> {
        const response = await this.http.get(`/mocks/${id}`);
        return response.data;
    }

    async createMock(mock: Omit<MockConfig, 'id'>): Promise<MockConfig> {
        const response = await this.http.post('/mocks', mock);
        return response.data;
    }

    async updateMock(id: string, mock: MockConfig): Promise<MockConfig> {
        const response = await this.http.put(`/mocks/${id}`, mock);
        return response.data;
    }

    async deleteMock(id: string): Promise<void> {
        await this.http.delete(`/mocks/${id}`);
    }

    async getStats(): Promise<ServerStats> {
        const response = await this.http.get('/stats');
        return response.data;
    }

    async getConfig(): Promise<ServerConfig> {
        const response = await this.http.get('/config');
        return response.data;
    }

    async exportMocks(format: string): Promise<string> {
        const response = await this.http.get(`/export?format=${format}`);
        return response.data;
    }

    async importMocks(data: string, format: string, merge: boolean): Promise<void> {
        await this.http.post(`/import?format=${format}&merge=${merge}`, data, {
            headers: { 'Content-Type': 'text/plain' }
        });
    }
}
