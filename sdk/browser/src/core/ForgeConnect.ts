/**
 * ForgeConnect - Main SDK Class
 *
 * Provides browser-based mock creation and management for MockForge
 */

import { ForgeConnectConfig, MockConfig, CapturedRequest, ConnectionStatus, Environment } from '../types';
import { MockForgeClient } from './MockForgeClient';
import { RequestInterceptor } from './RequestInterceptor';
import { ServiceWorkerInterceptor } from './ServiceWorkerInterceptor';
import { WebSocketClient } from './WebSocketClient';
import { EnvironmentManager } from './EnvironmentManager';
import { OfflineStorage } from './OfflineStorage';
import { SyncQueue } from './SyncQueue';
import { OfflineMockServer } from './OfflineMockServer';
import { OAuthPassthrough } from './OAuthPassthrough';
import { generateMockResponse, generateMockName } from '../utils/responseGenerator';
import { shouldCreateMock } from '../utils/requestAnalyzer';

/**
 * Main ForgeConnect SDK class
 */
export class ForgeConnect {
    private config: Required<Pick<ForgeConnectConfig, 'mockMode' | 'autoMockStatusCodes' | 'autoMockNetworkErrors' | 'enableLogging'>> & ForgeConnectConfig;
    private client?: MockForgeClient;
    private interceptor: RequestInterceptor;
    private serviceWorkerInterceptor?: ServiceWorkerInterceptor;
    private websocketClient?: WebSocketClient;
    private environmentManager?: EnvironmentManager;
    private offlineStorage?: OfflineStorage;
    private syncQueue?: SyncQueue;
    private offlineMockServer?: OfflineMockServer;
    private oauthPassthrough?: OAuthPassthrough;
    private connectionStatus: ConnectionStatus = { connected: false };
    private offlineMode: boolean = false;
    private discoveryPorts: number[] = [3000, 3001, 8080, 9080];
    private discoveryTimeout: number = 2000; // 2 seconds per port
    private useServiceWorker: boolean = false;
    private useWebSocket: boolean = false;

    constructor(config: ForgeConnectConfig = {}) {
        this.config = {
            mockMode: config.mockMode || 'hybrid',
            autoMockStatusCodes: config.autoMockStatusCodes || [404, 500, 502, 503, 504],
            autoMockNetworkErrors: config.autoMockNetworkErrors !== false,
            enableLogging: config.enableLogging !== false,
            ...config,
        };

        this.interceptor = new RequestInterceptor();
        this.interceptor.configureAutoMock({
            statusCodes: this.config.autoMockStatusCodes,
            networkErrors: this.config.autoMockNetworkErrors,
        });

        if (config.discoveryPorts) {
            this.discoveryPorts = config.discoveryPorts;
        }

        // Enable Service Worker if supported and requested
        this.useServiceWorker = config.enableServiceWorker !== false && 'serviceWorker' in navigator;

        // Enable WebSocket if requested
        this.useWebSocket = config.enableWebSocket === true;
    }

