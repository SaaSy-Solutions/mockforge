/**
 * Sync Queue
 *
 * Manages queue of pending operations to sync when online
 */

import { MockConfig } from '../types';
import { OfflineStorage } from './OfflineStorage';
import { MockForgeClient } from './MockForgeClient';

export interface SyncQueueItem {
    id: string;
    operation: 'create' | 'update' | 'delete';
    mock: MockConfig | null;
    mockId?: string;
    timestamp: number;
    retries: number;
}

/**
 * Sync Queue Manager
 * Handles syncing pending operations when connection is restored
 */
export class SyncQueue {
    private storage: OfflineStorage;
    private client: MockForgeClient;
    private maxRetries: number = 3;
    private isSyncing: boolean = false;

    constructor(storage: OfflineStorage, client: MockForgeClient) {
        this.storage = storage;
        this.client = client;
    }

    /**
     * Add operation to sync queue
     */
    async enqueue(operation: 'create' | 'update' | 'delete', mock: MockConfig | null, mockId?: string): Promise<string> {
        return await this.storage.addToSyncQueue(operation, mock, mockId);
    }

    /**
     * Process sync queue
     */
    async sync(): Promise<{ success: number; failed: number }> {
        if (this.isSyncing) {
            return { success: 0, failed: 0 };
        }

        this.isSyncing = true;
        const queue = await this.storage.getSyncQueue();
        let success = 0;
        let failed = 0;

        for (const item of queue) {
            try {
                // Check retry limit
                if (item.retries >= this.maxRetries) {
                    console.warn(`[SyncQueue] Max retries reached for ${item.id}, skipping`);
                    await this.storage.removeFromSyncQueue(item.id);
                    failed++;
                    continue;
                }

                // Try to sync
                let syncSuccess = false;
                switch (item.operation) {
                    case 'create':
                        if (item.mock) {
                            await this.client.createMock(item.mock);
                            syncSuccess = true;
                        }
                        break;
                    case 'update':
                        if (item.mock && item.mockId) {
                            await this.client.updateMock(item.mockId, item.mock);
                            syncSuccess = true;
                        }
                        break;
                    case 'delete':
                        if (item.mockId) {
                            await this.client.deleteMock(item.mockId);
                            syncSuccess = true;
                        }
                        break;
                }

                if (syncSuccess) {
                    await this.storage.removeFromSyncQueue(item.id);
                    success++;
                } else {
                    // Increment retry count
                    await this.storage.updateSyncQueueItem(item.id, item.retries + 1);
                    failed++;
                }
            } catch (error) {
                console.error(`[SyncQueue] Failed to sync ${item.id}:`, error);
                // Increment retry count
                await this.storage.updateSyncQueueItem(item.id, item.retries + 1);
                failed++;
            }
        }

        this.isSyncing = false;
        return { success, failed };
    }

    /**
     * Get queue size
     */
    async getQueueSize(): Promise<number> {
        const queue = await this.storage.getSyncQueue();
        return queue.length;
    }

    /**
     * Clear sync queue
     */
    async clear(): Promise<void> {
        await this.storage.clearSyncQueue();
    }
}
