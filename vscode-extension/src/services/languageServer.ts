//! Language server for MockForge configuration files
//!
//! Provides:
//! - Config file validation
//! - Inline preview of mock responses
//! - Autocomplete for config keys
//! - Hover documentation

import * as vscode from 'vscode';
import { ConfigValidator } from './configValidator';

/**
 * Language server for MockForge config files
 */
export class MockForgeLanguageServer {
    private configValidator: ConfigValidator;
    private diagnosticCollection: vscode.DiagnosticCollection;
    private disposables: vscode.Disposable[] = [];

    constructor() {
        this.configValidator = new ConfigValidator();
        this.diagnosticCollection = vscode.languages.createDiagnosticCollection('mockforge');
        this.disposables.push(this.diagnosticCollection);
    }

    /**
     * Activate the language server
     */
    activate(context: vscode.ExtensionContext): void {
        // Register document selector for MockForge config files
        const documentSelector: vscode.DocumentSelector = [
            { scheme: 'file', pattern: '**/mockforge.yaml' },
            { scheme: 'file', pattern: '**/mockforge.yml' },
            { scheme: 'file', pattern: '**/mockforge.toml' },
            { scheme: 'file', pattern: '**/mockforge.json' },
            { scheme: 'file', pattern: '**/*.mockforge.yaml' },
            { scheme: 'file', pattern: '**/*.mockforge.yml' },
        ];

        // Register validation on document change
        const validateDocument = async (document: vscode.TextDocument) => {
            if (this.isMockForgeConfigFile(document)) {
                const diagnostics = await this.configValidator.validateConfig(document);
                this.diagnosticCollection.set(document.uri, diagnostics);
            }
        };

        // Validate on open
        context.subscriptions.push(
            vscode.workspace.onDidOpenTextDocument(validateDocument)
        );

        // Validate on save
        context.subscriptions.push(
            vscode.workspace.onDidSaveTextDocument(validateDocument)
        );

        // Validate on change (debounced)
        let changeTimeout: NodeJS.Timeout | undefined;
        context.subscriptions.push(
            vscode.workspace.onDidChangeTextDocument((e) => {
                if (changeTimeout) {
                    clearTimeout(changeTimeout);
                }
                changeTimeout = setTimeout(() => {
                    validateDocument(e.document);
                }, 500); // Debounce 500ms
            })
        );

        // Validate all open documents
        vscode.workspace.textDocuments.forEach(validateDocument);

        // Register hover provider for inline documentation
        const hoverProvider = vscode.languages.registerHoverProvider(
            documentSelector,
            {
                provideHover: async (document, position) => {
                    return this.provideHover(document, position);
                },
            }
        );
        context.subscriptions.push(hoverProvider);

        // Register completion provider for autocomplete
        const completionProvider = vscode.languages.registerCompletionItemProvider(
            documentSelector,
            {
                provideCompletionItems: async (document, position) => {
                    return this.provideCompletionItems(document, position);
                },
            },
            '.', ':', '-' // Trigger characters
        );
        context.subscriptions.push(completionProvider);
    }

    /**
     * Check if document is a MockForge config file
     */
    private isMockForgeConfigFile(document: vscode.TextDocument): boolean {
        const fileName = document.fileName.toLowerCase();
        return fileName.includes('mockforge.yaml') ||
               fileName.includes('mockforge.yml') ||
               fileName.includes('mockforge.toml') ||
               fileName.includes('mockforge.json');
    }

    /**
     * Provide hover documentation
     */
    private async provideHover(
        document: vscode.TextDocument,
        position: vscode.Position
    ): Promise<vscode.Hover | null> {
        const wordRange = document.getWordRangeAtPosition(position);
        if (!wordRange) {
            return null;
        }

        const word = document.getText(wordRange);
        const line = document.lineAt(position.line);

        // Provide documentation for common config keys
        const documentation = this.getConfigKeyDocumentation(word, line.text);
        if (documentation) {
            return new vscode.Hover(documentation, wordRange);
        }

        // Check if this is an endpoint path - show mock response preview
        if (line.text.includes('path:') || line.text.includes('url:')) {
            const preview = await this.getMockResponsePreview(document, position);
            if (preview) {
                return new vscode.Hover(preview, wordRange);
            }
        }

        return null;
    }