    /**
     * Initialize ForgeConnect and connect to MockForge
     */
    async initialize(): Promise<boolean> {
        // Auto-discover MockForge server if URL not provided
        if (!this.config.serverUrl) {
            const discoveredUrl = await this.discoverMockForge();
            if (!discoveredUrl) {
                this.log('Could not discover MockForge server. Please provide serverUrl in config.');
                this.updateConnectionStatus(false, undefined, 'Could not discover MockForge server');
                return false;
            }
            this.config.serverUrl = discoveredUrl;
        }

        // Create client and check connection
        this.client = new MockForgeClient(this.config.serverUrl);
        const connected = await this.client.healthCheck();

        this.updateConnectionStatus(connected, this.config.serverUrl);

        if (connected) {
            // Initialize environment manager
            this.environmentManager = new EnvironmentManager(this.client, this.config.workspaceId);
            await this.environmentManager.initialize();

            // Initialize offline storage and sync queue
            this.offlineStorage = new OfflineStorage();
            await this.offlineStorage.initialize();
            this.syncQueue = new SyncQueue(this.offlineStorage, this.client);

            // Initialize offline mock server
            this.offlineMockServer = new OfflineMockServer(this.offlineStorage);

            // Initialize OAuth passthrough if configured
            if (this.config.oauthPassthrough) {
                this.oauthPassthrough = new OAuthPassthrough(this.config.oauthPassthrough);
            }

            // Cache existing mocks
            await this.cacheAllMocks();

            // Set OAuth passthrough on interceptor if configured
            if (this.oauthPassthrough) {
                this.interceptor.setOAuthPassthrough(this.oauthPassthrough);
            }

            // Start intercepting requests
            this.interceptor.start((request) => this.handleCapturedRequest(request));

            // Start Service Worker if enabled
            if (this.useServiceWorker) {
                this.serviceWorkerInterceptor = new ServiceWorkerInterceptor();
                await this.serviceWorkerInterceptor.start((request) => this.handleCapturedRequest(request));
            }

            // Connect WebSocket if enabled
            if (this.useWebSocket && this.config.serverUrl) {
                this.websocketClient = new WebSocketClient(this.config.serverUrl);
                const wsConnected = await this.websocketClient.connect();

                if (wsConnected) {
                    // Set up WebSocket event listeners
                    this.websocketClient.on('mock_created', (event) => {
                        // MockForge sends: { type: "mock_created", mock: {...}, timestamp: "..." }
                        const mock = event.payload.mock || event.payload;
                        if (this.config.onMockCreated && mock) {
                            this.config.onMockCreated(mock);
                        }
                        this.log(`Mock created via WebSocket: ${mock?.id || mock?.name}`);
                    });

                    this.websocketClient.on('mock_updated', (event) => {
                        const mock = event.payload.mock || event.payload;
                        this.log(`Mock updated via WebSocket: ${mock?.id || mock?.name}`);

                        // Trigger live reload callback if configured
                        if (this.config.onMockUpdated) {
                            this.config.onMockUpdated(mock);
                        }
                    });

                    this.websocketClient.on('mock_deleted', (event) => {
                        const id = event.payload.id || event.payload;
                        this.log(`Mock deleted via WebSocket: ${id}`);

                        // Trigger live reload callback if configured
                        if (this.config.onMockDeleted) {
                            this.config.onMockDeleted(id);
                        }
                    });

                    this.websocketClient.on('stats_updated', (event) => {
                        this.log('Server stats updated via WebSocket');
                    });

                    this.websocketClient.on('connection_status', (event) => {
                        const connected = event.payload.connected !== false;
                        this.updateConnectionStatus(connected, this.config.serverUrl);
                    });
                }
            }
        }

        return connected;
    }

    /**
     * Auto-discover MockForge server on localhost
     */
    private async discoverMockForge(): Promise<string | null> {
        const baseUrl = 'http://localhost';

        for (const port of this.discoveryPorts) {
            const url = `${baseUrl}:${port}`;
            this.log(`Trying to connect to ${url}...`);

            const client = new MockForgeClient(url);
            const connected = await client.healthCheck();

            if (connected) {
                this.log(`Connected to MockForge at ${url}`);
                return url;
            }
        }

        return null;
    }

    /**
     * Handle a captured request
     */
    private async handleCapturedRequest(request: CapturedRequest): Promise<void> {
        // Skip OAuth passthrough requests
        if (this.oauthPassthrough?.shouldBypass(request.url, request.method)) {
            return;
        }
        if (!this.client || !this.connectionStatus.connected) {
            return;
        }

        const shouldAutoMock = shouldCreateMock(
            request,
            this.config.autoMockStatusCodes,
            this.config.autoMockNetworkErrors
        );

        // Determine action based on mock mode
        let shouldCreate = false;

        switch (this.config.mockMode) {
            case 'auto':
                shouldCreate = shouldAutoMock;
                break;

            case 'prompt':
                if (this.config.promptMockCreation) {
                    shouldCreate = await this.config.promptMockCreation(request);
                } else {
                    // Default prompt: auto-create for failed requests
                    shouldCreate = shouldAutoMock;
                }
                break;

            case 'hybrid':
                // Auto-create for failed requests, prompt for others
                if (shouldAutoMock) {
                    shouldCreate = true;
                } else if (this.config.promptMockCreation) {
                    shouldCreate = await this.config.promptMockCreation(request);
                }
                break;
        }

        if (shouldCreate) {
            await this.createMockFromRequest(request);
        }
    }

