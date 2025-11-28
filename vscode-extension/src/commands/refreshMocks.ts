import * as vscode from 'vscode';
import { MocksTreeDataProvider } from '../providers/mocksTreeProvider';

/**
 * Command handler for refreshing the mocks tree view
 */
export function registerRefreshMocksCommand(
    context: vscode.ExtensionContext,
    mocksProvider: MocksTreeDataProvider
): void {
    context.subscriptions.push(
        vscode.commands.registerCommand('mockforge.refreshMocks', () => {
            mocksProvider.refresh();
        })
    );
}
