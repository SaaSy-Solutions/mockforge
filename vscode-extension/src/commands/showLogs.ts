import * as vscode from 'vscode';
import { Logger } from '../utils/logger';

/**
 * Command handler for showing extension logs
 */
export function registerShowLogsCommand(context: vscode.ExtensionContext): void {
    context.subscriptions.push(
        vscode.commands.registerCommand('mockforge.showLogs', () => {
            Logger.show();
        })
    );
}