    /**
     * Create a mock from a captured request
     */
    async createMockFromRequest(request: CapturedRequest): Promise<MockConfig | null> {
        if (!this.client) {
            throw new Error('ForgeConnect not initialized. Call initialize() first.');
        }

        try {
            const mockResponse = generateMockResponse(request);
            const mockName = generateMockName(request);

            const mock: MockConfig = {
                id: '', // Server will generate ID
                name: mockName,
                method: request.method,
                path: request.path,
                response: mockResponse,
                enabled: true,
                status_code: request.statusCode || (request.error ? 502 : 200),
            };

            // Try to create on server
            let created: MockConfig;
            try {
                created = await this.client.createMock(mock);

                // Cache the mock
                if (this.offlineStorage) {
                    const activeEnv = this.environmentManager?.getActiveEnvironment();
                    await this.offlineStorage.cacheMock(created, activeEnv?.id);
                }
            } catch (error) {
                // If offline, add to sync queue and cache locally
                if (this.offlineStorage && this.syncQueue) {
                    const queueId = await this.syncQueue.enqueue('create', mock);
                    // Generate a temporary ID for offline mock
                    mock.id = `offline_${queueId}`;
                    const activeEnv = this.environmentManager?.getActiveEnvironment();
                    await this.offlineStorage.cacheMock(mock, activeEnv?.id);

                    // Enable offline mode if not already enabled
                    if (!this.offlineMode && this.offlineMockServer) {
                        await this.enableOfflineMode();
                    }

                    this.log(`Created mock offline (queued for sync): ${mockName}`);
                    return mock;
                }
                throw error;
            }

            this.log(`Created mock: ${created.id} - ${mockName}`);

            if (this.config.onMockCreated) {
                this.config.onMockCreated(created);
            }

            return created;
        } catch (error) {
            this.log(`Failed to create mock: ${error instanceof Error ? error.message : 'Unknown error'}`);
            throw error;
        }
    }

    /**
     * Manually create a mock
     */
    async createMock(mock: MockConfig): Promise<MockConfig> {
        if (!this.client) {
            throw new Error('ForgeConnect not initialized. Call initialize() first.');
        }

        return await this.client.createMock(mock);
    }

    /**
     * List all mocks
     */
    async listMocks(): Promise<MockConfig[]> {
        if (!this.client) {
            throw new Error('ForgeConnect not initialized. Call initialize() first.');
        }

        return await this.client.listMocks();
    }

    /**
     * Get a mock by ID
     */
    async getMock(id: string): Promise<MockConfig> {
        if (!this.client) {
            throw new Error('ForgeConnect not initialized. Call initialize() first.');
        }

        return await this.client.getMock(id);
    }

    /**
     * Update a mock
     */
    async updateMock(id: string, mock: MockConfig): Promise<MockConfig> {
        if (!this.client) {
            throw new Error('ForgeConnect not initialized. Call initialize() first.');
        }

        try {
            const updated = await this.client.updateMock(id, mock);

            // Update cache
            if (this.offlineStorage) {
                const activeEnv = this.environmentManager?.getActiveEnvironment();
                await this.offlineStorage.cacheMock(updated, activeEnv?.id);
            }

            return updated;
        } catch (error) {
            // If offline, add to sync queue
            if (this.offlineStorage && this.syncQueue) {
                await this.syncQueue.enqueue('update', mock, id);
                const activeEnv = this.environmentManager?.getActiveEnvironment();
                await this.offlineStorage.cacheMock(mock, activeEnv?.id);

                if (!this.offlineMode && this.offlineMockServer) {
                    await this.enableOfflineMode();
                }

                return mock;
            }
            throw error;
        }
    }

