import * as vscode from 'vscode';
import { MockConfig } from '../types/mock';

/**
 * Command handler for editing a mock
 */
export function registerEditMockCommand(context: vscode.ExtensionContext): void {
    context.subscriptions.push(
        vscode.commands.registerCommand('mockforge.editMock', async (mock: MockConfig) => {
            // Open a new editor with mock configuration
            const doc = await vscode.workspace.openTextDocument({
                content: JSON.stringify(mock, null, 2),
                language: 'json'
            });

            await vscode.window.showTextDocument(doc);
        })
    );
}
