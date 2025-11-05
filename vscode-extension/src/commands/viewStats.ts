import * as vscode from 'vscode';
import { MockForgeClient } from '../services/mockforgeClient';
import { ServerStats } from '../types/mock';
import { Logger } from '../utils/logger';

/**
 * Generate HTML for statistics display
 */
function getStatsHtml(stats: ServerStats): string {
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

/**
 * Command handler for viewing server statistics
 */
export function registerViewStatsCommand(
    context: vscode.ExtensionContext,
    client: MockForgeClient
): void {
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
                Logger.error('Failed to get stats:', error);
                vscode.window.showErrorMessage(`Failed to get stats: ${error}`);
            }
        })
    );
}

