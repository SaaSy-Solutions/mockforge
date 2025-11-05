import axios, { AxiosInstance, InternalAxiosRequestConfig, AxiosResponse, AxiosError } from 'axios';
import WebSocket from 'ws';
import * as vscode from 'vscode';
import { MockEvent } from '../types/events';
import { Logger } from '../utils/logger';
import { MockConfig, ServerStats, ServerConfig, ConnectionState } from '../types/mock';

// Re-export types for convenience
export type { MockConfig, ServerStats, ServerConfig, ConnectionState } from '../types/mock';

/**
 * Client for interacting with MockForge server API and WebSocket
 */
export class MockForgeClient {
    private http: AxiosInstance;
    private ws?: WebSocket;
    private listeners: ((event: MockEvent) => void)[] = [];
    private stateListeners: ((state: ConnectionState) => void)[] = [];

    // Connection state management
    private _connectionState: ConnectionState = 'disconnected';
    private reconnectTimer?: NodeJS.Timeout;
    private reconnectAttempts = 0;
    private manualDisconnect = false;

    // Reconnection configuration (will be loaded from VS Code config)
    private reconnectEnabled = true;
    private reconnectInitialDelay = 1000;
    private reconnectMaxDelay = 30000;
    private reconnectMaxRetries = 10;

    constructor(private serverUrl: string) {
        // Load configuration
        this.loadConfiguration();

        // Get HTTP timeout from config
        const config = vscode.workspace.getConfiguration('mockforge');
        const httpTimeout = config.get<number>('http.timeout', 5000);

        this.http = axios.create({
            baseURL: `${serverUrl}/__mockforge/api`,
            timeout: httpTimeout
        });

        // Add HTTP retry interceptor
        this.setupHttpRetry();

        // Listen for configuration changes
        vscode.workspace.onDidChangeConfiguration((e: vscode.ConfigurationChangeEvent) => {
            if (e.affectsConfiguration('mockforge')) {
                this.loadConfiguration();
                // Update HTTP timeout if changed
                const newTimeout = config.get<number>('http.timeout', 5000);
                this.http.defaults.timeout = newTimeout;
            }
        });
    }

    /**
     * Setup HTTP retry logic with exponential backoff
     */
    private setupHttpRetry(): void {
        const config = vscode.workspace.getConfiguration('mockforge');
        const retryAttempts = config.get<number>('http.retryAttempts', 3);
        const retryDelay = config.get<number>('http.retryDelay', 1000);

        // Request interceptor (currently just passes through)
        this.http.interceptors.request.use(
            (config: InternalAxiosRequestConfig) => config,
            (error: AxiosError) => Promise.reject(error)
        );

        // Response interceptor with retry logic
        this.http.interceptors.response.use(
            (response: AxiosResponse) => response,
            async (error: AxiosError) => {
                const config = error.config;

                // Don't retry if config doesn't exist
                if (!config) {
                    return Promise.reject(error);
                }

                // Track retry count on the config object (extend type for retry tracking)
                interface ExtendedAxiosRequestConfig extends InternalAxiosRequestConfig {
                    __retryCount?: number;
                }
                const extendedConfig = config as ExtendedAxiosRequestConfig;
                const retryCount = extendedConfig.__retryCount || 0;

                // Check if we should retry (only retry on server errors)
                const shouldRetry =
                    retryCount < retryAttempts &&
                    error.response &&
                    error.response.status >= 500;

                if (shouldRetry) {
                    extendedConfig.__retryCount = retryCount + 1;

                    // Calculate delay with exponential backoff
                    const delay = retryDelay * Math.pow(2, retryCount);

                    Logger.debug(`HTTP request failed, retrying in ${delay}ms (attempt ${retryCount + 1}/${retryAttempts})`);

                    // Wait before retrying
                    await new Promise(resolve => setTimeout(resolve, delay));

                    return this.http(extendedConfig);
                }

                return Promise.reject(error);
            }
        );
    }

    /**
     * Load reconnection configuration from VS Code settings
     */
    private loadConfiguration(): void {
        const config = vscode.workspace.getConfiguration('mockforge');
        this.reconnectEnabled = config.get<boolean>('reconnect.enabled', true);
        this.reconnectInitialDelay = config.get<number>('reconnect.initialDelay', 1000);
        this.reconnectMaxDelay = config.get<number>('reconnect.maxDelay', 30000);
        this.reconnectMaxRetries = config.get<number>('reconnect.maxRetries', 10);
    }

    /**
     * Get current connection state
     */
    get connectionState(): ConnectionState {
        return this._connectionState;
    }

    /**
     * Set connection state and notify listeners
     */
    private setConnectionState(state: ConnectionState): void {
        if (this._connectionState !== state) {
            this._connectionState = state;
            this.stateListeners.forEach(listener => listener(state));
        }
    }

    /**
     * Register a listener for connection state changes
     */
    onStateChange(listener: (state: ConnectionState) => void): void {
        this.stateListeners.push(listener);
    }

    /**
     * Connect to MockForge server (HTTP and WebSocket)
     */
    async connect(): Promise<void> {
        // Test HTTP connection first
        try {
            await this.http.get('/health');
        } catch (error) {
            this.setConnectionState('disconnected');
            throw new Error(`Failed to connect to MockForge server: ${error instanceof Error ? error.message : 'Unknown error'}`);
        }

        // Reset manual disconnect flag
        this.manualDisconnect = false;
        this.reconnectAttempts = 0;

        // Connect WebSocket
        await this.connectWebSocket();
    }

