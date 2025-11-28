import * as vscode from 'vscode';
import { ConnectionState } from '../types/mock';

/**
 * Status bar item for displaying MockForge connection status
 */
export class MockForgeStatusBar {
    private statusBarItem: vscode.StatusBarItem;
    private currentState: ConnectionState = 'disconnected';

    constructor() {
        // Create status bar item with priority (lower number = more left)
        this.statusBarItem = vscode.window.createStatusBarItem(
            vscode.StatusBarAlignment.Right,
            100
        );
        this.statusBarItem.command = 'mockforge.viewStats';
        this.statusBarItem.tooltip = 'MockForge Server Status - Click to view statistics';
        this.updateStatus('disconnected');
    }

    /**
     * Update the status bar with current connection state
     */
    updateStatus(state: ConnectionState): void {
        this.currentState = state;

        switch (state) {
            case 'connected':
                this.statusBarItem.text = '$(pulse) MockForge: Connected';
                this.statusBarItem.backgroundColor = undefined;
                this.statusBarItem.color = undefined;
                this.statusBarItem.tooltip = 'MockForge server is connected - Click to view statistics';
                break;
            case 'connecting':
                this.statusBarItem.text = '$(sync~spin) MockForge: Connecting...';
                this.statusBarItem.backgroundColor = undefined;
                this.statusBarItem.color = undefined;
                this.statusBarItem.tooltip = 'Connecting to MockForge server...';
                break;
            case 'reconnecting':
                this.statusBarItem.text = '$(sync~spin) MockForge: Reconnecting...';
                this.statusBarItem.backgroundColor = new vscode.ThemeColor('statusBarItem.warningBackground');
                this.statusBarItem.tooltip = 'Reconnecting to MockForge server...';
                break;
            case 'disconnected':
                this.statusBarItem.text = '$(circle-slash) MockForge: Disconnected';
                this.statusBarItem.backgroundColor = new vscode.ThemeColor('statusBarItem.errorBackground');
                this.statusBarItem.tooltip = 'MockForge server is disconnected - Click to view connection options';
                break;
        }

        this.statusBarItem.show();
    }

    /**
     * Get current connection state
     */
    getState(): ConnectionState {
        return this.currentState;
    }

    /**
     * Dispose of the status bar item
     */
    dispose(): void {
        this.statusBarItem.dispose();
    }
}
