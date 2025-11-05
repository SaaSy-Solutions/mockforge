import * as vscode from 'vscode';
import { MockForgeClient } from '../services/mockforgeClient';
import { MocksTreeDataProvider } from '../providers/mocksTreeProvider';
import { Logger } from '../utils/logger';

/**
 * Command handler for creating a new mock
 */
export function registerCreateMockCommand(
    context: vscode.ExtensionContext,
    client: MockForgeClient,
    mocksProvider: MocksTreeDataProvider
): void {
    context.subscriptions.push(
        vscode.commands.registerCommand('mockforge.createMock', async () => {
            const name = await vscode.window.showInputBox({
                prompt: 'Enter mock name',
                placeHolder: 'My Mock'
            });

            if (!name) {
                return;
            }

            const method = await vscode.window.showQuickPick(
                ['GET', 'POST', 'PUT', 'DELETE', 'PATCH'],
                { placeHolder: 'Select HTTP method' }
            );

            if (!method) {
                return;
            }

            const path = await vscode.window.showInputBox({
                prompt: 'Enter API path',
                placeHolder: '/api/users'
            });

            if (!path) {
                return;
            }

            const body = await vscode.window.showInputBox({
                prompt: 'Enter response body (JSON)',
                placeHolder: '{"message": "success"}'
            });

            try {
                const responseBody = body ? JSON.parse(body) : {};
                await client.createMock({
                    name,
                    method,
                    path,
                    response: { body: responseBody },
                    enabled: true
                });

                Logger.info(`Mock "${name}" created successfully`);
                vscode.window.showInformationMessage(`Mock "${name}" created successfully`);
                mocksProvider.refresh();
            } catch (error) {
                Logger.error('Failed to create mock:', error);
                vscode.window.showErrorMessage(`Failed to create mock: ${error}`);
            }
        })
    );
}

