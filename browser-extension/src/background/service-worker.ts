/**
 * Background Service Worker
 *
 * Handles communication between content scripts, DevTools, and MockForge
 * Also captures network requests using webRequest API
 */

import { ExtensionMessage, CapturedRequest, ConnectionStatus, Environment } from '../shared/types';
import { MockForgeApiClient } from '../shared/api-client';

// Connection state
let mockForgeUrl: string | null = null;
let apiClient: MockForgeApiClient | null = null;
let connected: boolean = false;

// Request capture state
interface PendingRequest {
    requestId: string;
    method: string;
    url: string;
    path: string;
    queryParams?: Record<string, string>;
    headers?: Record<string, string>;
    body?: any;
    timestamp: number;
    tabId?: number;
}

const pendingRequests = new Map<string, PendingRequest>();
const capturedRequests: CapturedRequest[] = [];
const MAX_CAPTURED_REQUESTS = 1000; // Limit stored requests

// Auto-discover MockForge on startup
const discoveryPorts = [3000, 3001, 8080, 9080];

async function discoverMockForge(): Promise<string | null> {
    for (const port of discoveryPorts) {
        const url = `http://localhost:${port}`;
        const client = new MockForgeApiClient(url);
        if (await client.healthCheck()) {
            return url;
        }
    }
    return null;
}

async function initializeConnection() {
    // Try to get saved URL from storage
    const result = await chrome.storage.local.get(['mockForgeUrl']);
    if (result.mockForgeUrl) {
        mockForgeUrl = result.mockForgeUrl;
    } else {
        // Auto-discover
        mockForgeUrl = await discoverMockForge();
        if (mockForgeUrl) {
            await chrome.storage.local.set({ mockForgeUrl });
        }
    }

    if (mockForgeUrl) {
        apiClient = new MockForgeApiClient(mockForgeUrl);
        connected = await apiClient.healthCheck();
    }

    // Broadcast connection status
    broadcastConnectionStatus();
}

function broadcastConnectionStatus() {
    const status: ConnectionStatus = {
        connected,
        url: mockForgeUrl || undefined,
    };

    // Send to all tabs
    chrome.tabs.query({}, (tabs) => {
        tabs.forEach((tab) => {
            if (tab.id) {
                chrome.tabs.sendMessage(tab.id, {
                    type: 'CONNECTION_CHANGE',
                    payload: status,
                }).catch(() => {
                    // Ignore errors (tab might not have content script)
                });
            }
        });
    });
}

