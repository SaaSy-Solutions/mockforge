import * as vscode from 'vscode';
import { MockForgeClient } from '../services/mockforgeClient';

/**
 * Tree data provider for displaying server control information
 */
export class ServerControlProvider implements vscode.TreeDataProvider<ServerTreeItem> {
    private _onDidChangeTreeData = new vscode.EventEmitter<ServerTreeItem | undefined | null | void>();
    readonly onDidChangeTreeData = this._onDidChangeTreeData.event;

    constructor(private client: MockForgeClient) {
        // Listen for connection state changes to refresh the view
        client.onStateChange(() => {
            this.refresh();
        });
    }

    refresh(): void {
        this._onDidChangeTreeData.fire();
    }

    getTreeItem(element: ServerTreeItem): vscode.TreeItem {
        return element;
    }

    async getChildren(element?: ServerTreeItem): Promise<ServerTreeItem[]> {
        if (element) {
            return [];
        }

        // Get connection state
        const connectionState = this.client.connectionState;

        // Show connection status based on state
        const statusItems: ServerTreeItem[] = [];

        switch (connectionState) {
            case 'connected':
                try {
                    const [stats, config] = await Promise.all([
                        this.client.getStats(),
                        this.client.getConfig()
                    ]);

                    statusItems.push(
                        new ServerTreeItem('status', '✓ Connected', 'Server is running and connected'),
                        new ServerTreeItem('version', `v${config.version}`, 'Server version'),
                        new ServerTreeItem('port', `Port: ${config.port}`, 'Server port'),
                        new ServerTreeItem('uptime', `Uptime: ${stats.uptime_seconds}s`, 'Server uptime'),
                        new ServerTreeItem('requests', `Requests: ${stats.total_requests}`, 'Total requests'),
                        new ServerTreeItem('mocks', `Active: ${stats.active_mocks}`, 'Active mocks')
                    );
                } catch (error) {
                    statusItems.push(
                        new ServerTreeItem('status', '⚠ Connection Error', 'Connected but unable to fetch server info')
                    );
                }
                break;
            case 'connecting':
                statusItems.push(
                    new ServerTreeItem('status', '⟳ Connecting...', 'Attempting to connect to server')
                );
                break;
            case 'reconnecting':
                statusItems.push(
                    new ServerTreeItem('status', '⟳ Reconnecting...', 'Attempting to reconnect to server')
                );
                break;
            case 'disconnected':
                statusItems.push(
                    new ServerTreeItem('status', '✗ Disconnected', 'Server is not reachable')
                );
                break;
        }

        return statusItems;
    }
}

export class ServerTreeItem extends vscode.TreeItem {
    constructor(
        public readonly type: string,
        public readonly label: string,
        public readonly tooltip: string
    ) {
        super(label, vscode.TreeItemCollapsibleState.None);
        this.tooltip = tooltip;
        this.contextValue = type;

        // Set icons
        const iconMap: Record<string, string> = {
            'status': 'pulse',
            'version': 'tag',
            'port': 'server',
            'uptime': 'clock',
            'requests': 'graph',
            'mocks': 'files'
        };

        this.iconPath = new vscode.ThemeIcon(iconMap[type] || 'info');
    }
}
