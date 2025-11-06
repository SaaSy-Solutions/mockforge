/**
 * Service Worker Interceptor
 * 
 * Registers a service worker to intercept all network requests
 * for comprehensive request capture
 */

import { CapturedRequest } from '../types';
import { analyzeRequest } from '../utils/requestAnalyzer';

/**
 * Callback type for captured requests
 */
export type ServiceWorkerCaptureCallback = (request: CapturedRequest) => void | Promise<void>;

/**
 * Service Worker interceptor for comprehensive request capture
 */
export class ServiceWorkerInterceptor {
    private registration: ServiceWorkerRegistration | null = null;
    private captureCallback?: ServiceWorkerCaptureCallback;
    private serviceWorkerUrl: string;
    private enabled: boolean = false;

    constructor(serviceWorkerUrl: string = '/forgeconnect-sw.js') {
        this.serviceWorkerUrl = serviceWorkerUrl;
    }

    /**
     * Register service worker and start intercepting
     */
    async start(callback: ServiceWorkerCaptureCallback): Promise<boolean> {
        if (this.enabled) {
            return true;
        }

        if (!('serviceWorker' in navigator)) {
            console.warn('[ForgeConnect] Service Workers not supported in this browser');
            return false;
        }

        try {
            // Register service worker
            this.registration = await navigator.serviceWorker.register(this.serviceWorkerUrl, {
                scope: '/',
            });

            // Wait for service worker to be ready
            await navigator.serviceWorker.ready;

            // Set up message listener
            navigator.serviceWorker.addEventListener('message', (event) => {
                if (event.data && event.data.type === 'FORGECONNECT_REQUEST') {
                    const request: CapturedRequest = event.data.payload;
                    callback(request).catch(err => {
                        console.warn('[ForgeConnect] Error in capture callback:', err);
                    });
                }
            });

            this.captureCallback = callback;
            this.enabled = true;

            console.log('[ForgeConnect] Service Worker registered successfully');
            return true;
        } catch (error) {
            console.error('[ForgeConnect] Failed to register Service Worker:', error);
            return false;
        }
    }

    /**
     * Stop intercepting and unregister service worker
     */
    async stop(): Promise<void> {
        if (!this.enabled || !this.registration) {
            return;
        }

        try {
            await this.registration.unregister();
            this.registration = null;
            this.captureCallback = undefined;
            this.enabled = false;
            console.log('[ForgeConnect] Service Worker unregistered');
        } catch (error) {
            console.error('[ForgeConnect] Failed to unregister Service Worker:', error);
        }
    }

    /**
     * Check if service worker is enabled
     */
    isEnabled(): boolean {
        return this.enabled;
    }

    /**
     * Get service worker registration
     */
    getRegistration(): ServiceWorkerRegistration | null {
        return this.registration;
    }
}

/**
 * Generate service worker script content
 * This should be served as a static file or injected into the page
 */
export function generateServiceWorkerScript(): string {
    return `
// ForgeConnect Service Worker
// Intercepts all network requests

self.addEventListener('install', (event) => {
    self.skipWaiting();
});

self.addEventListener('activate', (event) => {
    event.waitUntil(self.clients.claim());
});

self.addEventListener('fetch', (event) => {
    const request = event.request;
    const url = new URL(request.url);

    // Skip chrome-extension and other non-HTTP(S) URLs
    if (!url.protocol.startsWith('http')) {
        return;
    }

    // Clone request for analysis
    const clonedRequest = request.clone();

    // Forward request
    event.respondWith(
        fetch(request)
            .then((response) => {
                // Clone response for analysis
                const clonedResponse = response.clone();

                // Analyze request/response asynchronously
                analyzeRequestAsync(clonedRequest, clonedResponse)
                    .then((captured) => {
                        // Send to main thread
                        self.clients.matchAll().then((clients) => {
                            clients.forEach((client) => {
                                client.postMessage({
                                    type: 'FORGECONNECT_REQUEST',
                                    payload: captured,
                                });
                            });
                        });
                    })
                    .catch((err) => {
                        console.warn('[ForgeConnect SW] Failed to analyze request:', err);
                    });

                return response;
            })
            .catch((error) => {
                // Network error - capture it
                analyzeRequestAsync(clonedRequest, null, error)
                    .then((captured) => {
                        self.clients.matchAll().then((clients) => {
                            clients.forEach((client) => {
                                client.postMessage({
                                    type: 'FORGECONNECT_REQUEST',
                                    payload: captured,
                                });
                            });
                        });
                    });

                throw error;
            })
    );
});

async function analyzeRequestAsync(request, response, error) {
    const method = request.method;
    const url = request.url;
    const headers = {};
    
    // Extract headers
    request.headers.forEach((value, key) => {
        headers[key] = value;
    });

    // Extract body if available
    let body;
    try {
        if (request.body) {
            const text = await request.clone().text();
            const contentType = headers['content-type'] || headers['Content-Type'] || '';
            if (contentType.includes('application/json')) {
                body = JSON.parse(text);
            } else {
                body = text;
            }
        }
    } catch {
        // Ignore body parsing errors
    }

    const urlObj = new URL(url);
    const path = urlObj.pathname;
    const queryParams = {};
    urlObj.searchParams.forEach((value, key) => {
        queryParams[key] = value;
    });

    let statusCode;
    let responseBody;
    let responseHeaders = {};

    if (response) {
        statusCode = response.status;
        response.headers.forEach((value, key) => {
            responseHeaders[key] = value;
        });

        try {
            const contentType = response.headers.get('content-type') || '';
            if (contentType.includes('application/json')) {
                responseBody = await response.clone().json();
            } else {
                responseBody = await response.clone().text();
            }
        } catch {
            // Ignore response parsing errors
        }
    }

    const captured = {
        method: method.toUpperCase(),
        url: url,
        path: path,
        queryParams: Object.keys(queryParams).length > 0 ? queryParams : undefined,
        headers: Object.keys(headers).length > 0 ? headers : undefined,
        body: body,
        statusCode: statusCode,
        responseBody: responseBody,
        responseHeaders: Object.keys(responseHeaders).length > 0 ? responseHeaders : undefined,
        timestamp: Date.now(),
    };

    if (error) {
        captured.error = {
            type: 'network',
            message: error.message || 'Network error',
        };
    }

    return captured;
}
`.trim();
}

