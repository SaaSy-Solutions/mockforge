import * as vscode from 'vscode';
import { MockForgeClient } from '../services/mockforgeClient';
import { MocksTreeDataProvider } from '../providers/mocksTreeProvider';
import { MockConfig } from '../types/mock';
import { Logger } from '../utils/logger';

/**
 * Command handler for deleting a mock
 */
export function registerDeleteMockCommand(
    context: vscode.ExtensionContext,
    client: MockForgeClient,
    mocksProvider: MocksTreeDataProvider
): void {
    context.subscriptions.push(
        vscode.commands.registerCommand('mockforge.deleteMock', async (mock: MockConfig) => {
            const confirm = await vscode.window.showWarningMessage(
                `Delete mock "${mock.name}"?`,
                'Yes',
                'No'
            );

            if (confirm === 'Yes') {
                try {
                    await client.deleteMock(mock.id);
                    Logger.info(`Mock "${mock.name}" deleted`);
                    vscode.window.showInformationMessage(`Mock "${mock.name}" deleted`);
                    mocksProvider.refresh();
                } catch (error) {
                    Logger.error('Failed to delete mock:', error);
                    vscode.window.showErrorMessage(`Failed to delete mock: ${error}`);
                }
            }
        })
    );
}
