import * as vscode from 'vscode';
import { MockForgeClient } from '../services/mockforgeClient';
import { MockConfig } from '../types/mock';
import { MockEvent } from '../types/events';

/**
 * Tree data provider for displaying mocks in the VS Code explorer
 *
 * Performance optimizations:
 * - Caches mock list to avoid repeated API calls
 * - Debounces refresh events to prevent rapid-fire updates
 * - Only refreshes when cache actually changes
 */
export class MocksTreeDataProvider implements vscode.TreeDataProvider<MockTreeItem> {
    private _onDidChangeTreeData = new vscode.EventEmitter<MockTreeItem | undefined | null | void>();
    readonly onDidChangeTreeData = this._onDidChangeTreeData.event;

    // Cache for mock list to avoid repeated API calls
    private cachedMocks: MockConfig[] | null = null;
    private lastCacheTime: number = 0;
    private readonly cacheTimeoutMs = 5000; // Cache for 5 seconds

    // Debounce timer for refresh events
    private refreshTimer: NodeJS.Timeout | undefined;
    private readonly debounceDelayMs = 300; // Debounce refresh by 300ms

    constructor(private client: MockForgeClient) {
        // Listen for WebSocket events to auto-refresh
        client.onEvent((event: MockEvent) => {
            // Handle different event types using type discrimination
            switch (event.type) {
                case 'mock_created':
                case 'mock_updated':
                case 'mock_deleted':
                    // Invalidate cache and debounce refresh
                    this.invalidateCache();
                    this.debouncedRefresh();
                    break;
                case 'stats_updated':
                    // Could refresh stats view here if needed
                    break;
                case 'connected':
                    // Connection established, refresh to show current state
                    this.invalidateCache();
                    this.debouncedRefresh();
                    break;
            }
        });
    }

    /**
     * Invalidate the cache to force fresh data on next fetch
     */
    private invalidateCache(): void {
        this.cachedMocks = null;
        this.lastCacheTime = 0;
    }

    /**
     * Debounced refresh to prevent rapid-fire updates when multiple events arrive
     */
    private debouncedRefresh(): void {
        // Clear existing timer
        if (this.refreshTimer) {
            clearTimeout(this.refreshTimer);
        }

        // Set new timer
        this.refreshTimer = setTimeout(() => {
            this.refresh();
            this.refreshTimer = undefined;
        }, this.debounceDelayMs);
    }

    /**
     * Refresh the tree view
     */
    refresh(): void {
        this._onDidChangeTreeData.fire();
    }

    getTreeItem(element: MockTreeItem): vscode.TreeItem {
        return element;
    }

    async getChildren(element?: MockTreeItem): Promise<MockTreeItem[]> {
        if (element) {
            return [];
        }

        try {
            // Check if cache is still valid
            const now = Date.now();
            const cacheValid = this.cachedMocks !== null &&
                              (now - this.lastCacheTime) < this.cacheTimeoutMs;

            let mocks: MockConfig[];

            if (cacheValid && this.cachedMocks !== null) {
                // Use cached data
                mocks = this.cachedMocks;
            } else {
                // Fetch fresh data and update cache
                mocks = await this.client.getMocks();
                this.cachedMocks = mocks;
                this.lastCacheTime = now;
            }

            // Map to tree items (this is lightweight, so we do it every time)
            return mocks.map(mock => new MockTreeItem(mock));
        } catch (error) {
            vscode.window.showErrorMessage(`Failed to load mocks: ${error}`);
            // Clear cache on error to force fresh fetch next time
            this.invalidateCache();
            return [];
        }
    }
}

export class MockTreeItem extends vscode.TreeItem {
    constructor(public readonly mock: MockConfig) {
        super(mock.name, vscode.TreeItemCollapsibleState.None);

        this.tooltip = `${mock.method} ${mock.path}`;
        this.description = `${mock.method} ${mock.path}`;
        this.contextValue = 'mock';

        // Set icon based on HTTP method
        const iconMap: Record<string, string> = {
            'GET': 'arrow-down',
            'POST': 'add',
            'PUT': 'edit',
            'DELETE': 'trash',
            'PATCH': 'diff'
        };

        this.iconPath = new vscode.ThemeIcon(
            iconMap[mock.method] || 'circle',
            mock.enabled ? undefined : new vscode.ThemeColor('disabledForeground')
        );

        // Add checkbox for enabled/disabled state
        this.checkboxState = mock.enabled
            ? vscode.TreeItemCheckboxState.Checked
            : vscode.TreeItemCheckboxState.Unchecked;
    }
}
