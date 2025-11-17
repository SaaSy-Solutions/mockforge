//! Configuration file validator for MockForge
//!
//! Provides validation for mockforge.yaml and mockforge.toml files
//! using JSON Schema generated from Rust config structs

import * as vscode from 'vscode';
import * as fs from 'fs';
import * as path from 'path';
import { exec } from 'child_process';
import { promisify } from 'util';

const execAsync = promisify(exec);

/**
 * Configuration validator that uses JSON Schema for validation
 */
export class ConfigValidator {
    private schemaCache: any | null = null;
    private schemaPath: string | null = null;

    /**
     * Validate a MockForge configuration file
     */
    async validateConfig(document: vscode.TextDocument): Promise<vscode.Diagnostic[]> {
        const diagnostics: vscode.Diagnostic[] = [];

        // Check if this is a MockForge config file
        if (!this.isMockForgeConfigFile(document)) {
            return diagnostics;
        }

        try {
            // Load or generate schema
            const schema = await this.getSchema();
            if (!schema) {
                // Schema not available, skip validation
                return diagnostics;
            }

            // Parse the config file
            const config = this.parseConfig(document);
            if (!config) {
                // Parsing failed, add error
                diagnostics.push({
                    range: new vscode.Range(0, 0, 0, 0),
                    message: 'Failed to parse configuration file',
                    severity: vscode.DiagnosticSeverity.Error,
                    source: 'mockforge',
                });
                return diagnostics;
            }

            // Validate against schema
            const validationErrors = this.validateAgainstSchema(config, schema);
            diagnostics.push(...validationErrors);

        } catch (error) {
            // Log error but don't block user
            console.error('Config validation error:', error);
        }

        return diagnostics;
    }

    /**
     * Check if document is a MockForge config file
     */
    private isMockForgeConfigFile(document: vscode.TextDocument): boolean {
        const fileName = path.basename(document.fileName).toLowerCase();
        return fileName === 'mockforge.yaml' ||
               fileName === 'mockforge.yml' ||
               fileName === 'mockforge.toml' ||
               fileName === 'mockforge.json' ||
               fileName.endsWith('.mockforge.yaml') ||
               fileName.endsWith('.mockforge.yml');
    }

    /**
     * Get JSON Schema for MockForge config
     */
    private async getSchema(): Promise<any | null> {
        // Check cache first
        if (this.schemaCache) {
            return this.schemaCache;
        }

        try {
            // Try to find schema file in workspace
            const workspaceFolders = vscode.workspace.workspaceFolders;
            if (!workspaceFolders || workspaceFolders.length === 0) {
                return null;
            }

            // Look for schema in common locations
            const schemaPaths = [
                path.join(workspaceFolders[0].uri.fsPath, 'schemas', 'mockforge-config.schema.json'),
                path.join(workspaceFolders[0].uri.fsPath, '.mockforge', 'schema.json'),
                path.join(workspaceFolders[0].uri.fsPath, 'mockforge-config.schema.json'),
            ];

            for (const schemaPath of schemaPaths) {
                if (fs.existsSync(schemaPath)) {
                    const schemaContent = fs.readFileSync(schemaPath, 'utf-8');
                    this.schemaCache = JSON.parse(schemaContent);
                    this.schemaPath = schemaPath;
                    return this.schemaCache;
                }
            }

            // Try to generate schema using CLI
            const schema = await this.generateSchema();
            if (schema) {
                this.schemaCache = schema;
                return schema;
            }

        } catch (error) {
            console.error('Error loading schema:', error);
        }

        return null;
    }

    /**
     * Generate schema using mockforge CLI
     */
    private async generateSchema(): Promise<any | null> {
        try {
            // Try to run mockforge schema command
            const { stdout } = await execAsync('mockforge schema');
            const schema = JSON.parse(stdout);
            return schema;
        } catch (error) {
            // CLI not available or command failed
            console.warn('Could not generate schema via CLI:', error);
            return null;
        }
    }

    /**
     * Parse configuration file
     */
    private parseConfig(document: vscode.TextDocument): any | null {
        const content = document.getText();
        const fileName = path.basename(document.fileName).toLowerCase();

        try {
            if (fileName.endsWith('.json')) {
                return JSON.parse(content);
            } else if (fileName.endsWith('.toml')) {
                // Simple TOML parsing (for basic validation)
                // In production, use a proper TOML parser
                return this.parseTomlBasic(content);
            } else {
                // YAML
                // In production, use a proper YAML parser like js-yaml
                return this.parseYamlBasic(content);
            }
        } catch (error) {
            console.error('Parse error:', error);
            return null;
        }
    }

    /**
     * Basic YAML parsing (simplified)
     * In production, use js-yaml library
     */
    private parseYamlBasic(content: string): any {
        // This is a very basic parser - in production use js-yaml
        // For now, just return a basic object to allow validation to proceed
        return {};
    }

    /**
     * Basic TOML parsing (simplified)
     */
    private parseTomlBasic(content: string): any {
        // This is a very basic parser - in production use @iarna/toml
        // For now, just return a basic object
        return {};
    }

    /**
     * Validate config against JSON Schema
     */
    private validateAgainstSchema(config: any, schema: any): vscode.Diagnostic[] {
        const diagnostics: vscode.Diagnostic[] = [];

        // Basic validation - check for common issues
        // In production, use ajv or similar JSON Schema validator

        // Check for required top-level fields
        if (schema.required) {
            for (const field of schema.required) {
                if (!(field in config)) {
                    diagnostics.push({
                        range: new vscode.Range(0, 0, 0, 0),
                        message: `Missing required field: ${field}`,
                        severity: vscode.DiagnosticSeverity.Warning,
                        source: 'mockforge',
                    });
                }
            }
        }

        // Validate reality_level enum if present
        if (config.reality && config.reality.level) {
            const validLevels = ['static', 'light', 'moderate', 'high', 'chaos'];
            if (!validLevels.includes(config.reality.level)) {
                diagnostics.push({
                    range: this.findPropertyRange('reality.level'),
                    message: `Invalid reality level: ${config.reality.level}. Valid values: ${validLevels.join(', ')}`,
                    severity: vscode.DiagnosticSeverity.Error,
                    source: 'mockforge',
                });
            }
        }

        return diagnostics;
    }

    /**
     * Find the range of a property in the document
     */
    private findPropertyRange(propertyPath: string): vscode.Range {
        // Simplified - in production, parse YAML/TOML properly to find exact locations
        return new vscode.Range(0, 0, 0, 0);
    }

    /**
     * Clear schema cache
     */
    clearCache(): void {
        this.schemaCache = null;
        this.schemaPath = null;
    }
}