    /**
     * Delete a mock
     */
    async deleteMock(id: string): Promise<void> {
        if (!this.client) {
            throw new Error('ForgeConnect not initialized. Call initialize() first.');
        }

        try {
            await this.client.deleteMock(id);

            // Remove from cache
            if (this.offlineStorage) {
                await this.offlineStorage.deleteCachedMock(id);
            }
        } catch (error) {
            // If offline, add to sync queue
            if (this.offlineStorage && this.syncQueue) {
                await this.syncQueue.enqueue('delete', null, id);
                await this.offlineStorage.deleteCachedMock(id);

                if (!this.offlineMode && this.offlineMockServer) {
                    await this.enableOfflineMode();
                }
            } else {
                throw error;
            }
        }
    }

    /**
     * Get connection status
     */
    getConnectionStatus(): ConnectionStatus {
        return { ...this.connectionStatus };
    }

    /**
     * List all environments
     */
    async listEnvironments(): Promise<Environment[]> {
        if (!this.environmentManager) {
            throw new Error('ForgeConnect not initialized. Call initialize() first.');
        }
        return this.environmentManager.getEnvironments();
    }

    /**
     * Get the active environment
     */
    async getActiveEnvironment(): Promise<Environment | null> {
        if (!this.environmentManager) {
            throw new Error('ForgeConnect not initialized. Call initialize() first.');
        }
        return this.environmentManager.getActiveEnvironment();
    }

    /**
     * Set the active environment
     */
    async setActiveEnvironment(environmentId: string): Promise<void> {
        if (!this.environmentManager) {
            throw new Error('ForgeConnect not initialized. Call initialize() first.');
        }
        await this.environmentManager.setActiveEnvironment(environmentId);
    }

    /**
     * Get environment variables for an environment
     */
    async getEnvironmentVariables(environmentId: string): Promise<Record<string, string>> {
        if (!this.environmentManager) {
            throw new Error('ForgeConnect not initialized. Call initialize() first.');
        }
        return this.environmentManager.getEnvironmentVariables(environmentId);
    }

    /**
     * Get variables for the active environment
     */
    async getActiveEnvironmentVariables(): Promise<Record<string, string>> {
        if (!this.environmentManager) {
            throw new Error('ForgeConnect not initialized. Call initialize() first.');
        }
        return this.environmentManager.getActiveEnvironmentVariables();
    }

    /**
     * Get the environment manager instance
     */
    getEnvironmentManager(): EnvironmentManager | undefined {
        return this.environmentManager;
    }

    /**
     * Enable live reload with callbacks
     * Sets up automatic refresh when mocks are updated/deleted
     */
    enableLiveReload(onMockUpdated?: (mock: MockConfig) => void, onMockDeleted?: (mockId: string) => void): void {
        this.config.onMockUpdated = onMockUpdated;
        this.config.onMockDeleted = onMockDeleted;

        // If WebSocket is not enabled, enable it for live reload
        if (!this.useWebSocket && this.config.serverUrl) {
            this.useWebSocket = true;
            this.websocketClient = new WebSocketClient(this.config.serverUrl);
            this.websocketClient.connect().then((connected) => {
                if (connected) {
                    this.setupWebSocketLiveReload();
                }
            });
        } else if (this.websocketClient && this.websocketClient.isConnected()) {
            this.setupWebSocketLiveReload();
        }
    }

    /**
     * Set up WebSocket listeners for live reload
     */
    private setupWebSocketLiveReload(): void {
        if (!this.websocketClient) return;

        this.websocketClient.on('mock_updated', (event) => {
            const mock = event.payload.mock || event.payload;
            if (this.config.onMockUpdated) {
                this.config.onMockUpdated(mock);
            }
        });

        this.websocketClient.on('mock_deleted', (event) => {
            const id = event.payload.id || event.payload;
            if (this.config.onMockDeleted) {
                this.config.onMockDeleted(id);
            }
        });
    }

