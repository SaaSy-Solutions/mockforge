/**
 * Service Worker for Mock Preview
 *
 * Intercepts fetch requests and serves preview mocks from IndexedDB
 */

const DB_NAME = 'forgeconnect_preview';
const DB_VERSION = 1;
const STORE_NAME = 'preview_mocks';

// Open IndexedDB
async function openDB() {
    return new Promise((resolve, reject) => {
        const request = indexedDB.open(DB_NAME, DB_VERSION);

        request.onerror = () => reject(request.error);
        request.onsuccess = () => resolve(request.result);

        request.onupgradeneeded = (event) => {
            const db = event.target.result;
            if (!db.objectStoreNames.contains(STORE_NAME)) {
                const store = db.createObjectStore(STORE_NAME, { keyPath: 'previewId' });
                store.createIndex('path', 'path', { unique: false });
                store.createIndex('method', 'method', { unique: false });
            }
        };
    });
}

// Get all preview mocks from IndexedDB
async function getAllPreviewMocks() {
    const db = await openDB();
    return new Promise((resolve, reject) => {
        const transaction = db.transaction([STORE_NAME], 'readonly');
        const store = transaction.objectStore(STORE_NAME);
        const request = store.getAll();

        request.onsuccess = () => resolve(request.result || []);
        request.onerror = () => reject(request.error);
    });
}

// Find matching mock for a request
async function findMatchingMock(method, path) {
    const mocks = await getAllPreviewMocks();

    // Find exact match first
    let match = mocks.find(
        (m) => m.method.toUpperCase() === method.toUpperCase() && m.path === path && m.enabled !== false
    );

    if (match) {
        return match;
    }

    // Try path pattern matching
    match = mocks.find((m) => {
        if (m.method.toUpperCase() !== method.toUpperCase() || m.enabled === false) {
            return false;
        }

        // Convert path pattern to regex
        const pattern = m.path.replace(/\*/g, '.*').replace(/\{([^}]+)\}/g, '[^/]+');
        const regex = new RegExp(`^${pattern}$`);
        return regex.test(path);
    });

    return match || null;
}

// Check if preview mode is enabled
async function isPreviewModeEnabled() {
    try {
        const result = await chrome.storage.local.get(['previewModeEnabled']);
        return result.previewModeEnabled !== false; // Default to enabled
    } catch {
        return true;
    }
}

// Intercept fetch requests
self.addEventListener('fetch', (event) => {
    event.respondWith(
        (async () => {
            const request = event.request;
            const url = new URL(request.url);

            // Skip non-HTTP(S) URLs
            if (!url.protocol.startsWith('http')) {
                return fetch(request);
            }

            // Check if preview mode is enabled
            const previewEnabled = await isPreviewModeEnabled();
            if (!previewEnabled) {
                return fetch(request);
            }

            // Skip chrome-extension URLs
            if (url.protocol === 'chrome-extension:') {
                return fetch(request);
            }

            // Try to find matching preview mock
            const method = request.method;
            const path = url.pathname;

            try {
                const mock = await findMatchingMock(method, path);

                if (mock && mock.response) {
                    // Serve preview mock
                    const responseBody = typeof mock.response.body === 'string'
                        ? mock.response.body
                        : JSON.stringify(mock.response.body);

                    const headers = new Headers({
                        'Content-Type': 'application/json',
                        ...(mock.response.headers || {}),
                    });

                    // Add CORS headers if needed
                    if (!headers.has('Access-Control-Allow-Origin')) {
                        headers.set('Access-Control-Allow-Origin', '*');
                    }

                    const statusCode = mock.status_code || 200;

                    return new Response(responseBody, {
                        status: statusCode,
                        statusText: getStatusText(statusCode),
                        headers,
                    });
                }
            } catch (error) {
                console.warn('[ForgeConnect Preview] Error finding mock:', error);
            }

            // No matching mock, proceed with original request
            return fetch(request);
        })()
    );
});

// Helper to get status text
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

// Install event - claim clients immediately
self.addEventListener('install', (event) => {
    self.skipWaiting();
});

// Activate event - claim clients
self.addEventListener('activate', (event) => {
    event.waitUntil(self.clients.claim());
});
