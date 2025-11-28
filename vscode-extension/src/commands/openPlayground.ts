/**
 * Command to open MockForge playground with a specific endpoint
 */

import * as vscode from 'vscode';
import { Logger } from '../utils/logger';

/**
 * Register the open playground command
 */
export function registerOpenPlaygroundCommand(context: vscode.ExtensionContext): void {
    const command = vscode.commands.registerCommand(
        'mockforge.openPlayground',
        async (endpoint?: { method: string; path: string }) => {
            try {
                // Get server URL from configuration
                const config = vscode.workspace.getConfiguration('mockforge');
                const serverUrl = config.get<string>('serverUrl', 'http://localhost:3000');

                // Determine playground URL
                // Try embedded admin first, then standalone
                let playgroundUrl: string;

                // Check if admin is embedded (common pattern)
                // We'll try the embedded path first, then standalone
                const embeddedUrl = `${serverUrl}/admin/#/playground`;
                const standaloneUrl = serverUrl.replace(':3000', ':9080') + '/#/playground';

                // Try to detect which one is available by checking the server URL
                // For now, we'll use embedded as default since it's more common
                playgroundUrl = embeddedUrl;

                // Build URL with endpoint parameters if provided
                if (endpoint) {
                    const params = new URLSearchParams({
                        method: endpoint.method,
                        path: endpoint.path,
                    });
                    playgroundUrl += `?${params.toString()}`;
                }

                // Open in external browser
                await vscode.env.openExternal(vscode.Uri.parse(playgroundUrl));

                Logger.info(`Opened playground: ${playgroundUrl}`);
            } catch (error) {
                const errorMessage = error instanceof Error ? error.message : 'Unknown error';
                Logger.error('Failed to open playground:', error);
                vscode.window.showErrorMessage(`Failed to open playground: ${errorMessage}`);
            }
        }
    );

    context.subscriptions.push(command);
}