    /**
     * Reconnect to MockForge
     */
    async reconnect(): Promise<boolean> {
        if (this.client) {
            const connected = await this.client.healthCheck();
            this.updateConnectionStatus(connected, this.client.getBaseUrl());

            if (connected) {
                // Sync pending operations
                if (this.syncQueue) {
                    const result = await this.syncQueue.sync();
                    this.log(`Synced ${result.success} operations, ${result.failed} failed`);
                }

                // Refresh cached mocks
                await this.cacheAllMocks();

                // Disable offline mode if enabled
                if (this.offlineMode && this.offlineMockServer) {
                    await this.disableOfflineMode();
                }
            }

            return connected;
        }
        return await this.initialize();
    }

    /**
     * Cache all mocks from server
     */
    private async cacheAllMocks(): Promise<void> {
        if (!this.offlineStorage || !this.client) {
            return;
        }

        try {
            const mocks = await this.client.listMocks();
            const activeEnv = this.environmentManager?.getActiveEnvironment();

            for (const mock of mocks) {
                await this.offlineStorage.cacheMock(mock, activeEnv?.id);
            }

            // Update offline mock server if enabled
            if (this.offlineMode && this.offlineMockServer) {
                await this.offlineMockServer.updateCachedMocks(activeEnv?.id);
            }
        } catch (error) {
            this.log(`Failed to cache mocks: ${error instanceof Error ? error.message : 'Unknown error'}`);
        }
    }

    /**
     * Enable offline mode
     */
    async enableOfflineMode(): Promise<void> {
        if (this.offlineMode || !this.offlineMockServer) {
            return;
        }

        try {
            await this.offlineMockServer.enable();
            this.offlineMode = true;
            this.log('Offline mode enabled');
        } catch (error) {
            this.log(`Failed to enable offline mode: ${error instanceof Error ? error.message : 'Unknown error'}`);
        }
    }

    /**
     * Disable offline mode
     */
    async disableOfflineMode(): Promise<void> {
        if (!this.offlineMode || !this.offlineMockServer) {
            return;
        }

        try {
            await this.offlineMockServer.disable();
            this.offlineMode = false;
            this.log('Offline mode disabled');
        } catch (error) {
            this.log(`Failed to disable offline mode: ${error instanceof Error ? error.message : 'Unknown error'}`);
        }
    }

    /**
     * Check if offline mode is enabled
     */
    isOfflineModeEnabled(): boolean {
        return this.offlineMode;
    }

    /**
     * Get sync queue size
     */
    async getSyncQueueSize(): Promise<number> {
        if (!this.syncQueue) {
            return 0;
        }
        return await this.syncQueue.getQueueSize();
    }

    /**
     * Manually trigger sync
     */
    async sync(): Promise<{ success: number; failed: number }> {
        if (!this.syncQueue) {
            return { success: 0, failed: 0 };
        }
        return await this.syncQueue.sync();
    }

    /**
     * Update connection status and notify callback
     */
    private updateConnectionStatus(connected: boolean, url?: string, error?: string): void {
        this.connectionStatus = {
            connected,
            url,
            error,
            lastConnected: connected ? Date.now() : this.connectionStatus.lastConnected,
        };

        if (this.config.onConnectionChange) {
            this.config.onConnectionChange(connected, url);
        }
    }

    /**
     * Stop intercepting requests
     */
    async stop(): Promise<void> {
        this.interceptor.stop();

        if (this.serviceWorkerInterceptor) {
            await this.serviceWorkerInterceptor.stop();
        }

        if (this.websocketClient) {
            this.websocketClient.disconnect();
        }
    }

    /**
     * Start intercepting requests (if already initialized)
     */
    start(): void {
        if (this.client && this.connectionStatus.connected) {
            this.interceptor.start((request) => this.handleCapturedRequest(request));
        }
    }

    /**
     * Log a message (if logging is enabled)
     */
    private log(message: string): void {
        if (this.config.enableLogging) {
            console.log(`[ForgeConnect] ${message}`);
        }
    }

    /**
     * Get the MockForge client (for advanced usage)
     */
    getClient(): MockForgeClient | undefined {
        return this.client;
    }
}
