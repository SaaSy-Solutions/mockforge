import * as vscode from 'vscode';
import { MockForgeClient } from '../services/mockforgeClient';
import { MocksTreeDataProvider } from '../providers/mocksTreeProvider';
import { Logger } from '../utils/logger';

/**
 * Command handler for importing mocks
 */
export function registerImportMocksCommand(
    context: vscode.ExtensionContext,
    client: MockForgeClient,
    mocksProvider: MocksTreeDataProvider
): void {
    context.subscriptions.push(
        vscode.commands.registerCommand('mockforge.importMocks', async () => {
            const uri = await vscode.window.showOpenDialog({
                canSelectMany: false,
                filters: {
                    // eslint-disable-next-line @typescript-eslint/naming-convention
                    'JSON/YAML': ['json', 'yaml', 'yml']
                }
            });

            if (!uri || uri.length === 0) {
                return;
            }

            const merge = await vscode.window.showQuickPick(['Replace', 'Merge'], {
                placeHolder: 'Import strategy'
            });

            if (!merge) {
                return;
            }

            try {
                const content = await vscode.workspace.fs.readFile(uri[0]);
                const format = uri[0].path.endsWith('.json') ? 'json' : 'yaml';
                await client.importMocks(content.toString(), format, merge.toLowerCase() === 'merge');
                Logger.info('Mocks imported successfully');
                vscode.window.showInformationMessage('Mocks imported successfully');
                mocksProvider.refresh();
            } catch (error) {
                Logger.error('Failed to import mocks:', error);
                vscode.window.showErrorMessage(`Failed to import mocks: ${error}`);
            }
        })
    );
}
