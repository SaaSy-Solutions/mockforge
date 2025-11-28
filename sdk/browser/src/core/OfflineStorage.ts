/**
 * Offline Storage
 *
 * Manages IndexedDB storage for caching mocks when offline
 */

import { MockConfig } from '../types';

const DB_NAME = 'forgeconnect_offline';
const DB_VERSION = 1;
const STORE_NAME = 'mocks';
const SYNC_QUEUE_STORE = 'sync_queue';

interface CachedMock extends MockConfig {
    cachedAt: number;
    syncStatus: 'synced' | 'pending' | 'conflict';
    environmentId?: string;
    version: number;
}

interface SyncQueueItem {
    id: string;
    operation: 'create' | 'update' | 'delete';
    mock: MockConfig | null;
    mockId?: string;
    timestamp: number;
    retries: number;
}

/**
 * Offline Storage Manager
 */
export class OfflineStorage {
    private db: IDBDatabase | null = null;

    /**
     * Initialize IndexedDB
     */
    async initialize(): Promise<void> {
        return new Promise((resolve, reject) => {
            const request = indexedDB.open(DB_NAME, DB_VERSION);

            request.onerror = () => reject(request.error);
            request.onsuccess = () => {
                this.db = request.result;
                resolve();
            };

            request.onupgradeneeded = (event) => {
                const db = (event.target as IDBOpenDBRequest).result;

                // Mocks store
                if (!db.objectStoreNames.contains(STORE_NAME)) {
                    const store = db.createObjectStore(STORE_NAME, { keyPath: 'id' });
                    store.createIndex('path', 'path', { unique: false });
                    store.createIndex('method', 'method', { unique: false });
                    store.createIndex('cachedAt', 'cachedAt', { unique: false });
                    store.createIndex('syncStatus', 'syncStatus', { unique: false });
                    store.createIndex('environmentId', 'environmentId', { unique: false });
                }

                // Sync queue store
                if (!db.objectStoreNames.contains(SYNC_QUEUE_STORE)) {
                    const store = db.createObjectStore(SYNC_QUEUE_STORE, { keyPath: 'id' });
                    store.createIndex('timestamp', 'timestamp', { unique: false });
                    store.createIndex('operation', 'operation', { unique: false });
                }
            };
        });
    }

    /**
     * Cache a mock
     */
    async cacheMock(mock: MockConfig, environmentId?: string): Promise<void> {
        if (!this.db) {
            await this.initialize();
        }

        const cachedMock: CachedMock = {
            ...mock,
            cachedAt: Date.now(),
            syncStatus: 'synced',
            environmentId,
            version: 1,
        };

        return new Promise((resolve, reject) => {
            if (!this.db) {
                reject(new Error('Database not initialized'));
                return;
            }

            const transaction = this.db.transaction([STORE_NAME], 'readwrite');
            const store = transaction.objectStore(STORE_NAME);
            const request = store.put(cachedMock);

            request.onsuccess = () => resolve();
            request.onerror = () => reject(request.error);
        });
    }

