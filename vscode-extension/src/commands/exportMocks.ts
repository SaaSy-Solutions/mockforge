import * as vscode from 'vscode';
import { MockForgeClient } from '../services/mockforgeClient';
import { Logger } from '../utils/logger';

/**
 * Command handler for exporting mocks
 */
export function registerExportMocksCommand(
    context: vscode.ExtensionContext,
    client: MockForgeClient
): void {
    context.subscriptions.push(
        vscode.commands.registerCommand('mockforge.exportMocks', async () => {
            const format = await vscode.window.showQuickPick(['JSON', 'YAML'], {
                placeHolder: 'Select export format'
            });

            if (!format) {
                return;
            }

            try {
                const data = await client.exportMocks(format.toLowerCase());
                const uri = await vscode.window.showSaveDialog({
                    defaultUri: vscode.Uri.file(`mocks.${format.toLowerCase()}`),
                    filters: {
                        [format]: [format.toLowerCase()]
                    }
                });

                if (uri) {
                    await vscode.workspace.fs.writeFile(uri, Buffer.from(data));
                    Logger.info('Mocks exported successfully');
                    vscode.window.showInformationMessage('Mocks exported successfully');
                }
            } catch (error) {
                Logger.error('Failed to export mocks:', error);
                vscode.window.showErrorMessage(`Failed to export mocks: ${error}`);
            }
        })
    );
}
