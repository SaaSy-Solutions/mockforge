import * as vscode from 'vscode';
import { MockForgeClient } from '../services/mockforgeClient';
import { MocksTreeDataProvider } from '../providers/mocksTreeProvider';
import { Logger } from '../utils/logger';

/**
 * Command handler for starting the MockForge server
 */
export function registerStartServerCommand(
    context: vscode.ExtensionContext,
    client: MockForgeClient,
    mocksProvider: MocksTreeDataProvider
): void {
    context.subscriptions.push(
        vscode.commands.registerCommand('mockforge.startServer', async () => {
            // Check if server is already running
            try {
                await client.getStats();
                vscode.window.showInformationMessage('MockForge server is already running');
                return;
            } catch {
                // Server is not running, proceed
            }

            // Open terminal with mockforge serve command
            const terminal = vscode.window.createTerminal('MockForge Server');
            terminal.sendText('mockforge serve');
            terminal.show();
            vscode.window.showInformationMessage('Starting MockForge server in terminal...');
            
            // Wait a bit and then try to connect
            setTimeout(async () => {
                try {
                    await client.connect();
                    Logger.info('Connected to MockForge server');
                    vscode.window.showInformationMessage('Connected to MockForge server');
                    mocksProvider.refresh();
                } catch (error) {
                    Logger.warn('Server may still be starting');
                    vscode.window.showWarningMessage('Server may still be starting. Use "Refresh Mocks" when ready.');
                }
            }, 2000);
        })
    );
}

/**
 * Command handler for stopping the MockForge server
 */
export function registerStopServerCommand(
    context: vscode.ExtensionContext,
    client: MockForgeClient,
    mocksProvider: MocksTreeDataProvider
): void {
    context.subscriptions.push(
        vscode.commands.registerCommand('mockforge.stopServer', async () => {
            // Check if server is running
            try {
                await client.getStats();
                vscode.window.showInformationMessage(
                    'MockForge server is running. To stop it, terminate the process running "mockforge serve" in your terminal.',
                    'Open Terminal'
                ).then((selection: string | undefined) => {
                    if (selection === 'Open Terminal') {
                        vscode.window.createTerminal('MockForge').show();
                    }
                });
            } catch {
                vscode.window.showInformationMessage('MockForge server is not running');
            }
            
            // Disconnect the client
            client.disconnect();
            mocksProvider.refresh();
        })
    );
}

/**
 * Command handler for restarting the MockForge server
 */
export function registerRestartServerCommand(
    context: vscode.ExtensionContext,
    client: MockForgeClient,
    mocksProvider: MocksTreeDataProvider
): void {
    context.subscriptions.push(
        vscode.commands.registerCommand('mockforge.restartServer', async () => {
            // Disconnect first
            client.disconnect();
            
            // Show message about restarting
            vscode.window.showInformationMessage(
                'To restart the server, stop the running "mockforge serve" process and start it again.',
                'Open Terminal'
            ).then((selection: string | undefined) => {
                if (selection === 'Open Terminal') {
                    const terminal = vscode.window.createTerminal('MockForge');
                    terminal.sendText('mockforge serve');
                    terminal.show();
                }
            });
            
            // Wait and try to reconnect
            setTimeout(async () => {
                try {
                    await client.connect();
                    Logger.info('Reconnected to MockForge server');
                    vscode.window.showInformationMessage('Reconnected to MockForge server');
                    mocksProvider.refresh();
                } catch (error) {
                    Logger.warn('Server may still be restarting');
                }
            }, 3000);
        })
    );
}