    /**
     * Get all cached mocks
     */
    async getAllCachedMocks(environmentId?: string): Promise<CachedMock[]> {
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

            request.onsuccess = () => {
                let mocks = (request.result || []) as CachedMock[];

                // Filter by environment if specified
                if (environmentId) {
                    mocks = mocks.filter(m => !m.environmentId || m.environmentId === environmentId);
                }

                resolve(mocks);
            };
            request.onerror = () => reject(request.error);
        });
    }

    /**
     * Find matching mock for a request
     */
    async findMatchingMock(method: string, path: string, environmentId?: string): Promise<CachedMock | null> {
        const mocks = await this.getAllCachedMocks(environmentId);

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

    /**
     * Delete a cached mock
     */
    async deleteCachedMock(mockId: string): Promise<void> {
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
            const request = store.delete(mockId);

            request.onsuccess = () => resolve();
            request.onerror = () => reject(request.error);
        });
    }

    /**
     * Clear all cached mocks
     */
    async clearAllMocks(): Promise<void> {
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
     * Add item to sync queue
     */
    async addToSyncQueue(operation: 'create' | 'update' | 'delete', mock: MockConfig | null, mockId?: string): Promise<string> {
        if (!this.db) {
            await this.initialize();
        }

        const queueItem: SyncQueueItem = {
            id: `sync_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
            operation,
            mock,
            mockId,
            timestamp: Date.now(),
            retries: 0,
        };

        return new Promise((resolve, reject) => {
            if (!this.db) {
                reject(new Error('Database not initialized'));
                return;
            }

            const transaction = this.db.transaction([SYNC_QUEUE_STORE], 'readwrite');
            const store = transaction.objectStore(SYNC_QUEUE_STORE);
            const request = store.put(queueItem);

            request.onsuccess = () => resolve(queueItem.id);
            request.onerror = () => reject(request.error);
        });
    }

    /**
     * Get all sync queue items
     */
    async getSyncQueue(): Promise<SyncQueueItem[]> {
        if (!this.db) {
            await this.initialize();
        }

        return new Promise((resolve, reject) => {
            if (!this.db) {
                reject(new Error('Database not initialized'));
                return;
            }

            const transaction = this.db.transaction([SYNC_QUEUE_STORE], 'readonly');
            const store = transaction.objectStore(SYNC_QUEUE_STORE);
            const request = store.getAll();

            request.onsuccess = () => resolve(request.result || []);
            request.onerror = () => reject(request.error);
        });
    }

    /**
     * Remove item from sync queue
     */
    async removeFromSyncQueue(queueId: string): Promise<void> {
        if (!this.db) {
            await this.initialize();
        }

        return new Promise((resolve, reject) => {
            if (!this.db) {
                reject(new Error('Database not initialized'));
                return;
            }

            const transaction = this.db.transaction([SYNC_QUEUE_STORE], 'readwrite');
            const store = transaction.objectStore(SYNC_QUEUE_STORE);
            const request = store.delete(queueId);

            request.onsuccess = () => resolve();
            request.onerror = () => reject(request.error);
        });
    }

    /**
     * Update sync queue item retry count
     */
    async updateSyncQueueItem(queueId: string, retries: number): Promise<void> {
        if (!this.db) {
            await this.initialize();
        }

        return new Promise((resolve, reject) => {
            if (!this.db) {
                reject(new Error('Database not initialized'));
                return;
            }

            const transaction = this.db.transaction([SYNC_QUEUE_STORE], 'readwrite');
            const store = transaction.objectStore(SYNC_QUEUE_STORE);
            const getRequest = store.get(queueId);

            getRequest.onsuccess = () => {
                const item = getRequest.result;
                if (item) {
                    item.retries = retries;
                    const putRequest = store.put(item);
                    putRequest.onsuccess = () => resolve();
                    putRequest.onerror = () => reject(putRequest.error);
                } else {
                    resolve();
                }
            };
            getRequest.onerror = () => reject(getRequest.error);
        });
    }

    /**
     * Clear sync queue
     */
    async clearSyncQueue(): Promise<void> {
        if (!this.db) {
            await this.initialize();
        }

        return new Promise((resolve, reject) => {
            if (!this.db) {
                reject(new Error('Database not initialized'));
                return;
            }

            const transaction = this.db.transaction([SYNC_QUEUE_STORE], 'readwrite');
            const store = transaction.objectStore(SYNC_QUEUE_STORE);
            const request = store.clear();

            request.onsuccess = () => resolve();
            request.onerror = () => reject(request.error);
        });
    }

    /**
     * Get cache size (approximate)
     */
    async getCacheSize(): Promise<number> {
        const mocks = await this.getAllCachedMocks();
        return mocks.length;
    }
}
