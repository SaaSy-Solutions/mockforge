import * as vscode from 'vscode';
import { MockForgeClient, MockConfig } from './mockforgeClient';

export class MocksTreeDataProvider implements vscode.TreeDataProvider<MockTreeItem> {
    private _onDidChangeTreeData = new vscode.EventEmitter<MockTreeItem | undefined | null | void>();
    readonly onDidChangeTreeData = this._onDidChangeTreeData.event;

    constructor(private client: MockForgeClient) {
        // Listen for WebSocket events to auto-refresh
        client.onEvent((event) => {
            if (['mock_created', 'mock_updated', 'mock_deleted'].includes(event.type)) {
                this.refresh();
            }
        });
    }

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
            const mocks = await this.client.getMocks();
            return mocks.map(mock => new MockTreeItem(mock));
        } catch (error) {
            vscode.window.showErrorMessage(`Failed to load mocks: ${error}`);
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
