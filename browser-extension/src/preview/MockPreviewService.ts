/**
 * Mock Preview Service
 *
 * Manages preview mocks stored in IndexedDB and coordinates with Service Worker
 */

import { MockConfig } from '../shared/types';

interface PreviewMock extends MockConfig {
    id: string;
    previewId: string; // Unique ID for preview mocks
    createdAt: number;
}

const DB_NAME = 'forgeconnect_preview';
const DB_VERSION = 1;
const STORE_NAME = 'preview_mocks';

/**
 * Initialize IndexedDB for preview mocks
 */
async function openDB(): Promise<IDBDatabase> {
    return new Promise((resolve, reject) => {
        const request = indexedDB.open(DB_NAME, DB_VERSION);

        request.onerror = () => reject(request.error);
        request.onsuccess = () => resolve(request.result);

        request.onupgradeneeded = (event) => {
            const db = (event.target as IDBOpenDBRequest).result;
            if (!db.objectStoreNames.contains(STORE_NAME)) {
                const store = db.createObjectStore(STORE_NAME, { keyPath: 'previewId' });
                store.createIndex('path', 'path', { unique: false });
                store.createIndex('method', 'method', { unique: false });
                store.createIndex('createdAt', 'createdAt', { unique: false });
            }
        };
    });
}

/**
 * Mock Preview Service
 */
export class MockPreviewService {
    private db: IDBDatabase | null = null;

    /**
     * Initialize the service
     */
    async initialize(): Promise<void> {
        this.db = await openDB();
    }

    /**
     * Save a preview mock
     */
    async savePreviewMock(mock: MockConfig): Promise<string> {
        if (!this.db) {
            await this.initialize();
        }

        const previewMock: PreviewMock = {
            ...mock,
            previewId: `preview_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
            id: mock.id || `preview_${Date.now()}`,
            createdAt: Date.now(),
        };

        return new Promise((resolve, reject) => {
            if (!this.db) {
                reject(new Error('Database not initialized'));
                return;
            }

            const transaction = this.db.transaction([STORE_NAME], 'readwrite');
            const store = transaction.objectStore(STORE_NAME);
            const request = store.put(previewMock);

            request.onsuccess = () => resolve(previewMock.previewId);
            request.onerror = () => reject(request.error);
        });
    }

    /**
     * Get all preview mocks
     */
    async getAllPreviewMocks(): Promise<PreviewMock[]> {
        if (!this.db) {
            await this.initialize();
        }

        return new Promise((resolve, reject) => {
            if (!this.db) {
                reject(new Error('Database not initialized'));
                return;
            }

            const transaction = this.db.transaction([STORE_NAME], 'readonly');
            const store = transaction.objectStore(STORE_NAME);
            const request = store.getAll();

            request.onsuccess = () => resolve(request.result || []);
            request.onerror = () => reject(request.error);
        });
    }

    /**
     * Get a preview mock by ID
     */
    async getPreviewMock(previewId: string): Promise<PreviewMock | null> {
        if (!this.db) {
            await this.initialize();
        }

        return new Promise((resolve, reject) => {
            if (!this.db) {
                reject(new Error('Database not initialized'));
                return;
            }

            const transaction = this.db.transaction([STORE_NAME], 'readonly');
            const store = transaction.objectStore(STORE_NAME);
            const request = store.get(previewId);

            request.onsuccess = () => resolve(request.result || null);
            request.onerror = () => reject(request.error);
        });
    }

    /**
     * Delete a preview mock
     */
    async deletePreviewMock(previewId: string): Promise<void> {
        if (!this.db) {
            await this.initialize();
        }

        return new Promise((resolve, reject) => {
            if (!this.db) {
                reject(new Error('Database not initialized'));
                return;
            }

            const transaction = this.db.transaction([STORE_NAME], 'readwrite');
            const store = transaction.objectStore(STORE_NAME);
            const request = store.delete(previewId);

            request.onsuccess = () => resolve();
            request.onerror = () => reject(request.error);
        });
    }

    /**
     * Clear all preview mocks
     */
    async clearAllPreviewMocks(): Promise<void> {
        if (!this.db) {
            await this.initialize();
        }

        return new Promise((resolve, reject) => {
            if (!this.db) {
                reject(new Error('Database not initialized'));
                return;
            }

            const transaction = this.db.transaction([STORE_NAME], 'readwrite');
            const store = transaction.objectStore(STORE_NAME);
            const request = store.clear();

            request.onsuccess = () => resolve();
            request.onerror = () => reject(request.error);
        });
    }

    /**
     * Find matching preview mock for a request
     */
    async findMatchingMock(method: string, path: string): Promise<PreviewMock | null> {
        const mocks = await this.getAllPreviewMocks();

        // Find exact match first
        let match = mocks.find(
            (m) => m.method.toUpperCase() === method.toUpperCase() && m.path === path && m.enabled !== false
        );

        if (match) {
            return match;
        }

        // Try path pattern matching (simple wildcard support)
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
}
