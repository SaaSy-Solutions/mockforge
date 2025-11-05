import * as vscode from 'vscode';
import { MocksTreeDataProvider } from './providers/mocksTreeProvider';
import { ServerControlProvider } from './providers/serverControlProvider';
import { MockForgeClient } from './services/mockforgeClient';
import { Logger } from './utils/logger';
import { validateConfiguration } from './utils/validation';

// Import command handlers
import { registerRefreshMocksCommand } from './commands/refreshMocks';
import { registerCreateMockCommand } from './commands/createMock';
import { registerEditMockCommand } from './commands/editMock';
import { registerDeleteMockCommand } from './commands/deleteMock';
import { registerToggleMockCommand } from './commands/toggleMock';
import { registerExportMocksCommand } from './commands/exportMocks';
import { registerImportMocksCommand } from './commands/importMocks';
import { registerViewStatsCommand } from './commands/viewStats';
import { registerStartServerCommand, registerStopServerCommand, registerRestartServerCommand } from './commands/serverControl';
import { registerShowLogsCommand } from './commands/showLogs';

/**
 * Extension activation function
 */
export function activate(context: vscode.ExtensionContext) {
    // Initialize logger
    Logger.initialize();
    Logger.info('MockForge extension is now active');

    // Get configuration
    const config = vscode.workspace.getConfiguration('mockforge');

    // Validate configuration
    const validationResult = validateConfiguration(config);
    if (!validationResult.valid) {
        Logger.warn('Configuration validation failed:', validationResult.error);
        vscode.window.showWarningMessage(`MockForge configuration issue: ${validationResult.error}`);
    }

    const serverUrl = config.get<string>('serverUrl', 'http://localhost:3000');

    // Initialize MockForge client
    let client = new MockForgeClient(serverUrl);

    // Create tree data providers
    let mocksProvider = new MocksTreeDataProvider(client);
    let serverProvider = new ServerControlProvider(client);

    // Register tree views
    vscode.window.registerTreeDataProvider('mockforge-explorer', mocksProvider);
    vscode.window.registerTreeDataProvider('mockforge-server', serverProvider);

    // Listen for configuration changes
    context.subscriptions.push(
        vscode.workspace.onDidChangeConfiguration(async (e: vscode.ConfigurationChangeEvent) => {
            if (e.affectsConfiguration('mockforge.serverUrl')) {
                // Server URL changed - reconnect with new URL
                const newServerUrl = config.get<string>('serverUrl', 'http://localhost:3000');

                // Disconnect old client
                client.disconnect();

                // Create new client with new URL
                client = new MockForgeClient(newServerUrl);

                // Recreate providers with new client
                mocksProvider = new MocksTreeDataProvider(client);
                serverProvider = new ServerControlProvider(client);

                // Re-register tree views
                vscode.window.registerTreeDataProvider('mockforge-explorer', mocksProvider);
                vscode.window.registerTreeDataProvider('mockforge-server', serverProvider);

                // Re-register all commands with new client and providers
                registerAllCommands(context, client, mocksProvider);

                // Auto-connect if enabled
                if (config.get<boolean>('autoConnect', true)) {
                    try {
                        await client.connect();
                        vscode.window.showInformationMessage(`Reconnected to MockForge server at ${newServerUrl}`);
                        mocksProvider.refresh();
                    } catch (error) {
                        vscode.window.showWarningMessage(`Could not connect to MockForge server at ${newServerUrl}: ${error instanceof Error ? error.message : 'Unknown error'}`);
                    }
                }
            }
        })
    );

    // Register all commands
    registerAllCommands(context, client, mocksProvider);

    // Auto-connect to server if enabled
    if (config.get<boolean>('autoConnect', true)) {
        client.connect().then(() => {
            Logger.info('Connected to MockForge server');
            vscode.window.showInformationMessage('Connected to MockForge server');
            mocksProvider.refresh();
        }).catch(error => {
            Logger.error('Failed to connect to MockForge server:', error);
            vscode.window.showWarningMessage(`Could not connect to MockForge server: ${error.message}`);
        });
    }
}

/**
 * Register all extension commands
 */
function registerAllCommands(
    context: vscode.ExtensionContext,
    client: MockForgeClient,
    mocksProvider: MocksTreeDataProvider
): void {
    registerRefreshMocksCommand(context, mocksProvider);
    registerCreateMockCommand(context, client, mocksProvider);
    registerEditMockCommand(context);
    registerDeleteMockCommand(context, client, mocksProvider);
    registerToggleMockCommand(context, client, mocksProvider);
    registerExportMocksCommand(context, client);
    registerImportMocksCommand(context, client, mocksProvider);
    registerViewStatsCommand(context, client);
    registerStartServerCommand(context, client, mocksProvider);
    registerStopServerCommand(context, client, mocksProvider);
    registerRestartServerCommand(context, client, mocksProvider);
    registerShowLogsCommand(context);
}

/**
 * Extension deactivation function
 */
export function deactivate() {
    Logger.dispose();
}