    /**
     * Connect WebSocket with event handlers
     */
    private async connectWebSocket(): Promise<void> {
        // Clear any existing reconnection timer
        if (this.reconnectTimer) {
            clearTimeout(this.reconnectTimer);
            this.reconnectTimer = undefined;
        }

        // Close existing connection if any
        if (this.ws) {
            this.ws.removeAllListeners();
            this.ws.close();
        }

        this.setConnectionState('connecting');

        const wsUrl = this.serverUrl.replace('http', 'ws') + '/__mockforge/ws';
        this.ws = new WebSocket(wsUrl);

        this.ws.on('open', () => {
            Logger.info('WebSocket connected');
            this.setConnectionState('connected');
            this.reconnectAttempts = 0; // Reset on successful connection
        });

        this.ws.on('message', (data: WebSocket.Data) => {
            try {
                const event = JSON.parse(data.toString()) as MockEvent;
                // Validate event has required type field
                if (event && typeof event.type === 'string') {
                    this.listeners.forEach(listener => listener(event));
                } else {
                    Logger.warn('Received invalid WebSocket event:', event);
                }
            } catch (error) {
                Logger.error('Failed to parse WebSocket message:', error);
            }
        });

        this.ws.on('error', (error: Error) => {
            Logger.error('WebSocket error:', error);
            // Don't change state on error - let close handler handle it
        });

        this.ws.on('close', (code: number, reason: Buffer) => {
            Logger.info(`WebSocket disconnected (code: ${code}, reason: ${reason.toString()})`);
            this.setConnectionState('disconnected');

            // Attempt reconnection if not manually disconnected and reconnection is enabled
            if (!this.manualDisconnect && this.reconnectEnabled) {
                this.attemptReconnect();
            }
        });
    }

    /**
     * Attempt to reconnect with exponential backoff
     */
    private attemptReconnect(): void {
        // Check if we've exceeded max retries
        if (this.reconnectMaxRetries > 0 && this.reconnectAttempts >= this.reconnectMaxRetries) {
            Logger.warn(`Max reconnection attempts (${this.reconnectMaxRetries}) reached. Stopping reconnection.`);
            return;
        }

        this.reconnectAttempts++;
        this.setConnectionState('reconnecting');

        // Calculate delay with exponential backoff
        const delay = Math.min(
            this.reconnectInitialDelay * Math.pow(2, this.reconnectAttempts - 1),
            this.reconnectMaxDelay
        );

        Logger.info(`Attempting to reconnect in ${delay}ms (attempt ${this.reconnectAttempts}/${this.reconnectMaxRetries || '∞'})`);

        this.reconnectTimer = setTimeout(async () => {
            try {
                await this.connectWebSocket();
            } catch (error) {
                Logger.error('Reconnection attempt failed:', error);
                // Will trigger another reconnection attempt via close handler
            }
        }, delay);
    }

    /**
     * Disconnect from MockForge server
     */
    disconnect(): void {
        this.manualDisconnect = true;

        // Clear reconnection timer
        if (this.reconnectTimer) {
            clearTimeout(this.reconnectTimer);
            this.reconnectTimer = undefined;
        }

        // Close WebSocket
        if (this.ws) {
            this.ws.removeAllListeners();
            this.ws.close();
            this.ws = undefined;
        }

        this.setConnectionState('disconnected');
        this.reconnectAttempts = 0;
    }

    /**
     * Register a listener for WebSocket events
     * @param listener Callback function that receives typed MockEvent
     */
    onEvent(listener: (event: MockEvent) => void): void {
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
        // Server expects an 'id' field (can be empty, server will generate one)
        // Include empty string to satisfy deserialization
        const mockWithId: MockConfig = {
            ...mock,
            id: ''
        };
        const response = await this.http.post('/mocks', mockWithId);
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

    /**
     * Import mocks from JSON or YAML file content
     * @param data File content as string (JSON or YAML)
     * @param format File format: 'json' or 'yaml'
     * @param merge If true, merge with existing mocks; if false, replace all mocks
     */
    async importMocks(data: string, format: string, merge: boolean): Promise<void> {
        let mocks: MockConfig[];

        try {
            if (format === 'json' || format === 'yaml' || format === 'yml') {
                // Parse the file content into MockConfig array
                if (format === 'json') {
                    mocks = JSON.parse(data);
                } else {
                    // For YAML, we'd need a YAML parser library
                    // For now, try to parse as JSON first (some YAML files are valid JSON)
                    try {
                        mocks = JSON.parse(data);
                    } catch {
                        // If JSON parsing fails, we need a YAML parser
                        // Since we don't have one, throw an informative error
                        throw new Error('YAML parsing not yet supported. Please use JSON format or install a YAML parser.');
                    }
                }

                // Validate that we have an array
                if (!Array.isArray(mocks)) {
                    throw new Error('Invalid file format: expected an array of mock configurations');
                }

                // Send as JSON array with proper Content-Type
                await this.http.post(`/import?format=${format}&merge=${merge}`, mocks, {
                    // eslint-disable-next-line @typescript-eslint/naming-convention
                    headers: { 'Content-Type': 'application/json' }
                });
            } else {
                throw new Error(`Unsupported format: ${format}. Supported formats: json, yaml`);
            }
        } catch (error) {
            if (error instanceof Error) {
                throw new Error(`Failed to import mocks: ${error.message}`);
            }
            throw error;
        }
    }
}