// Listen for messages from content scripts and DevTools
chrome.runtime.onMessage.addListener(
    (message: ExtensionMessage, sender, sendResponse) => {
        (async () => {
            try {
                switch (message.type) {
                    case 'GET_MOCKS':
                        if (!apiClient || !connected) {
                            await initializeConnection();
                        }
                        if (apiClient && connected) {
                            const mocks = await apiClient.listMocks();
                            sendResponse({ success: true, data: mocks });
                        } else {
                            sendResponse({ success: false, error: 'Not connected to MockForge' });
                        }
                        break;

                    case 'CREATE_MOCK':
                        if (!apiClient || !connected) {
                            await initializeConnection();
                        }
                        if (apiClient && connected) {
                            const mock = await apiClient.createMock(message.payload);
                            // Broadcast mock created event for live reload
                            chrome.runtime.sendMessage({
                                type: 'MOCK_CREATED',
                                payload: mock,
                            }).catch(() => {});
                            sendResponse({ success: true, data: mock });
                        } else {
                            sendResponse({ success: false, error: 'Not connected to MockForge' });
                        }
                        break;

                    case 'DELETE_MOCK':
                        if (!apiClient || !connected) {
                            await initializeConnection();
                        }
                        if (apiClient && connected) {
                            await apiClient.deleteMock(message.payload.id);
                            // Broadcast mock deleted event for live reload
                            chrome.runtime.sendMessage({
                                type: 'MOCK_DELETED',
                                payload: { id: message.payload.id },
                            }).catch(() => {});
                            sendResponse({ success: true });
                        } else {
                            sendResponse({ success: false, error: 'Not connected to MockForge' });
                        }
                        break;

                    case 'REQUEST_CAPTURED':
                        // Forward to DevTools if open
                        chrome.runtime.sendMessage(message).catch(() => {
                            // DevTools might not be open
                        });
                        sendResponse({ success: true });
                        break;

                    case 'GET_CAPTURED_REQUESTS':
                        sendResponse({ success: true, data: capturedRequests });
                        break;

                    case 'GET_ENVIRONMENTS':
                        if (!apiClient || !connected) {
                            await initializeConnection();
                        }
                        if (apiClient && connected) {
                            const environments = await apiClient.listEnvironments();
                            sendResponse({ success: true, data: environments });
                        } else {
                            sendResponse({ success: false, error: 'Not connected to MockForge' });
                        }
                        break;

                    case 'SET_ACTIVE_ENVIRONMENT':
                        if (!apiClient || !connected) {
                            await initializeConnection();
                        }
                        if (apiClient && connected) {
                            await apiClient.setActiveEnvironment(undefined, message.payload.environmentId);
                            sendResponse({ success: true });
                        } else {
                            sendResponse({ success: false, error: 'Not connected to MockForge' });
                        }
                        break;

                    case 'GET_ENVIRONMENT_VARIABLES':
                        if (!apiClient || !connected) {
                            await initializeConnection();
                        }
                        if (apiClient && connected) {
                            const variables = await apiClient.getEnvironmentVariables(undefined, message.payload.environmentId);
                            sendResponse({ success: true, data: variables });
                        } else {
                            sendResponse({ success: false, error: 'Not connected to MockForge' });
                        }
                        break;

                    default:
                        sendResponse({ success: false, error: 'Unknown message type' });
                }
            } catch (error) {
                sendResponse({
                    success: false,
                    error: error instanceof Error ? error.message : 'Unknown error',
                });
            }
        })();

        return true; // Keep channel open for async response
    }
);

// Initialize on startup
chrome.runtime.onStartup.addListener(() => {
    initializeConnection();
});

chrome.runtime.onInstalled.addListener(() => {
    initializeConnection();
});

// Initialize immediately
initializeConnection();

/**
 * Capture network requests using webRequest API
 */

// Extract query parameters from URL
function extractQueryParams(url: string): Record<string, string> {
    const urlObj = new URL(url);
    const params: Record<string, string> = {};
    urlObj.searchParams.forEach((value, key) => {
        params[key] = value;
    });
    return Object.keys(params).length > 0 ? params : undefined;
}

// Extract path from URL
function extractPath(url: string): string {
    try {
        const urlObj = new URL(url);
        return urlObj.pathname;
    } catch {
        return url;
    }
}

// Parse request body
function parseRequestBody(details: chrome.webRequest.WebRequestBodyDetails): any {
    if (!details.requestBody) {
        return undefined;
    }

    try {
        if (details.requestBody.formData) {
            // Form data
            const formData: Record<string, string> = {};
            for (const [key, values] of Object.entries(details.requestBody.formData)) {
                formData[key] = values[0];
            }
            return formData;
        } else if (details.requestBody.raw) {
            // Raw body (binary)
            const decoder = new TextDecoder('utf-8');
            const body = details.requestBody.raw[0]?.bytes;
            if (body) {
                const text = decoder.decode(body);
                // Try to parse as JSON
                try {
                    return JSON.parse(text);
                } catch {
                    return text;
                }
            }
        }
    } catch (error) {
        console.warn('[ForgeConnect] Failed to parse request body:', error);
    }

    return undefined;
}

