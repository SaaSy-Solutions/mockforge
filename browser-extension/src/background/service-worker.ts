/**
 * Background Service Worker
 * 
 * Handles communication between content scripts, DevTools, and MockForge
 */

import { ExtensionMessage, CapturedRequest, ConnectionStatus } from '../shared/types';
import { MockForgeApiClient } from '../shared/api-client';

// Connection state
let mockForgeUrl: string | null = null;
let apiClient: MockForgeApiClient | null = null;
let connected: boolean = false;

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

