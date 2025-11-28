/**
 * Offline Mock Server
 *
 * Service Worker-based local mock server for offline mode
 */

import { MockConfig } from '../types';
import { OfflineStorage } from './OfflineStorage';

/**
 * Offline Mock Server
 * Serves cached mocks via Service Worker when MockForge is unavailable
 */
export class OfflineMockServer {
    private storage: OfflineStorage;
    private serviceWorkerRegistration: ServiceWorkerRegistration | null = null;
    private isEnabled: boolean = false;

    constructor(storage: OfflineStorage) {
        this.storage = storage;
    }

    /**
     * Enable offline mock server
     */
    async enable(): Promise<void> {
        if (this.isEnabled) {
            return;
        }

        if (!('serviceWorker' in navigator)) {
            throw new Error('Service Workers are not supported in this browser');
        }

        try {
            // Generate and register Service Worker
            const swScript = OfflineMockServer.generateServiceWorkerScript();
            const swBlob = new Blob([swScript], { type: 'application/javascript' });
            const swUrl = URL.createObjectURL(swBlob);

            // Register Service Worker for offline mock serving
            const registration = await navigator.serviceWorker.register(swUrl, {
                scope: '/',
            });

            await navigator.serviceWorker.ready;
            this.serviceWorkerRegistration = registration;
            this.isEnabled = true;

            // Update cached mocks
            await this.updateCachedMocks();

            // Notify Service Worker to enable offline mode
            if (registration.active) {
                registration.active.postMessage({
                    type: 'ENABLE_OFFLINE_MODE',
                });
            }

            // Clean up blob URL after registration
            URL.revokeObjectURL(swUrl);
        } catch (error) {
            console.error('[OfflineMockServer] Failed to register Service Worker:', error);
            // Try fallback: register from a known path if blob registration fails
            try {
                const registration = await navigator.serviceWorker.register('/forgeconnect-offline-sw.js', {
                    scope: '/',
                });
                await navigator.serviceWorker.ready;
                this.serviceWorkerRegistration = registration;
                this.isEnabled = true;
                await this.updateCachedMocks();
            } catch (fallbackError) {
                console.error('[OfflineMockServer] Fallback registration also failed:', fallbackError);
                throw error; // Throw original error
            }
        }
    }

    /**
     * Disable offline mock server
     */
    async disable(): Promise<void> {
        if (!this.isEnabled) {
            return;
        }

        // Notify Service Worker to disable offline mode
        if (this.serviceWorkerRegistration?.active) {
            this.serviceWorkerRegistration.active.postMessage({
                type: 'DISABLE_OFFLINE_MODE',
            });
        }

        this.isEnabled = false;
    }

    /**
     * Check if offline mode is enabled
     */
    isOfflineModeEnabled(): boolean {
        return this.isEnabled;
    }

    /**
     * Update cached mocks in Service Worker
     */
    async updateCachedMocks(environmentId?: string): Promise<void> {
        if (!this.isEnabled || !this.serviceWorkerRegistration?.active) {
            return;
        }

        const mocks = await this.storage.getAllCachedMocks(environmentId);

        this.serviceWorkerRegistration.active.postMessage({
            type: 'UPDATE_MOCKS',
            payload: { mocks },
        });
    }

    /**
     * Generate Service Worker script for offline mock serving
     */
    static generateServiceWorkerScript(): string {
        return `
// ForgeConnect Offline Mock Server Service Worker
const CACHE_NAME = 'forgeconnect-mocks-v1';
let cachedMocks = [];

self.addEventListener('install', (event) => {
    self.skipWaiting();
});

self.addEventListener('activate', (event) => {
    event.waitUntil(self.clients.claim());
});

// Listen for messages from main thread
self.addEventListener('message', (event) => {
    if (event.data.type === 'ENABLE_OFFLINE_MODE') {
        // Enable offline mode
    } else if (event.data.type === 'DISABLE_OFFLINE_MODE') {
        // Disable offline mode
    } else if (event.data.type === 'UPDATE_MOCKS') {
        cachedMocks = event.data.payload.mocks || [];
    }
});

// Intercept fetch requests
self.addEventListener('fetch', (event) => {
    const request = event.request;
    const url = new URL(request.url);

    // Skip non-HTTP(S) URLs
    if (!url.protocol.startsWith('http')) {
        return;
    }

    // Skip chrome-extension URLs
    if (url.protocol === 'chrome-extension:') {
        return;
    }

    // Try to find matching mock
    const method = request.method;
    const path = url.pathname;

    const mock = findMatchingMock(method, path);

    if (mock && mock.response) {
        // Serve cached mock
        const responseBody = typeof mock.response.body === 'string'
            ? mock.response.body
            : JSON.stringify(mock.response.body);

        const headers = new Headers({
            'Content-Type': 'application/json',
            'X-MockForge-Offline': 'true',
            ...(mock.response.headers || {}),
        });

        // Add CORS headers
        if (!headers.has('Access-Control-Allow-Origin')) {
            headers.set('Access-Control-Allow-Origin', '*');
        }

        const statusCode = mock.status_code || 200;

        event.respondWith(
            new Response(responseBody, {
                status: statusCode,
                statusText: getStatusText(statusCode),
                headers,
            })
        );
        return;
    }

    // No matching mock, proceed with original request
    // (will fail if offline, which is expected)
});

function findMatchingMock(method, path) {
    // Find exact match first
    let match = cachedMocks.find(
        (m) => m.method.toUpperCase() === method.toUpperCase() && m.path === path && m.enabled !== false
    );

    if (match) {
        return match;
    }

    // Try path pattern matching
    match = cachedMocks.find((m) => {
        if (m.method.toUpperCase() !== method.toUpperCase() || m.enabled === false) {
            return false;
        }

        // Convert path pattern to regex
        const pattern = m.path.replace(/\\*/g, '.*').replace(/\\{([^}]+)\\}/g, '[^/]+');
        const regex = new RegExp(\`^\${pattern}$\`);
        return regex.test(path);
    });

    return match || null;
}

function getStatusText(status) {
    const statusTexts = {
        200: 'OK',
        201: 'Created',
        204: 'No Content',
        400: 'Bad Request',
        401: 'Unauthorized',
        403: 'Forbidden',
        404: 'Not Found',
        500: 'Internal Server Error',
        502: 'Bad Gateway',
        503: 'Service Unavailable',
    };
    return statusTexts[status] || 'OK';
}
`.trim();
    }
}