// Capture request before it's sent
chrome.webRequest.onBeforeRequest.addListener(
    (details: chrome.webRequest.WebRequestBodyDetails) => {
        // Skip chrome-extension and other non-HTTP(S) URLs
        if (!details.url.startsWith('http://') && !details.url.startsWith('https://')) {
            return;
        }

        const method = details.method;
        const url = details.url;
        const path = extractPath(url);
        const queryParams = extractQueryParams(url);
        const body = parseRequestBody(details);

        // Extract headers
        const headers: Record<string, string> = {};
        if (details.requestHeaders) {
            details.requestHeaders.forEach((header) => {
                headers[header.name.toLowerCase()] = header.value || '';
            });
        }

        const pendingRequest: PendingRequest = {
            requestId: details.requestId,
            method,
            url,
            path,
            queryParams,
            headers: Object.keys(headers).length > 0 ? headers : undefined,
            body,
            timestamp: details.timeStamp,
            tabId: details.tabId,
        };

        pendingRequests.set(details.requestId, pendingRequest);
    },
    { urls: ['<all_urls>'] },
    ['requestBody']
);

// Capture response when request completes
chrome.webRequest.onCompleted.addListener(
    async (details: chrome.webRequest.WebResponseDetails) => {
        const pendingRequest = pendingRequests.get(details.requestId);
        if (!pendingRequest) {
            return;
        }

        // Extract response headers
        const responseHeaders: Record<string, string> = {};
        if (details.responseHeaders) {
            details.responseHeaders.forEach((header) => {
                responseHeaders[header.name.toLowerCase()] = header.value || '';
            });
        }

        // Try to get response body (requires additional fetch)
        let responseBody: any = undefined;
        try {
            // Note: webRequest API doesn't provide response body directly
            // We'll rely on the injector script for response body capture
        } catch (error) {
            console.warn('[ForgeConnect] Failed to capture response body:', error);
        }

        const captured: CapturedRequest = {
            method: pendingRequest.method,
            url: pendingRequest.url,
            path: pendingRequest.path,
            queryParams: pendingRequest.queryParams,
            headers: pendingRequest.headers,
            body: pendingRequest.body,
            statusCode: details.statusCode,
            responseHeaders: Object.keys(responseHeaders).length > 0 ? responseHeaders : undefined,
            responseBody,
            timestamp: pendingRequest.timestamp,
        };

        // Store captured request
        capturedRequests.push(captured);
        if (capturedRequests.length > MAX_CAPTURED_REQUESTS) {
            capturedRequests.shift(); // Remove oldest
        }

        // Clean up pending request
        pendingRequests.delete(details.requestId);

        // Broadcast to DevTools and content scripts
        const message: ExtensionMessage = {
            type: 'REQUEST_CAPTURED',
            payload: captured,
        };

        // Send to DevTools
        chrome.runtime.sendMessage(message).catch(() => {
            // DevTools might not be open
        });

        // Send to content script in the tab
        if (pendingRequest.tabId) {
            chrome.tabs.sendMessage(pendingRequest.tabId, message).catch(() => {
                // Content script might not be loaded
            });
        }
    },
    { urls: ['<all_urls>'] },
    ['responseHeaders']
);

// Capture network errors
chrome.webRequest.onErrorOccurred.addListener(
    (details: chrome.webRequest.WebResponseErrorDetails) => {
        const pendingRequest = pendingRequests.get(details.requestId);
        if (!pendingRequest) {
            return;
        }

        const captured: CapturedRequest = {
            method: pendingRequest.method,
            url: pendingRequest.url,
            path: pendingRequest.path,
            queryParams: pendingRequest.queryParams,
            headers: pendingRequest.headers,
            body: pendingRequest.body,
            error: {
                type: 'network',
                message: details.error || 'Network error occurred',
            },
            timestamp: pendingRequest.timestamp,
        };

        // Store captured request
        capturedRequests.push(captured);
        if (capturedRequests.length > MAX_CAPTURED_REQUESTS) {
            capturedRequests.shift();
        }

        // Clean up pending request
        pendingRequests.delete(details.requestId);

        // Broadcast error
        const message: ExtensionMessage = {
            type: 'REQUEST_CAPTURED',
            payload: captured,
        };

        chrome.runtime.sendMessage(message).catch(() => {});
        if (pendingRequest.tabId) {
            chrome.tabs.sendMessage(pendingRequest.tabId, message).catch(() => {});
        }
    },
    { urls: ['<all_urls>'] }
);