    /**
     * Get documentation for a config key
     */
    private getConfigKeyDocumentation(key: string, line: string): vscode.MarkdownString | null {
        const docs: { [key: string]: string } = {
            'reality_level': 'Reality level controls how realistic mock responses are. Values: static, light, moderate, high, chaos',
            'reality': 'Reality configuration for unified realism control',
            'personas': 'Persona definitions for consistent, personality-driven data generation',
            'drift_budget': 'Contract drift budget configuration for monitoring API changes',
            'http': 'HTTP server configuration',
            'websocket': 'WebSocket server configuration',
            'grpc': 'gRPC server configuration',
            'admin': 'Admin UI configuration',
            'observability': 'Metrics, tracing, and observability configuration',
        };

        // Check if key is in the line
        if (line.includes(key)) {
            const doc = docs[key];
            if (doc) {
                return new vscode.MarkdownString(doc);
            }
        }

        return null;
    }

    /**
     * Get mock response preview for an endpoint
     */
    private async getMockResponsePreview(
        document: vscode.TextDocument,
        position: vscode.Position
    ): Promise<vscode.MarkdownString | null> {
        // This would query the MockForge server for the endpoint's mock response
        // For now, return a placeholder
        return new vscode.MarkdownString('**Mock Response Preview**\n\n*Connect to MockForge server to see preview*');
    }

    /**
     * Provide completion items for autocomplete
     */
    private async provideCompletionItems(
        document: vscode.TextDocument,
        position: vscode.Position
    ): Promise<vscode.CompletionItem[] | null> {
        const line = document.lineAt(position.line);
        const textBeforeCursor = line.text.substring(0, position.character);

        const completions: vscode.CompletionItem[] = [];

        // Provide completions for top-level config keys
        if (textBeforeCursor.trim().length === 0 || textBeforeCursor.endsWith('\n')) {
            completions.push(
                this.createCompletionItem('http', 'HTTP server configuration', vscode.CompletionItemKind.Module),
                this.createCompletionItem('websocket', 'WebSocket server configuration', vscode.CompletionItemKind.Module),
                this.createCompletionItem('grpc', 'gRPC server configuration', vscode.CompletionItemKind.Module),
                this.createCompletionItem('admin', 'Admin UI configuration', vscode.CompletionItemKind.Module),
                this.createCompletionItem('reality', 'Reality level configuration', vscode.CompletionItemKind.Property),
                this.createCompletionItem('personas', 'Persona definitions', vscode.CompletionItemKind.Property),
                this.createCompletionItem('drift_budget', 'Drift budget configuration', vscode.CompletionItemKind.Property),
                this.createCompletionItem('observability', 'Observability configuration', vscode.CompletionItemKind.Module),
            );
        }

        // Provide completions for reality_level enum
        if (textBeforeCursor.includes('reality_level:') || textBeforeCursor.includes('level:')) {
            completions.push(
                this.createCompletionItem('static', 'Static stubs - no simulation', vscode.CompletionItemKind.Value),
                this.createCompletionItem('light', 'Light simulation - minimal realism', vscode.CompletionItemKind.Value),
                this.createCompletionItem('moderate', 'Moderate realism - balanced', vscode.CompletionItemKind.Value),
                this.createCompletionItem('high', 'High realism - production-like', vscode.CompletionItemKind.Value),
                this.createCompletionItem('chaos', 'Production chaos - full realism', vscode.CompletionItemKind.Value),
            );
        }

        return completions.length > 0 ? completions : null;
    }

    /**
     * Create a completion item
     */
    private createCompletionItem(
        label: string,
        documentation: string,
        kind: vscode.CompletionItemKind
    ): vscode.CompletionItem {
        const item = new vscode.CompletionItem(label, kind);
        item.documentation = new vscode.MarkdownString(documentation);
        return item;
    }

    /**
     * Dispose resources
     */
    dispose(): void {
        this.disposables.forEach(d => d.dispose());
        this.diagnosticCollection.dispose();
    }
}
