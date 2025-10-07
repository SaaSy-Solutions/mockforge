import * as vscode from 'vscode';
import { MockForgeClient } from './mockforgeClient';

export class ServerControlProvider implements vscode.TreeDataProvider<ServerTreeItem> {
    private _onDidChangeTreeData = new vscode.EventEmitter<ServerTreeItem | undefined | null | void>();
    readonly onDidChangeTreeData = this._onDidChangeTreeData.event;

    constructor(private client: MockForgeClient) {}

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

        try {
            const [stats, config] = await Promise.all([
                this.client.getStats(),
                this.client.getConfig()
            ]);

            return [
                new ServerTreeItem('status', '✓ Connected', 'Server is running'),
                new ServerTreeItem('version', `v${config.version}`, 'Server version'),
                new ServerTreeItem('port', `Port: ${config.port}`, 'Server port'),
                new ServerTreeItem('uptime', `Uptime: ${stats.uptime_seconds}s`, 'Server uptime'),
                new ServerTreeItem('requests', `Requests: ${stats.total_requests}`, 'Total requests'),
                new ServerTreeItem('mocks', `Active: ${stats.active_mocks}`, 'Active mocks')
            ];
        } catch (error) {
            return [
                new ServerTreeItem('status', '✗ Disconnected', 'Server is not reachable')
            ];
        }
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
