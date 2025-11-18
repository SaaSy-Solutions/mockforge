//! Mock Preview Provider
//!
//! Provides inline preview of mock responses when hovering over endpoint references in code

import * as vscode from 'vscode';
import { MockForgeClient } from './mockforgeClient';
import { Logger } from '../utils/logger';
import { MockResponse } from '../types/mock';

/**
 * Mock preview provider for showing mock responses in hover tooltips
 */
export class MockPreviewProvider {
    private client: MockForgeClient | null = null;
    private enabled: boolean = true;

    constructor() {
        // Load configuration
        this.loadConfiguration();

        // Listen for configuration changes
        vscode.workspace.onDidChangeConfiguration((e: vscode.ConfigurationChangeEvent) => {
            if (e.affectsConfiguration('mockforge')) {
                this.loadConfiguration();
            }
        });
    }

    /**
     * Set the MockForge client instance
     */
    setClient(client: { getMockResponse: (method: string, path: string) => Promise<MockResponse | null> } | null): void {
        this.client = client as MockForgeClient | null;
    }

    /**
     * Load configuration
     */
    private loadConfiguration(): void {
        const config = vscode.workspace.getConfiguration('mockforge');
        this.enabled = config.get<boolean>('inlinePreview.enabled', true);
    }

    /**
     * Extract endpoint information from code
     */
    extractEndpoint(document: vscode.TextDocument, position: vscode.Position): { method: string; path: string } | null {
        const line = document.lineAt(position.line);
        const text = line.text;

        // Pattern 1: HTTP method followed by URL string
        // Examples: fetch('/api/users'), axios.get('/api/users'), http.get('/api/users')
        const httpMethodPattern = /(?:fetch|axios|http)\.?(get|post|put|patch|delete|options|head)\s*\(['"`]([^'"`]+)['"`]/i;
        const methodMatch = text.match(httpMethodPattern);
        if (methodMatch) {
            return {
                method: methodMatch[1].toUpperCase(),
                path: methodMatch[2]
            };
        }

        // Pattern 2: URL string with method in comment or nearby
        // Example: '/api/users' with 'GET' on previous line
        const urlPattern = /['"`]([/][^'"`]+)['"`]/;
        const urlMatch = text.match(urlPattern);
        if (urlMatch) {
            // Check previous line for HTTP method
            if (position.line > 0) {
                const prevLine = document.lineAt(position.line - 1).text;
                const methodMatch = prevLine.match(/\b(GET|POST|PUT|PATCH|DELETE|OPTIONS|HEAD)\b/i);
                if (methodMatch) {
                    return {
                        method: methodMatch[1].toUpperCase(),
                        path: urlMatch[1]
                    };
                }
            }
            // Default to GET if no method found
            return {
                method: 'GET',
                path: urlMatch[1]
            };
        }

        // Pattern 3: REST client patterns (axios, fetch without method)
        const fetchPattern = /fetch\s*\(['"`]([^'"`]+)['"`]/i;
        const fetchMatch = text.match(fetchPattern);
        if (fetchMatch) {
            return {
                method: 'GET', // fetch defaults to GET
                path: fetchMatch[1]
            };
        }

        // Pattern 4: MockForge config file - path: '/api/users'
        if (document.fileName.includes('mockforge') || document.fileName.includes('scenario')) {
            const pathMatch = text.match(/path:\s*['"`]?([^'"`\s]+)['"`]?/i);
            const methodMatch = text.match(/method:\s*['"`]?([^'"`\s]+)['"`]?/i);
            if (pathMatch) {
                return {
                    method: methodMatch ? methodMatch[1].toUpperCase() : 'GET',
                    path: pathMatch[1]
                };
            }
        }

        return null;
    }

    /**
     * Get mock response preview for an endpoint
     */
    async getPreview(endpoint: { method: string; path: string }): Promise<vscode.MarkdownString | null> {
        if (!this.enabled) {
            return null;
        }

        if (!this.client) {
            return new vscode.MarkdownString('**Mock Response Preview**\n\n*MockForge server not connected*');
        }

        try {
            // Query MockForge server for mock response
            // This would use the MockForge API to get the mock response
            // For now, we'll construct a preview based on the endpoint

            // Try to get mock from server
            const response = await this.client.getMockResponse(endpoint.method, endpoint.path);

            if (response) {
                const preview = this.formatResponsePreview(response);
                return preview;
            } else {
                return new vscode.MarkdownString(
                    `**Mock Response Preview**\n\n` +
                    `**${endpoint.method}** \`${endpoint.path}\`\n\n` +
                    `*No mock configured for this endpoint*`
                );
            }
        } catch (error) {
            Logger.error('Failed to get mock preview:', error);
            return new vscode.MarkdownString(
                `**Mock Response Preview**\n\n` +
                `*Error: ${error instanceof Error ? error.message : 'Unknown error'}*`
            );
        }
    }

    /**
     * Format response preview as markdown
     */
    private formatResponsePreview(response: MockResponse): vscode.MarkdownString {
        const md = new vscode.MarkdownString();
        md.isTrusted = true;

        md.appendMarkdown(`**Mock Response Preview**\n\n`);

        if (response.headers) {
            md.appendMarkdown(`**Headers:**\n`);
            md.appendCodeblock(JSON.stringify(response.headers, null, 2), 'json');
            md.appendMarkdown(`\n`);
        }

        if (response.body !== undefined) {
            md.appendMarkdown(`**Body:**\n`);
            const bodyStr = typeof response.body === 'string'
                ? response.body
                : JSON.stringify(response.body, null, 2);
            md.appendCodeblock(bodyStr, 'json');
        }

        return md;
    }
}
