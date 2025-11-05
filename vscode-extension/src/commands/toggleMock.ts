import * as vscode from 'vscode';
import { MockForgeClient } from '../services/mockforgeClient';
import { MocksTreeDataProvider } from '../providers/mocksTreeProvider';
import { MockConfig } from '../types/mock';
import { Logger } from '../utils/logger';

/**
 * Command handler for toggling a mock's enabled state
 */
export function registerToggleMockCommand(
    context: vscode.ExtensionContext,
    client: MockForgeClient,
    mocksProvider: MocksTreeDataProvider
): void {
    context.subscriptions.push(
        vscode.commands.registerCommand('mockforge.toggleMock', async (mock: MockConfig) => {
            try {
                await client.updateMock(mock.id, {
                    ...mock,
                    enabled: !mock.enabled
                });
                const newState = mock.enabled ? 'disabled' : 'enabled';
                Logger.info(`Mock "${mock.name}" ${newState}`);
                vscode.window.showInformationMessage(`Mock "${mock.name}" ${newState}`);
                mocksProvider.refresh();
            } catch (error) {
                Logger.error('Failed to toggle mock:', error);
                vscode.window.showErrorMessage(`Failed to toggle mock: ${error}`);
            }
        })
    );
}
