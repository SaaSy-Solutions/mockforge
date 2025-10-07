import * as vscode from 'vscode';
import { MocksTreeDataProvider } from './mocksTreeProvider';
import { ServerControlProvider } from './serverControlProvider';
import { MockForgeClient } from './mockforgeClient';

export function activate(context: vscode.ExtensionContext) {
    console.log('MockForge extension is now active');

    // Get configuration
    const config = vscode.workspace.getConfiguration('mockforge');
    const serverUrl = config.get<string>('serverUrl', 'http://localhost:3000');

    // Initialize MockForge client
    const client = new MockForgeClient(serverUrl);

    // Create tree data providers
    const mocksProvider = new MocksTreeDataProvider(client);
    const serverProvider = new ServerControlProvider(client);

    // Register tree views
    vscode.window.registerTreeDataProvider('mockforge-explorer', mocksProvider);
    vscode.window.registerTreeDataProvider('mockforge-server', serverProvider);

    // Register commands
    context.subscriptions.push(
        vscode.commands.registerCommand('mockforge.refreshMocks', () => {
            mocksProvider.refresh();
        })
    );

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
                    id: '',
                    name,
                    method,
                    path,
                    response: { body: responseBody },
                    enabled: true
                });

                vscode.window.showInformationMessage(`Mock "${name}" created successfully`);
                mocksProvider.refresh();
            } catch (error) {
                vscode.window.showErrorMessage(`Failed to create mock: ${error}`);
            }
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('mockforge.editMock', async (mock) => {
            // Open a new editor with mock configuration
            const doc = await vscode.workspace.openTextDocument({
                content: JSON.stringify(mock, null, 2),
                language: 'json'
            });

            await vscode.window.showTextDocument(doc);
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('mockforge.deleteMock', async (mock) => {
            const confirm = await vscode.window.showWarningMessage(
                `Delete mock "${mock.name}"?`,
                'Yes',
                'No'
            );

            if (confirm === 'Yes') {
                try {
                    await client.deleteMock(mock.id);
                    vscode.window.showInformationMessage(`Mock "${mock.name}" deleted`);
                    mocksProvider.refresh();
                } catch (error) {
                    vscode.window.showErrorMessage(`Failed to delete mock: ${error}`);
                }
            }
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('mockforge.toggleMock', async (mock) => {
            try {
                await client.updateMock(mock.id, {
                    ...mock,
                    enabled: !mock.enabled
                });
                vscode.window.showInformationMessage(
                    `Mock "${mock.name}" ${mock.enabled ? 'disabled' : 'enabled'}`
                );
                mocksProvider.refresh();
            } catch (error) {
                vscode.window.showErrorMessage(`Failed to toggle mock: ${error}`);
            }
        })
    );

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
                    vscode.window.showInformationMessage('Mocks exported successfully');
                }
            } catch (error) {
                vscode.window.showErrorMessage(`Failed to export mocks: ${error}`);
            }
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('mockforge.importMocks', async () => {
            const uri = await vscode.window.showOpenDialog({
                canSelectMany: false,
                filters: {
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
                vscode.window.showInformationMessage('Mocks imported successfully');
                mocksProvider.refresh();
            } catch (error) {
                vscode.window.showErrorMessage(`Failed to import mocks: ${error}`);
            }
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('mockforge.viewStats', async () => {
            try {
                const stats = await client.getStats();
                const panel = vscode.window.createWebviewPanel(
                    'mockforgeStats',
                    'MockForge Statistics',
                    vscode.ViewColumn.One,
                    {}
                );

                panel.webview.html = getStatsHtml(stats);
            } catch (error) {
                vscode.window.showErrorMessage(`Failed to get stats: ${error}`);
            }
        })
    );

    // Auto-connect to server if enabled
    if (config.get<boolean>('autoConnect', true)) {
        client.connect().then(() => {
            vscode.window.showInformationMessage('Connected to MockForge server');
            mocksProvider.refresh();
        }).catch(error => {
            vscode.window.showWarningMessage(`Could not connect to MockForge server: ${error.message}`);
        });
    }
}

function getStatsHtml(stats: any): string {
    return `
        <!DOCTYPE html>
        <html>
        <head>
            <style>
                body {
                    font-family: var(--vscode-font-family);
                    color: var(--vscode-foreground);
                    padding: 20px;
                }
                .stat-row {
                    display: flex;
                    justify-content: space-between;
                    padding: 10px;
                    border-bottom: 1px solid var(--vscode-panel-border);
                }
                .stat-label {
                    font-weight: bold;
                }
            </style>
        </head>
        <body>
            <h1>MockForge Server Statistics</h1>
            <div class="stat-row">
                <span class="stat-label">Uptime:</span>
                <span>${stats.uptime_seconds}s</span>
            </div>
            <div class="stat-row">
                <span class="stat-label">Total Requests:</span>
                <span>${stats.total_requests}</span>
            </div>
            <div class="stat-row">
                <span class="stat-label">Active Mocks:</span>
                <span>${stats.active_mocks}</span>
            </div>
            <div class="stat-row">
                <span class="stat-label">Enabled Mocks:</span>
                <span>${stats.enabled_mocks}</span>
            </div>
            <div class="stat-row">
                <span class="stat-label">Registered Routes:</span>
                <span>${stats.registered_routes}</span>
            </div>
        </body>
        </html>
    `;
}

export function deactivate() {}
